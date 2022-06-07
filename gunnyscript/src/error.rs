//! Errors produced by the Gunnyscript parser.

use core::str::Utf8Error;

#[derive(Debug, Clone, PartialEq)]
pub struct Located<E> {
    pub line: usize,
    pub err: E,
}

impl<E> Located<E> {
    pub fn new(line: usize, err: E) -> Self {
        Self { line, err }
    }
}

pub fn located_err<T, E>(line: usize, err: E) -> Result<T, Located<E>> {
    Err(Located::new(line, err))
}

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    IncompleteUtf8Char,
    UnexpectedEof,
    UnexpectedChar,
    Utf8Error(Utf8Error),
    InvalidPropertyId,
    StringLiteralDelimTooLong { len: usize, max_len: usize },
    MissingTerminator,
}

impl Error {
    pub fn located(self, line: usize) -> Located<Self> {
        Located { line, err: self }
    }
}
