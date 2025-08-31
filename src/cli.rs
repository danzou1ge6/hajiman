use std::{
    io::{BufRead, BufReader, BufWriter, Cursor, Read, Seek, Write, stdin, stdout},
    path::PathBuf,
};

use clap::{Parser, Subcommand};

use crate::{
    CharacterCounter, CharacterFrequency, JimiDecoder, JimiEncoder, JimiEncoding, JimiError,
    bits::Bits8,
    bits_key::{Bits, ConcatError},
    hajimi_tokens, lexing,
};

#[derive(Parser)]
#[command(
    version,
    author,
    about = "Command line utility to cipher and decipher honey water codec"
)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,

    #[arg(short, long)]
    /// Input from file instead of standard input or command line argument
    input_file: Option<String>,

    #[arg(short, long)]
    /// Output to file instead of standard output
    output_file: Option<String>,

    #[arg(short, long, default_value = "false")]
    /// If set to true, encoding will be created based on frequency of bytes;
    frequency_based: bool,

    #[arg(short, long)]
    /// Read encoding from file.
    ///
    /// This argument has higher precedence then `--frequency-based`
    encoding_file: Option<PathBuf>,

    #[arg(short, long, default_value = "false")]
    /// Whether to output encoding in pretty JSON
    pretty_encoding: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Encode honey water.
    ///
    /// If `--frequency-based` is set to false and no `--encoding-file` is provided,
    /// encoding is created assuming all bytes appear with uniform probability.
    ///
    /// Before outputing encoded data, the encoding it self is outputed with JSON format.
    Encode {
        /// Input from command line argument intead of standard input
        data: Option<String>,
    },
    /// Decode honey water.
    ///
    /// If `--encoding-file` is not provided,
    /// we will first look for enclosed JSON format of encoding from begining of the input;
    ///
    /// If that is not found, we fall back to uniform probability encoding.
    Decode {
        /// Input from command line argument intead of standard input
        data: Option<String>,
    },
}
use Command::*;

impl Cli {
    fn data(&self) -> &Option<String> {
        match &self.command {
            Encode { data } => data,
            Decode { data } => data,
        }
    }
}

type Enc = JimiEncoding<Bits8>;

fn read_encoding<'a>(
    mut input: impl Read,
) -> Option<Result<JimiEncoding<Bits8>, serde_json::Error>> {
    let mut deserializer = serde_json::Deserializer::from_reader(&mut input).into_iter::<Enc>();
    deserializer.next()
}

fn count_character(
    reader: &mut dyn ReadSeek,
    counter: &mut CharacterCounter<Bits8>,
) -> std::io::Result<()> {
    let mut buf = vec![0; 512];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => return Ok(()),
            Ok(n) => {
                counter.count(Bits8::iter_bytes(&buf[..n]).data);
            }
            Err(e) => return Err(e),
        }
    }
}

fn encode(
    reader: &mut dyn ReadSeek,
    encoder: &JimiEncoder<Bits8>,
    mut writer: impl Write,
) -> Result<(), String> {
    let mut buf = vec![0; 512];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => return Ok(()),
            Ok(n) => {
                let encoded = encoder.encode(&buf[..n]);
                for s in encoded.data {
                    writer
                        .write(s.as_bytes())
                        .map_err(|e| format!("write output failed: {}", e))?;
                }
            }
            Err(e) => return Err(format!("read input failed: {}", e)),
        }
    }
}

fn encode_enclosing_encoding(
    reader: &mut dyn ReadSeek,
    encoding: &JimiEncoding<Bits8>,
    encoder: &JimiEncoder<Bits8>,
    mut writer: impl Write,
    pretty_encoding: bool,
) -> Result<(), String> {
    if pretty_encoding {
        serde_json::to_writer_pretty(&mut writer, &encoding)
            .map_err(|e| format!("write encoding to output failed: {}", e))?;
    } else {
        serde_json::to_writer(&mut writer, &encoding)
            .map_err(|e| format!("write encoding to output failed: {}", e))?;
    }
    write!(&mut writer, "\n").map_err(|e| format!("write newline to output failed: {}", e))?;
    encode(reader, &encoder, writer)?;

    Ok(())
}

