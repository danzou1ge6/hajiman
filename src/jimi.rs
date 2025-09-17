use crate::bits_key::{Bits, BitsMap, ConcatError, Padded};
use crate::characters::CharacterFrequency;
use crate::encoding::{Decoder, Encoding};
use crate::letters::{LetterCosts, LetterIdIndexed};
use crate::lexing::{self, LexemError, Lexer, StringLexer};

#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct JimiEncoding<B>
where
    B: Bits,
{
    encoding: Encoding<B>,
    tokens: LetterIdIndexed<String>,
}

impl<B> std::fmt::Debug for JimiEncoding<B>
where
    B: Bits,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (b, code) in self.encoding.char2code().iter() {
            writeln!(
                f,
                "{:?}: {:?} {}",
                b,
                code,
                code.iter().map(|i| &self.tokens[i][..]).collect::<String>()
            )?;
        }
        Ok(())
    }
}

impl<B> JimiEncoding<B>
where
    B: Bits,
{
    pub fn encoder(&self) -> JimiEncoder<B> {
        JimiEncoder::from_encoding(self)
    }

    pub fn decoder(&self) -> Result<JimiDecoder<B>, LexemError> {
        JimiDecoder::from_encoding(self)
    }

    pub fn new(tokens: LetterIdIndexed<String>, freq: &CharacterFrequency<B>) -> Self {
        let letters =
            LetterCosts::build(tokens.map_by_ref(|_, s| s.len().try_into().unwrap())).unwrap();
        Self {
            encoding: Encoding::build(letters, &freq),
            tokens,
        }
    }
}

mod encoder {
    use super::*;

    #[derive(Debug, Clone)]
    pub struct JimiEncoder<B> {
        chunk: String,
        char2code: BitsMap<B, (usize, usize)>,
    }

    impl<B> JimiEncoder<B>
    where
        B: Bits,
    {
        pub fn from_encoding(encoding: &JimiEncoding<B>) -> Self {
            let mut chunk = String::new();

            let char2code = encoding.encoding.char2code().map(|_, code| {
                let offset = chunk.len();
                for &letter_id in code.iter() {
                    chunk.push_str(&encoding.tokens[letter_id]);
                }
                let len = chunk.len() - offset;
                (offset, len)
            });

            Self { chunk, char2code }
        }

        pub fn encode_bits(&self, bits: B) -> &str {
            let (offset, len) = self.char2code[bits];
            unsafe {
                std::str::from_utf8_unchecked(std::slice::from_raw_parts(
                    self.chunk.as_ptr().add(offset),
                    len,
                ))
            }
        }

        pub fn encode_bits_iter(
            &self,
            bits: impl Iterator<Item = B>,
        ) -> impl Iterator<Item = &str> {
            bits.map(|b| self.encode_bits(b))
        }

        pub fn encode(&self, bytes: &[u8]) -> Padded<impl Iterator<Item = &str>> {
            let Padded {
                data,
                original_length,
            } = B::iter_bytes(bytes);
            Padded {
                data: data.map(|b| self.encode_bits(b)),
                original_length,
            }
        }
    }
}

pub use encoder::JimiEncoder;

mod decoder {
    use crate::{encoding, letters::LetterId};

    use super::*;

    #[derive(Debug, Clone)]
    pub enum Error {
        Lexing(lexing::iter::Error<char>),
        Hajiman(lexing::iter::Error<String>),
    }

    #[derive(Debug, Clone)]
    pub struct JimiDecoder<B> {
        lexer: StringLexer,
        decoder: Decoder<B>,
        tokens: LetterIdIndexed<String>,
    }

    impl<B> JimiDecoder<B>
    where
        B: Bits,
    {
        pub fn from_encoding(encoding: &JimiEncoding<B>) -> Result<Self, LexemError> {
            let lexer = StringLexer::new(&encoding.tokens)?;
            Ok(Self {
                lexer,
                decoder: encoding.encoding.decoder(),
                tokens: encoding.tokens.clone(),
            })
        }

        pub fn map_error(&self, e: lexing::Error<LetterId, lexing::iter::Error<char>>) -> Error {
            e.map(|letter_id| self.tokens[letter_id].clone())
                .flatten(|le| Error::Lexing(le), |he| Error::Hajiman(he))
        }

        pub fn decode_chars<It: Iterator<Item = char>>(
            &self,
            chars: It,
        ) -> encoding::decoder::IterFromError<
            '_,
            B,
            lexing::string_lexer::Iter<'_, It>,
            lexing::iter::Error<char>,
        > {
            self.decoder.decode_from_error(self.lexer.lex(chars))
        }

        pub fn decode_to_bits<'a, S: AsRef<str> + ?Sized + 'a>(
            &'a self,
            s: &'a S,
        ) -> impl Iterator<Item = Result<B, Error>> + 'a {
            self.decode_chars(s.as_ref().chars())
                .map(|x| x.map_err(|e| self.map_error(e)))
        }

        pub fn decode<'a, S: AsRef<str> + ?Sized + 'a>(
            &'a self,
            s: &'a S,
            writer: impl std::io::Write,
        ) -> Result<(), ConcatError<Error>> {
            B::concat(self.decode_to_bits(s), writer)
        }

        pub fn decode_to_vec<'a, S: AsRef<str> + ?Sized + 'a>(
            &'a self,
            s: &'a S,
        ) -> Result<Vec<u8>, ConcatError<Error>> {
            let mut v = Vec::new();
            self.decode(s, &mut v)?;
            Ok(v)
        }

        pub fn lexer(&self) -> &StringLexer {
            &self.lexer
        }
    }
}

pub use decoder::Error as JimiError;
pub use decoder::JimiDecoder;

#[cfg(test)]
mod test {
    use super::*;
    use crate::bits_key::{
        Bits,
        bits::{Bits4, Bits6, Bits8},
    };
    use crate::characters::CharacterFrequency;
    use crate::hajimi::hajimi_tokens;

    fn test_honey_water<B: Bits>() {
        let encoding = JimiEncoding::<B>::new(hajimi_tokens(), &CharacterFrequency::all_equal());
        let (encoder, decoder) = (encoding.encoder(), encoding.decoder().unwrap());
        let src: Vec<u8> = (0..255).chain((10..200).rev()).chain(100..190).collect();

        let encoded: Padded<String> = encoder.encode(&src).map(|x| x.collect());

        let mut decoded = Vec::new();
        decoder.decode(&encoded.data, &mut decoded).unwrap();

        assert_eq!(src, decoded[..encoded.original_length]);
    }

    #[test]
    fn test_honey_water_8bit() {
        test_honey_water::<Bits8>();
    }

    #[test]
    fn test_honey_water_4bit() {
        test_honey_water::<Bits4>();
    }

    #[test]
    fn test_honey_water_6bit() {
        test_honey_water::<Bits6>();
    }
}
