use std::collections::BTreeSet;

use crate::bits_key::{Bits, BitsIter, BitsMap};
use crate::characters::CharacterFrequency;
use crate::letters::{Code, LetterCosts, LetterId};

struct EncodingBuilder<B> {
    char2code: BitsMap<B, Option<Code>>,
    letters: LetterCosts,
}

impl<B> EncodingBuilder<B>
where
    B: Bits,
{
    pub fn new(letters: LetterCosts) -> Self {
        Self {
            char2code: BitsMap::new(None),
            letters,
        }
    }

    fn set_code(&mut self, char: B, code: Code) {
        if self.char2code[char.clone()].is_some() {
            panic!("code for character {:?} is already set", char);
        }

        self.char2code[char] = Some(code);
    }

    /// Construct encoding scheme using method from
    ///   K. Mehlhorn, "An efficient algorithm for constructing nearly optimal prefix codes,"
    ///   in IEEE Transactions on Information Theory, vol. 26, no. 5, pp. 513-517, September 1980,
    fn code(&mut self, l: B, r: B, prefix: &Code, characters: &CharacterFrequency<B>) {
        if l == r {
            self.set_code(l, prefix.clone());
        } else {
            let apl = l.prev().map(|p| characters.accu_freq(p)).unwrap_or(0.0);
            let apr = characters.accu_freq(r.clone());

            let mut partitions = self.letters.map(|m| {
                let lm = apl
                    + (apr - apl)
                        * m.before()
                            .map(|j| self.letters.c().powi(self.letters.cost(j)))
                            .sum::<f32>();
                let rm = lm + (apr - apl) * self.letters.c().powi(self.letters.cost(m));

                BitsIter::<B>::closed_interval(l.clone(), r.clone())
                    .filter(|char| {
                        let s = characters.accu_freq2(char.clone());
                        lm <= s && s < rm
                    })
                    .collect::<BTreeSet<_>>()
            });

            if partitions.first().unwrap().is_empty() {
                let m = self
                    .letters
                    .letters()
                    .filter(|&m| !partitions[m].is_empty())
                    .next()
                    .unwrap();
                if !partitions[m].remove(&l) {
                    panic!("expected l to be in partition[m]");
                }
                partitions.first_mut().unwrap().insert(l.clone());
            }

            if partitions.last().unwrap().is_empty() {
                let m = self
                    .letters
                    .letters()
                    .rev()
                    .filter(|&m| !partitions[m].is_empty())
                    .next()
                    .unwrap();

                if !partitions[m].remove(&r) {
                    panic!("expected r to be in partition[m]");
                }
                partitions.last_mut().unwrap().insert(r.clone());
            }

            for (m, par) in partitions.into_iter() {
                if !par.is_empty() {
                    self.code(
                        par.iter().min().unwrap().clone(),
                        par.iter().max().unwrap().clone(),
                        &prefix.join(m),
                        characters,
                    )
                }
            }
        }
    }

    pub fn build(&mut self, characters: &CharacterFrequency<B>) {
        self.code(B::zero(), B::biggest(), &Code::empty(), characters)
    }

    pub fn finish(self) -> Encoding<B> {
        Encoding {
            char2code: self.char2code.map(|_, c| c.as_ref().unwrap().clone()),
            n_letters: self.letters.n_letters(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Encoding<B>
where
    B: Bits,
{
    char2code: BitsMap<B, Code>,
    n_letters: LetterId,
}

impl<B> Encoding<B>
where
    B: Bits,
{
    pub fn encoder(&self) -> Encoder<B> {
        Encoder::from_encoding(self)
    }

    pub fn build(letters: LetterCosts, characters: &CharacterFrequency<B>) -> Self {
        let mut builder = EncodingBuilder::new(letters);
        builder.build(characters);
        builder.finish()
    }

    pub fn char2code(&self) -> &BitsMap<B, Code> {
        &self.char2code
    }
}

impl<B> Encoding<B>
where
    B: Bits,
{
    pub fn decoder(&self) -> Decoder<B> {
        Decoder::from_encoding(self)
    }
}

pub mod decoder;
pub mod encoder;

pub use decoder::Decoder;
pub use encoder::Encoder;