fn skip_until_newline(reader: &mut dyn ReadSeek) -> Result<(), String> {
    let _ = reader
        .skip_until(b'\n')
        .map_err(|e| format!("skip until newline failed: {e}"))?;
    Ok(())
}

fn decode(
    reader: &mut dyn ReadSeek,
    decoder: &JimiDecoder<Bits8>,
    mut writer: impl Write,
) -> Result<(), String> {
    let mut iter = decoder.decode_chars("".to_string().into_chars());

    let mut offset = 0;
    let mut buf = vec![0; 512];
    let mut termination_error = None;

    loop {
        match reader.read(&mut buf[offset..]) {
            Ok(0) => {
                if offset != 0 {
                    return Err(format!("input is not complete UTF-8 string"));
                } else if let Some(te) = termination_error {
                    return Err(format!("error parsing honey water: {:?}", te));
                } else {
                    return Ok(());
                }
            }
            Ok(n) => {
                let s = match str::from_utf8(&buf[0..offset + n]) {
                    Ok(b) => {
                        offset = 0;
                        b.to_string()
                    }
                    Err(e) => {
                        let idx = e.valid_up_to();

                        if idx == 0 {
                            return Err("input is not valid UTF-8".to_string());
                        }
                        let r = str::from_utf8(&buf[0..idx]).unwrap().to_string();
                        let leftover = buf[idx..offset + n].to_owned();
                        offset = offset + n - idx;
                        buf[0..offset].copy_from_slice(&leftover);
                        r
                    }
                };

                iter = iter.cont(|inner| inner.cont(|_| s.into_chars()));

                let iter = &mut iter;
                let mut bytes = Vec::new();
                let result = Bits8::concat(
                    iter.map(|x| x.map_err(|e| decoder.map_error(e))),
                    &mut bytes,
                );

                match &result {
                    Ok(()) => {
                        termination_error = None;
                    }
                    Err(ConcatError::Io(..)) => {
                        panic!("concating to vector should not produce any error")
                    }
                    Err(ConcatError::Parent(JimiError::Lexing(
                        lexing::Error::UnexpectedTermination(..),
                    )))
                    | Err(ConcatError::Parent(JimiError::Hajiman(
                        lexing::Error::UnexpectedTermination(..),
                    ))) => {
                        termination_error = Some(result.unwrap_err());
                    }
                    Err(ConcatError::Parent(e)) => {
                        return Err(format!("error parsing honey water: {:?}", e));
                    }
                };

                writer
                    .write(&bytes)
                    .map_err(|e| format!("error writing output: {}", e))?;
            }
            Err(e) => return Err(format!("read input failed: {}", e)),
        }
    }
}

trait ReadSeek: BufRead + Seek {}

impl<T> ReadSeek for BufReader<T> where T: Seek + Read {}
impl<T> ReadSeek for Cursor<T> where T: AsRef<[u8]> {}

