//! Errors produced by the GunnyScript parser.

use core::num::ParseIntError;

use fixed::ParseFixedError;

use crate::prelude::*;

#[derive(Debug, Clone)]
pub enum Error {
    /// Parsing error, providing a line/column number.
    LocatedParse(LocatedParseError),
    /// A parsing error without a location.
    Parse(ParseError),
    UnexpectedEof,
}

#[derive(Debug, Clone)]
pub struct LocatedParseError {
    line: usize,
    column: usize,
    err: ParseError,
}

impl LocatedParseError {
    pub fn new(line: usize, column: usize, err: ParseError) -> Self {
        Self { line, column, err }
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn column(&self) -> usize {
        self.column
    }

    pub fn err(&self) -> &ParseError {
        &self.err
    }
}

#[derive(Debug, Clone)]
pub enum ParseError {
    /// Invalid UTF-8 byte.
    InvalidUtf8,
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
    /// Dangling doc comment. Doc comments must occur just before root values or
    /// object properties.
    DanglingDocComment,
    /// Unexpected item encountered during parsing.
    UnexpectedItem,
    /// Duplicate property name in object.
    DuplicatePropertyName(String),
}
