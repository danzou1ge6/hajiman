use std::{marker::PhantomData, ops::Deref};

mod map;
pub use map::Map;

pub trait Lexer {
    type Src;
    type Dst;
    fn lex_from_error<E, It: Iterator<Item = std::result::Result<Self::Src, E>>>(
        &self,
        incoming: It,
    ) -> impl Iterator<Item = iter_from_error::Result<Self::Dst, Self::Src, E>>;

    fn lex<It: Iterator<Item = Self::Src>>(
        &self,
        incoming: It,
    ) -> impl Iterator<Item = iter::Result<Self::Dst, Self::Src>>;
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Code<I>(Vec<I>);

impl<I> Code<I>
where
    I: Clone,
{
    pub fn join(&self, tail: I) -> Self {
        let mut v = self.0.clone();
        v.push(tail);
        Self(v)
    }
    pub fn head(&self) -> I {
        self.0.first().cloned().unwrap()
    }

    pub fn tail(&self) -> Option<Code<I>> {
        if self.0.len() >= 2 {
            Some(Self(self.0[1..].to_vec()))
        } else {
            None
        }
    }

    pub fn new(it: impl Iterator<Item = I>) -> Self {
        Self(it.collect())
    }
}

impl<I> Code<I> {
    pub fn empty() -> Self {
        Self(Vec::new())
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &I> {
        self.0.iter()
    }
}

#[derive(Debug, Clone)]
pub enum Tree<I, L, M> {
    Leaf(L),
    Invalid,
    Inner(M, PhantomData<I>),
}

#[derive(Debug, Clone)]
pub enum Layer<I, L> {
    Leaf(L),
    Invalid,
    Inner(Vec<(L, Code<I>)>),
}

#[derive(Debug)]
pub struct NonPrefixFreeError;

impl<I, L> Layer<I, L> {
    fn or_inner(&mut self) -> std::result::Result<&mut Vec<(L, Code<I>)>, NonPrefixFreeError> {
        use Layer::*;
        match self {
            Inner(..) => {}
            Leaf(..) => return Err(NonPrefixFreeError),
            Invalid => *self = Inner(vec![]),
        }
        match self {
            Inner(v) => Ok(v),
            _ => unreachable!(),
        }
    }

    fn or_leaf_set_to(&mut self, leaf: L) -> std::result::Result<(), NonPrefixFreeError> {
        use Layer::*;
        match self {
            Inner(..) => return Err(NonPrefixFreeError),
            _ => *self = Leaf(leaf),
        }
        Ok(())
    }
}

pub fn build_tree<M1, I, L, M, T>(
    suffice: impl Iterator<Item = (L, Code<I>)>,
    code_letters: impl Iterator<Item = I> + Clone,
) -> std::result::Result<M, NonPrefixFreeError>
where
    M: Map<I, Output = T>,
    T: From<Tree<I, L, M>>,
    M1: Map<I, Output = Layer<I, L>>,
    I: Clone,
{
    use Layer::*;

    let mut layer = M1::init(code_letters.clone(), Invalid);

    for (l, code) in suffice {
        let head = code.head();

        if let Some(tail) = code.tail() {
            layer.get_mut(&head).or_inner()?.push((l, tail));
        } else {
            layer.get_mut(&head).or_leaf_set_to(l)?;
        }
    }

    let mut tree_layer = M::init(code_letters.clone(), Tree::Invalid.into());
    for (letter, node) in layer.into_iter() {
        *tree_layer.get_mut(&letter) = match node {
            Leaf(leaf) => Tree::Leaf(leaf).into(),
            Invalid => Tree::Invalid.into(),
            Inner(suffice) => Tree::Inner(
                build_tree::<M1, _, _, _, _>(suffice.into_iter(), code_letters.clone())?,
                PhantomData,
            )
            .into(),
        }
    }

    Ok(tree_layer)
}

#[derive(Debug, Clone)]
pub enum Error<W, E> {
    UnexpectedTermination(Vec<W>),
    Unexpected(Vec<W>, W),
    Invalid(W),
    Parent(E),
}

impl<W, E> Error<W, E> {
    pub fn map<W1>(self, mut f: impl FnMut(W) -> W1) -> Error<W1, E> {
        use Error::*;
        match self {
            UnexpectedTermination(prefix) => {
                UnexpectedTermination(prefix.into_iter().map(f).collect())
            }
            Unexpected(prefix, now) => Unexpected(prefix.into_iter().map(&mut f).collect(), f(now)),
            Invalid(w) => Invalid(f(w)),
            Parent(e) => Parent(e),
        }
    }

    pub fn flatten<E1>(self, f1: impl FnOnce(E) -> E1, f2: impl FnOnce(Error<W, !>) -> E1) -> E1 {
        use Error::*;
        let e = match self {
            UnexpectedTermination(x) => UnexpectedTermination(x),
            Unexpected(x, w) => Unexpected(x, w),
            Invalid(w) => Invalid(w),
            Parent(e) => return f1(e),
        };
        f2(e)
    }
}

pub mod iter_from_error {
    use super::*;

