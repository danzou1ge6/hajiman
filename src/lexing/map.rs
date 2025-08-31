use std::collections::HashMap;
use std::hash::Hash;

use crate::letters::{LetterId, LetterIdIndexed};

pub trait Map<K> {
    type Output;

    fn init(ks: impl Iterator<Item = K>, v: Self::Output) -> Self;
    fn get<'a>(&'a self, k: &K) -> Option<&'a Self::Output>
    where
        Self::Output: 'a;
    fn get_mut<'a>(&'a mut self, k: &K) -> &'a mut Self::Output
    where
        Self::Output: 'a;
    fn into_iter(self) -> impl Iterator<Item = (K, Self::Output)>;
}

impl<K, V> Map<K> for HashMap<K, V>
where
    K: Eq + Hash,
    V: Clone,
{
    type Output = V;

    fn init(ks: impl Iterator<Item = K>, v: V) -> Self {
        ks.zip(std::iter::repeat(v)).collect()
    }

    fn get_mut<'a>(&'a mut self, k: &K) -> &'a mut V
    where
        V: 'a,
    {
        HashMap::get_mut(self, k).unwrap()
    }

    fn get<'a>(&'a self, k: &K) -> Option<&'a V>
    where
        V: 'a,
    {
        HashMap::get(self, k)
    }

    fn into_iter(self) -> impl Iterator<Item = (K, V)> {
        IntoIterator::into_iter(self)
    }
}

impl<V> Map<LetterId> for LetterIdIndexed<V>
where
    V: Clone,
{
    type Output = V;

    fn init(ks: impl Iterator<Item = LetterId>, v: V) -> Self {
        LetterIdIndexed::repeat(v, ks.count())
    }

    fn get_mut<'a>(&'a mut self, k: &LetterId) -> &'a mut V
    where
        V: 'a,
    {
        &mut self[*k]
    }

    fn get<'a>(&'a self, k: &LetterId) -> Option<&'a V>
    where
        V: 'a,
    {
        Some(&self[*k])
    }

    fn into_iter(self) -> impl Iterator<Item = (LetterId, V)> {
        self.into_iter()
    }
}
