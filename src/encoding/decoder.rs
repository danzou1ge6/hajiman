use crate::lexing;
use std::ops::Deref;

use crate::bits_key::Bits;
use crate::letters::{LetterId, LetterIdIndexed};

use super::Encoding;

#[derive(Debug, Clone)]
pub struct Tree<B>(lexing::Tree<LetterId, B, LetterIdIndexed<Tree<B>>>);

impl<B> Deref for Tree<B> {
    type Target = lexing::Tree<LetterId, B, LetterIdIndexed<Tree<B>>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<B> From<lexing::Tree<LetterId, B, LetterIdIndexed<Tree<B>>>> for Tree<B> {
    fn from(value: lexing::Tree<LetterId, B, LetterIdIndexed<Tree<B>>>) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone)]
pub struct Decoder<B> {
    roots: LetterIdIndexed<Tree<B>>,
}

pub type Iter<'t, B, It> = lexing::iter::LexingIter<'t, LetterId, B, LetterIdIndexed<Tree<B>>, It>;
pub type IterFromError<'t, B, It, E> =
    lexing::iter_from_error::LexingIter<'t, LetterId, B, LetterIdIndexed<Tree<B>>, It, E>;

impl<B> Decoder<B>
where
    B: Bits,
{
    pub fn from_encoding(encoding: &Encoding<B>) -> Self {
        let roots = lexing::build_tree::<LetterIdIndexed<_>, _, _, _, _>(
            encoding
                .char2code
                .iter()
                .map(|(char, code)| (char, code.clone())),
            encoding.n_letters.before(),
        )
        .expect("Extended-Hoffman encoding should be prefix free");

        Self { roots }
    }

    pub fn decode_from_error<E, It: Iterator<Item = Result<LetterId, E>>>(
        &self,
        letters: It,
    ) -> IterFromError<'_, B, It, E> {
        lexing::iter_from_error::LexingIter::new(&self.roots, letters)
    }

    pub fn decode<It: Iterator<Item = LetterId>>(&self, letters: It) -> Iter<'_, B, It> {
        lexing::iter::LexingIter::new(&self.roots, letters)
    }
}

#[cfg(test)]
mod test {
    use super::super::Encoding;
    use crate::bits::Bits8;
    use crate::characters::{CharacterFrequency, test::example_characters};
    use crate::letters::test::example_letters;

    #[test]
    fn test_build_decoding() {
        let chars = example_characters();
        let letters = example_letters();

        let encoding = Encoding::build(letters, &chars);
        let encoder = encoding.encoder();
        let decoder = encoding.decoder();

        let plain = vec![0, 1, 2, 0];

        let code: Vec<_> = plain
            .iter()
            .map(|&x| encoder.encode(Bits8::from(x)).iter())
            .flatten()
            .cloned()
            .collect();

        let decoded: Vec<u8> = decoder
            .decode(code.into_iter())
            .map(|x| x.unwrap())
            .map(|b| b.into())
            .collect();

        assert_eq!(decoded, plain);
    }

    #[test]
    fn test_build_decoding_for_all_equal_frequency() {
        let chars = CharacterFrequency::all_equal();
        let letters = example_letters();

        let encoding = Encoding::build(letters, &chars);
        let encoder = encoding.encoder();
        let decoder = encoding.decoder();

        let plain: Vec<_> = (0..255).collect();

        let code: Vec<_> = plain
            .iter()
            .map(|&x| encoder.encode(Bits8::from(x)).iter())
            .flatten()
            .cloned()
            .collect();

        let decoded: Vec<u8> = decoder
            .decode(code.into_iter())
            .map(|x| x.unwrap())
            .map(|b| b.into())
            .collect();

        assert_eq!(decoded, plain);
    }
}
