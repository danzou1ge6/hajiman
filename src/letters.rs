use std::ops::{Index, IndexMut};

use crate::lexing;

#[derive(Debug, Clone)]
pub struct LetterCosts {
    costs: LetterIdIndexed<i32>,
    /// Root of the characteristics equaion
    ///   $ sum x^(c_j) = 1 $
    /// where $c_j$ is the cost of the $j$ th letter.
    c: f32,
}

#[derive(Debug, Clone)]
struct Polynomial {
    /// Represents polynomial
    /// $ f(x) = a_0 + a_1 x + dots + a_n x^n $
    coefs: Vec<isize>,
}

impl Polynomial {
    pub fn zero() -> Self {
        Self { coefs: Vec::new() }
    }

    pub fn add_power(&mut self, power: i32) {
        assert!(power >= 0);
        let power = power as usize;

        self.coefs.resize((power + 1).max(self.coefs.len()), 0);
        self.coefs[power] += 1;
    }

    pub fn positive_roots(&self) -> Vec<f32> {
        let normalizer = self.coefs.last().unwrap().clone() as f32;
        let mut coefs = vec![0.0; self.coefs.len() - 1];
        let n = self.coefs.len() - 1;

        for (i, c) in self.coefs.iter().enumerate().take(n) {
            coefs[n - 1 - i] = *c as f32 / normalizer;
        }

        roots::find_roots_sturm(&coefs, &mut 1e-5)
            .into_iter()
            .filter_map(|x| x.ok())
            .filter(|x| *x > 0.0)
            .collect()
    }

    pub fn coef_mut(&mut self, deg: usize) -> &mut isize {
        &mut self.coefs[deg]
    }
}

#[derive(Debug)]
pub struct SolveCharacteristicsEquationFail;

impl LetterCosts {
    pub fn len(&self) -> usize {
        self.costs.len()
    }

    pub fn cost(&self, i: LetterId) -> i32 {
        self.costs[i]
    }

    pub fn c(&self) -> f32 {
        self.c
    }

    pub fn build(costs: LetterIdIndexed<i32>) -> Result<Self, SolveCharacteristicsEquationFail> {
        let mut poly = Polynomial::zero();
        costs.iter().for_each(|&cost| {
            poly.add_power(cost);
        });
        *poly.coef_mut(0) -= 1;

        let c = *poly
            .positive_roots()
            .first()
            .ok_or(SolveCharacteristicsEquationFail)?;

        let r = LetterCosts { costs, c };
        r.check();

        Ok(r)
    }

    fn check(&self) {
        let sum: f32 = self.costs.iter().map(|c| self.c.powi(*c)).sum();
        if !((sum - 1.0).abs() < 1e-4) {
            panic!("calculated root is wrong! sum is {}", sum);
        }
    }

    pub fn letters(&self) -> impl DoubleEndedIterator<Item = LetterId> + Clone {
        self.costs.iter_id()
    }

    pub fn map<T>(&self, mut f: impl FnMut(LetterId) -> T) -> LetterIdIndexed<T> {
        self.costs.map_by_ref(|id, _| f(id))
    }

    pub fn n_letters(&self) -> LetterId {
        LetterId(self.len())
    }
}

pub type Code = lexing::Code<LetterId>;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct LetterId(usize);

impl LetterId {
    pub fn before(self) -> impl Iterator<Item = LetterId> + Clone {
        (0..self.0).map(|x| LetterId(x))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct LetterIdIndexed<T>(Vec<T>);

impl<T> LetterIdIndexed<T> {
    pub fn new(v: Vec<T>) -> Self {
        Self(v)
    }

    pub fn repeat(t: T, len: usize) -> Self
    where
        T: Clone,
    {
        Self(vec![t; len])
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn iter_id(&self) -> impl DoubleEndedIterator<Item = LetterId> + Clone {
        (0..self.0.len()).map(LetterId)
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &T> {
        self.0.iter()
    }

    pub fn iter_with_id(&self) -> impl DoubleEndedIterator<Item = (LetterId, &T)> {
        self.0.iter().enumerate().map(|(i, x)| (LetterId(i), x))
    }

    pub fn map<T1>(self, mut f: impl FnMut(LetterId, T) -> T1) -> LetterIdIndexed<T1> {
        LetterIdIndexed(
            self.0
                .into_iter()
                .enumerate()
                .map(|(i, x)| f(LetterId(i), x))
                .collect(),
        )
    }

    pub fn map_by_ref<T1>(&self, mut f: impl FnMut(LetterId, &T) -> T1) -> LetterIdIndexed<T1> {
        LetterIdIndexed(
            self.0
                .iter()
                .enumerate()
                .map(|(i, x)| f(LetterId(i), x))
                .collect(),
        )
    }

    pub fn first(&self) -> Option<&T> {
        self.0.first()
    }

    pub fn last(&self) -> Option<&T> {
        self.0.last()
    }

    pub fn first_mut(&mut self) -> Option<&mut T> {
        self.0.first_mut()
    }

    pub fn last_mut(&mut self) -> Option<&mut T> {
        self.0.last_mut()
    }

    pub fn into_iter(self) -> impl Iterator<Item = (LetterId, T)> {
        self.0
            .into_iter()
            .enumerate()
            .map(|(i, x)| (LetterId(i), x))
    }
}

impl<T> Index<&LetterId> for LetterIdIndexed<T> {
    type Output = T;
    fn index(&self, index: &LetterId) -> &Self::Output {
        &self.0[index.0]
    }
}

impl<T> Index<LetterId> for LetterIdIndexed<T> {
    type Output = T;
    fn index(&self, index: LetterId) -> &Self::Output {
        &self.0[index.0]
    }
}

impl<T> IndexMut<LetterId> for LetterIdIndexed<T> {
    fn index_mut(&mut self, index: LetterId) -> &mut Self::Output {
        &mut self.0[index.0]
    }
}

#[cfg(test)]
pub mod test {
    use super::*;

    pub fn example_letters() -> LetterCosts {
        let costs = LetterIdIndexed::new(vec![1, 1, 1, 2, 2, 2, 2, 3, 3, 4]);

        LetterCosts::build(costs).expect("cannot build Letters")
    }

    #[test]
    fn test_build_letters() {
        let _ = example_letters();
    }
}
