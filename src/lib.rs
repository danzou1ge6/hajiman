#![feature(never_type)]
#![feature(iter_array_chunks)]
#![feature(string_into_chars)]
#![allow(refining_impl_trait)]

mod bits_key;
mod characters;
pub mod cli;
mod encoding;
mod hajimi;
mod jimi;
mod letters;
mod lexing;

pub use bits_key::{Bits, BitsIter, bits};

pub use characters::{CharacterCounter, CharacterFrequency};
pub use encoding::{Decoder, Encoder, Encoding};
pub use hajimi::{HAJIMI, hajimi_tokens};
pub use jimi::{JimiDecoder, JimiEncoder, JimiEncoding, JimiError};
pub use letters::LetterCosts;
pub use lexing::{LexemError, Lexer, StringLexer};

pub use serde_json;
