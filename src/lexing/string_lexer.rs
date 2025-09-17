use std::{
    collections::{HashMap, HashSet},
    ops::Deref,
};

use crate::letters::{LetterId, LetterIdIndexed};

#[derive(Debug, Clone)]
pub struct Tree(super::Tree<char, LetterId, HashMap<char, Tree>>);

impl Deref for Tree {
    type Target = super::Tree<char, LetterId, HashMap<char, Tree>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<super::Tree<char, LetterId, HashMap<char, Tree>>> for Tree {
    fn from(value: super::Tree<char, LetterId, HashMap<char, Tree>>) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone)]
pub struct StringLexer {
    tree: HashMap<char, Tree>,
}

pub type Iter<'t, It> = super::iter::LexingIter<'t, char, LetterId, HashMap<char, Tree>, It>;
pub type IterFromError<'t, It, E> =
    super::iter_from_error::LexingIter<'t, char, LetterId, HashMap<char, Tree>, It, E>;

impl super::Lexer for StringLexer {
    type Src = char;
    type Dst = LetterId;

    fn lex<It: Iterator<Item = Self::Src>>(&self, incoming: It) -> Iter<'_, It> {
        super::iter::LexingIter::new(&self.tree, incoming)
    }

    fn lex_from_error<E, It: Iterator<Item = Result<Self::Src, E>>>(
        &self,
        incoming: It,
    ) -> IterFromError<'_, It, E> {
        super::iter_from_error::LexingIter::new(&self.tree, incoming)
    }
}

impl StringLexer {
    pub fn new(tokens: &LetterIdIndexed<String>) -> Result<Self, super::LexemError> {
        let chars = tokens
            .iter()
            .map(|s| s.chars())
            .flatten()
            .collect::<HashSet<_>>();

        let roots = super::build_tree::<HashMap<_, _>, _, _, _, _>(
            tokens
                .iter_with_id()
                .map(|(letter_id, token)| (letter_id, super::Code::new(token.chars()))),
            chars.iter().cloned(),
        )?;

        Ok(Self { tree: roots })
    }
}

#[cfg(test)]
mod test {
    use super::super::Lexer;
    use super::*;

    #[test]
    #[should_panic]
    fn test_non_prefix_encoding() {
        let _ = StringLexer::new(&LetterIdIndexed::new(vec![
            "ab".to_string(),
            "a".to_string(),
        ]))
        .unwrap();
    }

    fn test_tokens() -> LetterIdIndexed<String> {
        LetterIdIndexed::new(vec![
            "aa".to_string(),
            "aba".to_string(),
            "abb".to_string(),
            "baa".to_string(),
            "bb".to_string(),
        ])
    }

    #[test]
    fn test_string_lexer() {
        let tokens = test_tokens();
        let lexer = StringLexer::new(&tokens).unwrap();

        let answer = ["aa", "bb", "abb", "baa", "aa", "aba"];
        let answer_ids = answer
            .iter()
            .map(|tok| {
                tokens
                    .iter_with_id()
                    .find(|(_, t)| t == tok)
                    .map(|(id, _)| id)
                    .unwrap()
            })
            .collect::<Vec<_>>();

        let r = lexer
            .lex(answer.concat().chars())
            .map(|x| x.unwrap())
            .collect::<Vec<_>>();

        assert_eq!(r, answer_ids);
    }

    #[test]
    #[should_panic]
    fn test_string_lexer_unexpected_termination() {
        let tokens = test_tokens();
        let lexer = StringLexer::new(&tokens).unwrap();

        let _ = lexer
            .lex("aab".chars())
            .map(|x| x.unwrap())
            .collect::<Vec<_>>();
    }

    #[test]
    #[should_panic]
    fn test_string_lexer_invalid_char() {
        let tokens = test_tokens();
        let lexer = StringLexer::new(&tokens).unwrap();

        let _ = lexer
            .lex("aabbbaacaa".chars())
            .map(|x| x.unwrap())
            .collect::<Vec<_>>();
    }

    #[test]
    #[should_panic]
    fn test_string_lexer_unexpected_char() {
        let tokens = test_tokens();
        let lexer = StringLexer::new(&tokens).unwrap();

        let _ = lexer
            .lex("aaabababbb".chars())
            .map(|x| x.unwrap())
            .collect::<Vec<_>>();
    }
}