    pub struct LexingIter<'t, I, L, M, It, E> {
        roots: &'t M,
        current_tree: &'t M,
        prefix: Vec<I>,
        incoming: It,
        _phantom: PhantomData<(L, E)>,
    }

    impl<'t, I, L, M, It, E> LexingIter<'t, I, L, M, It, E> {
        pub fn new(roots: &'t M, incoming: It) -> Self {
            Self {
                roots,
                current_tree: roots,
                prefix: Vec::new(),
                incoming,
                _phantom: PhantomData,
            }
        }

        pub fn cont<It2>(self, f: impl FnOnce(It) -> It2) -> LexingIter<'t, I, L, M, It2, E> {
            LexingIter {
                roots: self.roots,
                current_tree: self.current_tree,
                prefix: self.prefix,
                incoming: f(self.incoming),
                _phantom: PhantomData,
            }
        }
    }

    pub type Result<L, I, E> = std::result::Result<L, Error<I, E>>;

    impl<'t, I, L, M, It, T, E> Iterator for LexingIter<'t, I, L, M, It, E>
    where
        It: Iterator<Item = std::result::Result<I, E>>,
        I: Clone + 't,
        L: Clone + 't,
        M: Map<I, Output = T>,
        T: Deref<Target = Tree<I, L, M>> + 't,
    {
        type Item = Result<L, I, E>;
        fn next(&mut self) -> Option<Self::Item> {
            use Tree::*;

            while let Some(i) = self.incoming.next() {
                match i {
                    Ok(i) => match self.current_tree.get(&i) {
                        Some(t) => match t.deref() {
                            Invalid => {
                                return Some(Err(Error::Unexpected(
                                    self.prefix.clone(),
                                    i.clone(),
                                )));
                            }
                            Leaf(l) => {
                                self.prefix.clear();
                                self.current_tree = self.roots;
                                return Some(Ok(l.clone()));
                            }
                            Inner(children, _) => {
                                self.prefix.push(i.clone());
                                self.current_tree = children;
                            }
                        },
                        None => return Some(Err(Error::Invalid(i.clone()))),
                    },
                    Err(e) => return Some(Err(Error::Parent(e))),
                }
            }

            if self.prefix.is_empty() {
                None
            } else {
                Some(Err(Error::UnexpectedTermination(self.prefix.clone())))
            }
        }
    }
}

pub mod iter {
    use super::*;

    pub type Error<W> = super::Error<W, !>;

    pub struct LexingIter<'t, I, L, M, It> {
        roots: &'t M,
        current_tree: &'t M,
        prefix: Vec<I>,
        incoming: It,
        _phantom: PhantomData<L>,
    }

    impl<'t, I, L, M, It> LexingIter<'t, I, L, M, It> {
        pub fn new(roots: &'t M, incoming: It) -> Self {
            Self {
                roots,
                current_tree: roots,
                prefix: Vec::new(),
                incoming,
                _phantom: PhantomData,
            }
        }

        pub fn cont<It2>(self, f: impl FnOnce(It) -> It2) -> LexingIter<'t, I, L, M, It2> {
            LexingIter {
                roots: self.roots,
                current_tree: self.current_tree,
                prefix: self.prefix,
                incoming: f(self.incoming),
                _phantom: PhantomData,
            }
        }
    }

    pub type Result<L, I> = std::result::Result<L, Error<I>>;

    impl<'t, I, L, M, It, T> Iterator for LexingIter<'t, I, L, M, It>
    where
        It: Iterator<Item = I>,
        I: Clone + 't,
        L: Clone + 't,
        M: Map<I, Output = T>,
        T: Deref<Target = Tree<I, L, M>> + 't,
    {
        type Item = Result<L, I>;
        fn next(&mut self) -> Option<Self::Item> {
            use Tree::*;

            while let Some(i) = self.incoming.next() {
                match self.current_tree.get(&i) {
                    Some(t) => match t.deref() {
                        Invalid => {
                            return Some(Err(Error::Unexpected(self.prefix.clone(), i.clone())));
                        }
                        Leaf(l) => {
                            self.prefix.clear();
                            self.current_tree = self.roots;
                            return Some(Ok(l.clone()));
                        }
                        Inner(children, _) => {
                            self.prefix.push(i.clone());
                            self.current_tree = children;
                        }
                    },
                    None => return Some(Err(Error::Invalid(i.clone()))),
                }
            }

            if self.prefix.is_empty() {
                None
            } else {
                Some(Err(Error::UnexpectedTermination(self.prefix.clone())))
            }
        }
    }
}

pub mod string_lexer;
pub use string_lexer::StringLexer;
