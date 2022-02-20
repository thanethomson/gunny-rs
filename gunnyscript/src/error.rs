//! Errors produced by the GunnyScript parser.

use core::num::ParseIntError;

use fixed::ParseFixedError;

use crate::prelude::*;

#[derive(Debug, Clone)]
pub enum Error {
    Encoding(EncodingError),
    Parse(ParseError),
}

#[derive(Debug, Clone)]
pub enum EncodingError {
    /// Invalid UTF-8 byte.
    InvalidUtf8,
}

impl From<EncodingError> for Error {
    fn from(e: EncodingError) -> Self {
        Self::Encoding(e)
    }
}

#[derive(Debug, Clone)]
pub enum ParseError {
    /// Unexpected character in input stream.
    UnexpectedChar(char),
    /// Invalid identifier in input stream.
    InvalidIdentifier(String),
    /// Invalid character in property name.
    InvalidPropertyNameChar(char),
    /// Invalid escape sequence in string.
    InvalidEscapeSequence(char),
    /// Invalid character in literal string definition.
    InvalidLiteralStringChar(char),
    /// Invalid hexadecimal number.
    InvalidHexNumber(ParseIntError),
    /// Invalid signed number.
    InvalidSignedNumber(ParseIntError),
    /// Invalid unsigned number.
    InvalidUnsignedNumber(ParseIntError),
    /// Invalid fixed-point number.
    InvalidFixedPointNumber(ParseFixedError),
}

impl From<ParseError> for Error {
    fn from(e: ParseError) -> Self {
        Self::Parse(e)
    }
}
