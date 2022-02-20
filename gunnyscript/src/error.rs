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
    /// Invalid octal number.
    InvalidOctalNumber(ParseIntError),
    /// Invalid comment delimiter character.
    InvalidCommentDelimiter(char),
    /// Doc comments for values are only supported at the root of the document.
    UnexpectedDocComment,
    /// Invalid date/time value. Expecting RFC3339-formatted date.
    InvalidDateTime(time::error::Parse),
    /// Date value is missing its year.
    MissingYearInDate,
    /// Date value is missing its month.
    MissingMonthInDate,
    /// Date value is missing its day.
    MissingDayInDate,
    /// Failed to parse the year as an integer value.
    InvalidDateYear(ParseIntError),
    /// Failed to parse the month as an integer value.
    InvalidDateMonth(ParseIntError),
    /// Failed to parse the day as an integer value.
    InvalidDateDay(ParseIntError),
    /// Invalid date: one or more components is out of range.
    InvalidDate(time::error::ComponentRange),
}

impl From<ParseError> for Error {
    fn from(e: ParseError) -> Self {
        Self::Parse(e)
    }
}
