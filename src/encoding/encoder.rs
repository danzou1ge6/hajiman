use super::Encoding;
use crate::bits_key::{Bits, BitsMap};
use crate::letters::Code;

pub struct Encoder<B> {
    char_encodings: BitsMap<B, Code>,
}

impl<B> Encoder<B>
where
    B: Bits,
{
    pub fn from_encoding(encoding: &Encoding<B>) -> Self {
        Self {
            char_encodings: encoding.char2code.clone(),
        }
    }

    pub fn encode(&self, char: B) -> &Code {
        &self.char_encodings[char]
    }
}

#[cfg(test)]
mod test {
    use super::super::Encoding;
    use crate::bits::Bits8;
    use crate::characters::{CharacterFrequency, test::example_characters};
    use crate::letters::test::example_letters;

    #[test]
    fn test_build_encoding() {
        let chars = example_characters();
        let letters = example_letters();

        let encoding = Encoding::build(letters, &chars);
        let encoder = encoding.encoder();

        let _ = encoder.encode(Bits8::from(1));
    }

    #[test]
    fn test_build_encoding_for_all_equal_frequency() {
        let chars = CharacterFrequency::all_equal();
        let letters = example_letters();

        let encoding = Encoding::build(letters, &chars);
        let encoder = encoding.encoder();

        let _ = encoder.encode(Bits8::from(1));
    }
}
