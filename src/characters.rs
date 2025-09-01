use crate::bits_key::{Bits, BitsIter, BitsMap};

pub struct CharacterFrequency<B> {
    _freq: BitsMap<B, f32>,
    /// The accumulated frequency
    ///   $ P_k = p_0 + p_1 + dots + p_k $
    /// with $P_(-1)$ defined to zero.
    accu_freq: BitsMap<B, f32>,
    /// The accumulated frequency
    ///   $ P_k = p_0 + p_1 + dots + p_(k - 1) + p_k / 2 $
    /// with $P_(-1)$ defined to zero.
    accu_freq2: BitsMap<B, f32>,
}

impl<B> CharacterFrequency<B>
where
    B: Bits,
{
    pub fn accu_freq(&self, char: B) -> f32 {
        self.accu_freq[char]
    }

    pub fn accu_freq2(&self, char: B) -> f32 {
        self.accu_freq2[char]
    }

    pub fn all_equal() -> Self {
        CharacterCounter::all_equal().finish()
    }
}

pub struct CharacterCounter<B> {
    counts: BitsMap<B, usize>,
    total: usize,
}

impl<B> CharacterCounter<B>
where
    B: Bits,
{
    pub fn empty() -> Self {
        Self {
            counts: BitsMap::new(0),
            total: 0,
        }
    }

    pub fn all_equal() -> Self {
        Self {
            counts: BitsMap::new(1),
            total: 2usize.pow(B::N),
        }
    }

    pub fn count_one(&mut self, b: B) {
        self.counts[b] += 1;
        self.total += 1;
    }

    pub fn count(&mut self, it: impl Iterator<Item = B>) -> &mut Self {
        it.for_each(|b| self.count_one(b));
        self
    }

    pub fn finish(&self) -> CharacterFrequency<B> {
        let freq = self.freq();
        let n_zero_freq = freq.iter().filter(|(_, x)| **x == 0.0).count();
        let shared_freq = 0.05_f32.min(n_zero_freq as f32 * 0.005);
        let added_freq = shared_freq / (n_zero_freq as f32);
        let left_freq = 1.0 - shared_freq;

        let freq = freq.map(|_, x| {
            if *x == 0.0 {
                added_freq
            } else {
                *x * left_freq
            }
        });

        let sum: f32 = freq.iter().map(|(_, x)| *x).sum();
        if !((sum - 1.0).abs() < 1e-4) {
            panic!("after mending zero freqs, sum is {} not one", sum);
        }

        characters_from_freq(freq)
    }

    fn freq(&self) -> BitsMap<B, f32> {
        let mut freq = BitsMap::new(0.0);
        for i in BitsIter::<B>::begin_zero() {
            freq[i.clone()] = (self.counts[i.clone()] as f32) / (self.total as f32);
        }

        freq
    }
}

fn characters_from_freq<B>(freq: BitsMap<B, f32>) -> CharacterFrequency<B>
where
    B: Bits,
{
    let mut accu_freq = BitsMap::new(0.0);
    let mut accu_freq2 = BitsMap::new(0.0);

    accu_freq[B::zero()] = freq[B::zero()];
    accu_freq2[B::zero()] = freq[B::zero()] / 2.0;

    for i in BitsIter::<B>::begin_zero().skip(1) {
        accu_freq[i.clone()] = accu_freq[i.clone().prev().unwrap()] + freq[i.clone()];
        accu_freq2[i.clone()] = accu_freq[i.clone()] - freq[i.clone()] / 2.0;
    }

    CharacterFrequency {
        _freq: freq,
        accu_freq,
        accu_freq2,
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::bits::Bits8;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-6
    }

    fn approx_iter(a: impl Iterator<Item = f32>, b: impl Iterator<Item = f32>) -> bool {
        a.zip(b).all(|(a, b)| approx(a, b))
    }

    pub fn example_characters() -> CharacterFrequency<Bits8> {
        let mut counter = CharacterCounter::empty();

        let chars = [0, 1, 1, 1, 2, 2, 2, 2, 3, 3];

        counter
            .count(chars.into_iter().map(|c| Bits8::from(c)))
            .finish()
    }

    #[test]
    fn test_character_frequency() {
        let mut counter = CharacterCounter::empty();

        let chars = [0, 1, 1, 1, 2, 2, 2, 2, 3, 3];

        counter.count(chars.into_iter().map(|c| Bits8::from(c)));
        let freq = characters_from_freq(counter.freq());

        assert!(approx_iter(
            freq._freq.iter().map(|(_, x)| *x),
            [0.1, 0.3, 0.4, 0.2].into_iter()
        ));

        assert!(approx_iter(
            freq.accu_freq.iter().map(|(_, x)| *x),
            [0.1, 0.4, 0.8, 1.0].into_iter()
        ));

        assert!(approx_iter(
            freq.accu_freq2.iter().map(|(_, x)| *x),
            [0.05, 0.25, 0.6, 0.9].into_iter()
        ));
    }

    #[test]
    fn test_all_equal_frequency() {
        let chars = CharacterFrequency::<Bits8>::all_equal();

        for (_, freq) in chars._freq.iter() {
            assert!(approx(*freq, 1.0 / (2usize.pow(Bits8::N)) as f32))
        }
    }
}