pub fn run(cli: Cli) -> Result<(), String> {
    let mut input: Box<dyn ReadSeek> = if let Some(input_fpath) = &cli.input_file {
        let f = std::fs::File::open(input_fpath)
            .map_err(|e| format!("open file {:?} failed: {}", input_fpath, e))?;
        Box::new(BufReader::new(f))
    } else if let Some(data) = cli.data() {
        Box::new(Cursor::new(data))
    } else {
        let mut s = String::new();
        stdin()
            .read_to_string(&mut s)
            .map_err(|e| format!("read STDIN failed: {}", e))?;
        Box::new(Cursor::new(s))
    };

    let output: Box<dyn Write> = if let Some(output_fpath) = &cli.output_file {
        let f = std::fs::File::create(&output_fpath)
            .map_err(|e| format!("create file {:?} failed: {}", output_fpath, e))?;
        Box::new(BufWriter::new(f))
    } else {
        Box::new(stdout())
    };

    let encoding = if let Some(encoding_file) = &cli.encoding_file {
        let f = std::fs::File::open(encoding_file)
            .map_err(|e| format!("open file {:?} failed: {}", encoding_file, e))?;
        match read_encoding(BufReader::new(f)) {
            Some(Ok(e)) => e,
            Some(Err(e)) => {
                return Err(format!(
                    "error parsing encoding file {:?}: {}",
                    encoding_file, e
                ));
            }
            None => {
                return Err(format!(
                    "encoding file {:?} does not contain any JSON data",
                    encoding_file
                ));
            }
        }
    } else {
        match cli.command {
            Encode { .. } if cli.frequency_based => {
                let mut counter = CharacterCounter::empty();
                count_character(input.as_mut(), &mut counter)
                    .map_err(|e| format!("read input failed: {}", e))?;
                let freq = counter.finish();

                input
                    .seek(std::io::SeekFrom::Start(0))
                    .map_err(|e| format!("seek input to begin failed: {}", e))?;

                JimiEncoding::new(hajimi_tokens(), &freq)
            }
            Encode { .. } => {
                let freq = CharacterFrequency::all_equal();
                JimiEncoding::new(hajimi_tokens(), &freq)
            }
            Decode { .. } => {
                if let Some(Ok(enc)) = read_encoding(input.as_mut()) {
                    skip_until_newline(input.as_mut())?;
                    enc
                } else {
                    let freq = CharacterFrequency::all_equal();
                    JimiEncoding::new(hajimi_tokens(), &freq)
                }
            }
        }
    };

    match cli.command {
        Encode { .. } => {
            let encoder = encoding.encoder();
            encode_enclosing_encoding(
                input.as_mut(),
                &encoding,
                &encoder,
                output,
                cli.pretty_encoding,
            )?;
        }
        Decode { .. } => {
            let decoder = encoding
                .decoder()
                .expect("honey water is of course prefix-free");
            decode(input.as_mut(), &decoder, output)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    fn test_inputs() -> Vec<u8> {
        (0u8..200)
            .chain((40..100).rev())
            .chain(2..100)
            .chain((90..200).rev())
            .collect()
    }

    #[test]
    fn test_encode_decode() {
        let freq = CharacterFrequency::<Bits8>::all_equal();
        let encoding = JimiEncoding::new(hajimi_tokens(), &freq);

        let inputs = test_inputs();
        let encoded = {
            let mut reader = Cursor::new(&inputs);
            let mut s = Vec::new();

            encode(&mut reader, &encoding.encoder(), &mut s).unwrap();

            String::from_utf8(s).unwrap()
        };

        println!("{encoded}");

        let decoded = {
            let mut reader = Cursor::new(encoded.as_bytes());
            let mut s = Vec::new();

            decode(&mut reader, &encoding.decoder().unwrap(), &mut s).unwrap();
            s
        };

        assert_eq!(inputs.as_slice(), decoded.as_slice());
    }

    #[test]
    fn test_serialize_encoding() {
        let freq = CharacterFrequency::<Bits8>::all_equal();
        let encoding = JimiEncoding::new(hajimi_tokens(), &freq);

        let json = serde_json::to_string_pretty(&encoding).unwrap() + "some trailing data";
        let parsed = read_encoding(Cursor::new(&json)).unwrap().unwrap();

        assert_eq!(parsed, encoding);
    }

    #[test]
    fn test_encode_with_enclosed_encoding_and_decode() {
        let freq = CharacterFrequency::<Bits8>::all_equal();
        let encoding = JimiEncoding::new(hajimi_tokens(), &freq);
        let (encoder, decoder) = (encoding.encoder(), encoding.decoder().unwrap());

        let inputs = test_inputs();
        let mut encoded = Vec::new();

        encode_enclosing_encoding(
            &mut Cursor::new(&inputs),
            &encoding,
            &encoder,
            &mut encoded,
            false,
        )
        .unwrap();

        let mut encoded_cursor = Cursor::new(&encoded);
        let encoding_read = read_encoding(&mut encoded_cursor).unwrap().unwrap();
        skip_until_newline(&mut encoded_cursor).unwrap();

        assert_eq!(&encoding_read, &encoding);

        let mut decoded = Vec::new();
        decode(&mut encoded_cursor, &decoder, &mut decoded).unwrap();

        assert_eq!(&decoded, &inputs);
    }
}
