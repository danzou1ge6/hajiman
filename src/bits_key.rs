use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};

use serde::ser::SerializeSeq;

pub trait Seq: Sized {
    fn prev(&self) -> Option<Self>;
    fn succ(&self) -> Option<Self>;
}

pub struct Padded<T> {
    pub data: T,
    pub original_length: usize,
}

impl<T> Padded<T> {
    pub fn map<T1>(self, f: impl FnOnce(T) -> T1) -> Padded<T1> {
        Padded {
            data: f(self.data),
            original_length: self.original_length,
        }
    }
}

#[derive(Debug)]
pub enum ConcatError<E> {
    Parent(E),
    Io(std::io::Error),
}

impl<E> ConcatError<E> {
    pub fn unwrap_parent(self) -> E {
        match self {
            Self::Parent(p) => p,
            _ => panic!("called unwrap_parent on ConcatError::Io"),
        }
    }
}

impl<E> From<E> for ConcatError<E> {
    fn from(value: E) -> Self {
        Self::Parent(value)
    }
}

pub trait Bits: Seq + Eq + Ord + Debug + Clone {
    const N: u32;

    fn iter_bytes(arr: &[u8]) -> Padded<impl Iterator<Item = Self>>;
    fn concat<E>(
        it: impl Iterator<Item = Result<Self, E>>,
        writer: impl std::io::Write,
    ) -> Result<(), ConcatError<E>>;
    fn to_usize(self) -> usize;
    fn zero() -> Self;
    fn biggest() -> Self;
}

pub struct BitsIter<B> {
    now: Option<B>,
    before: Option<B>,
}

impl<B> BitsIter<B>
where
    B: Bits,
{
    pub fn begin_zero() -> Self {
        Self {
            now: Some(B::zero()),
            before: None,
        }
    }

    pub fn closed_interval(left: B, right: B) -> Self {
        Self {
            now: Some(left),
            before: Some(right),
        }
    }
}

impl<B> Iterator for BitsIter<B>
where
    B: Bits,
{
    type Item = B;
    fn next(&mut self) -> Option<Self::Item> {
        let r = self.now.clone();
        self.now = self.now.clone()?.succ();
        if self
            .before
            .as_ref()
            .is_some_and(|_| r.clone() == self.before.clone())
        {
            self.now = None;
        }
        r
    }
}

pub mod bits;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitsMap<B, T>(Vec<T>, PhantomData<B>);

impl<B, T> BitsMap<B, T>
where
    B: Bits,
{
    pub fn len() -> usize {
        2usize.pow(B::N)
    }

    pub fn new(t: T) -> Self
    where
        T: Clone,
    {
        Self(vec![t; Self::len()], PhantomData)
    }

    pub fn iter(&self) -> impl Iterator<Item = (B, &T)> {
        BitsIter::begin_zero().zip(self.0.iter())
    }

    pub fn map<T1>(&self, mut f: impl FnMut(B, &T) -> T1) -> BitsMap<B, T1> {
        BitsMap(self.iter().map(|(b, t)| f(b, t)).collect(), PhantomData)
    }
}

impl<B, T> Index<B> for BitsMap<B, T>
where
    B: Bits,
{
    type Output = T;
    fn index(&self, index: B) -> &Self::Output {
        unsafe { self.0.get_unchecked(index.to_usize()) }
    }
}

impl<B, T> IndexMut<B> for BitsMap<B, T>
where
    B: Bits,
{
    fn index_mut(&mut self, index: B) -> &mut Self::Output {
        unsafe { self.0.get_unchecked_mut(index.to_usize()) }
    }
}

mod serialize {
    use super::*;

    impl<B, T> serde::Serialize for BitsMap<B, T>
    where
        B: Bits,
        T: serde::Serialize,
    {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let mut seq = serializer.serialize_seq(Some(BitsMap::<B, T>::len()))?;
            for (_, t) in self.iter() {
                seq.serialize_element(t)?;
            }
            seq.end()
        }
    }

    struct Visitor<B, T> {
        _phantom: PhantomData<(B, T)>,
    }

    impl<B, T> Visitor<B, T> {
        fn new() -> Self {
            Self {
                _phantom: PhantomData,
            }
        }
    }

    impl<'de, B, T> serde::de::Visitor<'de> for Visitor<B, T>
    where
        T: serde::Deserialize<'de>,
        B: Bits,
    {
        type Value = BitsMap<B, T>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a vector of length of 2's power")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let mut v = Vec::new();
            while let Some(item) = seq.next_element()? {
                v.push(item);
            }
            use serde::de::Error;

            if v.len() != BitsMap::<B, T>::len() {
                return Err(A::Error::custom(format!(
                    "expected 2^{} elements, got {}",
                    B::N,
                    v.len()
                )));
            }

            Ok(BitsMap(v, PhantomData))
        }
    }

    impl<'de, B, T> serde::Deserialize<'de> for BitsMap<B, T>
    where
        B: Bits,
        T: serde::Deserialize<'de>,
    {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_seq(Visitor::new())
        }
    }
}
