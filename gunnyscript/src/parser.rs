//! Parsing functionality for GunnyScript.

use core::marker::PhantomData;

use bytes::{Buf, Bytes};
use time::{Date, OffsetDateTime};

use crate::{
    encoding::{Decoder, Utf8Decoder},
    prelude::*,
    Error, Fixed, Number, ParseError,
};

#[derive(Debug, Clone, Copy)]
pub struct Location {
    pub line: usize,
    pub column: usize,
}

impl Default for Location {
    fn default() -> Self {
        // Prefer human-readable locations.
        Self { line: 1, column: 1 }
    }
}

impl Location {
    #[inline]
    pub fn next_line(&mut self) {
        self.line += 1;
        self.column = 1;
    }

    #[inline]
    pub fn next_column(&mut self) {
        self.column += 1;
    }
}

/// Error or incomplete.
#[derive(Debug, Clone)]
pub enum EoI {
    Error(Location, Error),
    Incomplete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Linespace,
    DocComment(String),
    SimpleValue(SimpleValue),
    Start(ComplexValue),
    PropertyName(String),
    End(ComplexValue),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimpleValue {
    Null,
    Boolean(bool),
    String(String),
    Number(Number),
    Date(Date),
    DateTime(OffsetDateTime),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComplexValue {
    Array,
    Object,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    ExpectingValue,
    ExpectingPropertyName,
}

impl Default for State {
    fn default() -> Self {
        // Every parser starts off by expecting a value. We only expect
        // identifiers when inside an object.
        Self::ExpectingValue
    }
}

pub type Utf8Parser = Parser<Utf8Decoder>;

#[derive(Debug, Default)]
pub struct Parser<D> {
    state: State,
    location: Location,
    buf_location: Location,
    maybe_peek: Option<char>,
    newline_count: usize,
    nesting: Vec<ComplexValue>,
    _decoder: PhantomData<D>,
}

impl<D: Decoder> Parser<D> {
    // Error at a specific location within the buffer.
    #[inline]
    fn err_in_buf<E: Into<Error>>(&self, e: E) -> EoI {
        EoI::Error(self.buf_location, e.into())
    }

    // Error at the beginning of the buffer.
    #[inline]
    fn err_at_buf<E: Into<Error>>(&self, e: E) -> EoI {
        EoI::Error(self.location, e.into())
    }

    fn next_char(&mut self, buf: &mut Bytes) -> Result<char, EoI> {
        if let Some(ch) = self.maybe_peek.take() {
            // If we've already peeked at the next character, just return it
            // without modifying our location information.
            return Ok(ch);
        }
        let ch = D::decode_char(buf)
            .map_err(|e| self.err_in_buf(e))?
            .ok_or(EoI::Incomplete)?;
        if ch == '\n' {
            self.buf_location.next_line();
        } else if ch != '\r' {
            self.buf_location.next_column();
        }
        Ok(ch)
    }

    pub fn next(&mut self, buf: &mut Bytes) -> Result<Event, EoI> {
        let mut maybe_ev = None;

        while maybe_ev.is_none() {
            self.buf_location = self.location;
            // We automatically consume and discard any whitespace.
            self.skip_whitespace(buf)?;

            let mut ch = self.next_char(buf)?;
            while ch == '\n' {
                self.newline_count += 1;
                // We only care whether we encounter a single linespace. More than
                // one doesn't matter.
                if self.newline_count == 2 {
                    return Ok(Event::Linespace);
                }
                self.skip_whitespace(buf)?;
                ch = self.next_char(buf)?;
            }
            self.newline_count = 0;

            maybe_ev = match self.state {
                State::ExpectingValue => match ch {
                    '{' => Some(self.start_object()),
                    '[' => Some(self.start_array()),
                    ']' => Some(self.end_complex_value(ch, ComplexValue::Array)?),
                    '-' | '0'..='9' => Some(self.parse_number_or_date(ch, buf)?),
                    't' | 'f' => Some(self.parse_bool(ch, buf)?),
                    'n' => Some(self.parse_null(ch, buf)?),
                    '#' => Some(self.parse_string_literal(buf)?),
                    '"' => Some(self.parse_string(buf)?),
                    '/' => Some(self.parse_pre_value_comment(buf)?),
                    ',' => {
                        if self.nesting.is_empty() {
                            return Err(self.err_in_buf(ParseError::UnexpectedChar(ch)));
                        } else {
                            None
                        }
                    }
                    _ => return Err(self.err_in_buf(ParseError::UnexpectedChar(ch))),
                },
                State::ExpectingPropertyName => match ch {
                    '}' => Some(self.end_complex_value(ch, ComplexValue::Object)?),
                    'a'..='z' | 'A'..='Z' => {
                        let id = self.parse_property_name(ch, buf)?;
                        self.state = State::ExpectingValue;
                        Some(id)
                    }
                    '/' => Some(self.parse_pre_propname_comment(buf)?),
                    ',' => None,
                    _ => return Err(self.err_in_buf(ParseError::InvalidPropertyNameChar(ch))),
                },
            };
            if let Some(Event::SimpleValue(_)) = maybe_ev {
                if let Some(ComplexValue::Object) = self.nesting.last() {
                    self.state = State::ExpectingPropertyName;
                }
            }
        }
        Ok(maybe_ev.unwrap())
    }

    fn consume_until_not(&mut self, buf: &mut Bytes, keep_matching: &[char]) -> Result<(), EoI> {
        while buf.has_remaining() {
            let ch = self.next_char(buf)?;
            if !keep_matching.contains(&ch) {
                self.maybe_peek = Some(ch);
                return Ok(());
            }
        }
        Err(EoI::Incomplete)
    }

    fn skip_whitespace(&mut self, buf: &mut Bytes) -> Result<(), EoI> {
        self.consume_until_not(buf, &[' ', '\t', '\r'])
    }

    fn start_object(&mut self) -> Event {
        self.nesting.push(ComplexValue::Object);
        self.state = State::ExpectingPropertyName;
        Event::Start(ComplexValue::Object)
    }

    fn start_array(&mut self) -> Event {
        self.nesting.push(ComplexValue::Array);
        Event::Start(ComplexValue::Array)
    }

    fn end_complex_value(&mut self, ch: char, cv: ComplexValue) -> Result<Event, EoI> {
        if self.nesting.is_empty() {
            return Err(self.err_in_buf(ParseError::UnexpectedChar(ch)));
        }
        let nested = *self.nesting.last().unwrap();
        if nested == cv {
            let _ = self.nesting.pop();
            Ok(Event::End(cv))
        } else {
            Err(self.err_in_buf(ParseError::UnexpectedChar(ch)))
        }
    }

    fn parse_non_string_simple_value(
        &mut self,
        first_char: char,
        buf: &mut Bytes,
    ) -> Result<String, EoI> {
        let mut s = String::new();
        let mut ch: char;
        s.push(first_char);

        while buf.has_remaining() {
            // We can return EoI::Incomplete here if we encounter an incomplete
            // character.
            ch = self.next_char(buf)?;
            match ch {
                ' ' | '\t' | '\r' => return Ok(s),
                ',' => {
                    if self.nesting.is_empty() {
                        return Err(self.err_in_buf(ParseError::UnexpectedChar(ch)));
                    } else {
                        return Ok(s);
                    }
                }
                '\n' => {
                    // We want to process this character again to start counting
                    // newlines for linespaces.
                    self.maybe_peek = Some(ch);
                    return Ok(s);
                }
                '0'..='9' | 'a'..='z' | '-' | ':' | '.' => {
                    s.push(ch);
                }
                _ => return Err(self.err_in_buf(ParseError::UnexpectedChar(ch))),
            }
        }
        if self.nesting.is_empty() {
            Ok(s)
        } else {
            Err(EoI::Incomplete)
        }
    }

    fn parse_number_or_date(&mut self, first_char: char, buf: &mut Bytes) -> Result<Event, EoI> {
        match first_char {
            '-' | '+' => self.parse_number(first_char, buf),
            _ => {
                let value = self.parse_non_string_simple_value(first_char, buf)?;
                if value.contains(':') {
                    let date_time = parse_date_time(&value).map_err(|e| self.err_at_buf(e))?;
                    Ok(Event::SimpleValue(SimpleValue::DateTime(date_time)))
                } else if value.contains('-') {
                    let date = parse_date(&value).map_err(|e| self.err_at_buf(e))?;
                    Ok(Event::SimpleValue(SimpleValue::Date(date)))
                } else {
                    let num = parse_number(&value).map_err(|e| self.err_at_buf(e))?;
                    Ok(Event::SimpleValue(SimpleValue::Number(num)))
                }
            }
        }
    }

    fn parse_number(&mut self, first_char: char, buf: &mut Bytes) -> Result<Event, EoI> {
        let value = self.parse_non_string_simple_value(first_char, buf)?;
        let num = parse_number(&value).map_err(|e| self.err_at_buf(e))?;
        Ok(Event::SimpleValue(SimpleValue::Number(num)))
    }

    fn parse_bool(&mut self, first_char: char, buf: &mut Bytes) -> Result<Event, EoI> {
        let value = self.parse_non_string_simple_value(first_char, buf)?;
        match value.as_str() {
            "true" => Ok(Event::SimpleValue(SimpleValue::Boolean(true))),
            "false" => Ok(Event::SimpleValue(SimpleValue::Boolean(false))),
            _ => Err(self.err_at_buf(ParseError::InvalidIdentifier(value))),
        }
    }

    fn parse_null(&mut self, first_char: char, buf: &mut Bytes) -> Result<Event, EoI> {
        let value = self.parse_non_string_simple_value(first_char, buf)?;
        match value.as_str() {
            "null" => Ok(Event::SimpleValue(SimpleValue::Null)),
            _ => Err(self.err_at_buf(ParseError::InvalidIdentifier(value))),
        }
    }

    fn parse_string(&mut self, buf: &mut Bytes) -> Result<Event, EoI> {
        let mut value = String::new();
        let mut ch: char;

        while buf.has_remaining() {
            ch = self.next_char(buf)?;
            match ch {
                '\\' => {
                    value.push(self.parse_escape_seq(buf)?);
                }
                '"' => return Ok(Event::SimpleValue(SimpleValue::String(value))),
                _ => value.push(ch),
            }
        }
        Err(EoI::Incomplete)
    }

    fn parse_escape_seq(&mut self, buf: &mut Bytes) -> Result<char, EoI> {
        let escape_type = self.next_char(buf)?;
        Ok(match escape_type {
            '"' => '"',
            '\'' => '\'',
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            '\\' => '\\',
            '0' => '\0',
            // TODO: Support \x and \u escape sequences
            _ => return Err(self.err_in_buf(ParseError::InvalidEscapeSequence(escape_type))),
        })
    }

    fn parse_string_literal(&mut self, buf: &mut Bytes) -> Result<Event, EoI> {
        let mut in_string = false;
        let mut ch: char;
        let mut value = String::new();
        let mut closing_tag = "\"#".to_string();

        while buf.has_remaining() {
            ch = self.next_char(buf)?;
            if in_string {
                value.push(ch);
                if value.ends_with(&closing_tag) {
                    return Ok(Event::SimpleValue(SimpleValue::String(
                        value.strip_suffix(&closing_tag).unwrap().to_string(),
                    )));
                }
            } else {
                match ch {
                    '#' => closing_tag.push('#'),
                    '"' => {
                        in_string = true;
                    }
                    _ => return Err(self.err_in_buf(ParseError::InvalidLiteralStringChar(ch))),
                }
            }
        }
        Err(EoI::Incomplete)
    }

    fn parse_pre_value_comment(&mut self, buf: &mut Bytes) -> Result<Event, EoI> {
        todo!()
    }

    fn parse_pre_propname_comment(&mut self, buf: &mut Bytes) -> Result<Event, EoI> {
        todo!()
    }

    fn parse_property_name(&mut self, first_char: char, buf: &mut Bytes) -> Result<Event, EoI> {
        let mut name = String::new();
        let mut ch: char;
        name.push(first_char);

        while buf.has_remaining() {
            ch = self.next_char(buf)?;
            match ch {
                // We only allow ASCII-based characters in property names at the
                // moment.
                'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => {
                    name.push(ch);
                }
                ' ' | '\t' => return Ok(Event::PropertyName(name)),
                _ => return Err(self.err_in_buf(ParseError::UnexpectedChar(ch))),
            }
        }
        Err(EoI::Incomplete)
    }
}

fn parse_date_time(s: &str) -> Result<OffsetDateTime, ParseError> {
    todo!()
}

fn parse_date(s: &str) -> Result<Date, ParseError> {
    todo!()
}

fn parse_number(s: &str) -> Result<Number, ParseError> {
    if s.starts_with("0x") {
        parse_hex(s.strip_prefix("0x").unwrap())
    } else if s.starts_with('-') {
        parse_signed(s)
    } else if s.contains('.') {
        parse_fixed(s)
    } else {
        parse_unsigned(s)
    }
}

#[inline]
fn parse_hex(s: &str) -> Result<Number, ParseError> {
    let value = u64::from_str_radix(s, 16).map_err(ParseError::InvalidHexNumber)?;
    Ok(Number::Unsigned(value))
}

#[inline]
fn parse_signed(s: &str) -> Result<Number, ParseError> {
    let value = s.parse::<i64>().map_err(ParseError::InvalidSignedNumber)?;
    Ok(Number::Signed(value))
}

#[inline]
fn parse_unsigned(s: &str) -> Result<Number, ParseError> {
    let value = s
        .parse::<u64>()
        .map_err(ParseError::InvalidUnsignedNumber)?;
    Ok(Number::Unsigned(value))
}

#[inline]
fn parse_fixed(s: &str) -> Result<Number, ParseError> {
    let value = Fixed::from_str(s).map_err(ParseError::InvalidFixedPointNumber)?;
    Ok(Number::Fixed(value))
}

#[cfg(test)]
mod test {
    use super::*;
    use lazy_static::lazy_static;

    lazy_static! {
        static ref SIMPLE_OBJECTS: Vec<(&'static str, Vec<Event>)> = vec![
            (
                "{ a true, b false, c 3 }",
                vec![
                    Event::Start(ComplexValue::Object),
                    Event::PropertyName("a".to_string()),
                    Event::SimpleValue(SimpleValue::Boolean(true)),
                    Event::PropertyName("b".to_string()),
                    Event::SimpleValue(SimpleValue::Boolean(false)),
                    Event::PropertyName("c".to_string()),
                    Event::SimpleValue(SimpleValue::Number(Number::Unsigned(3))),
                    Event::End(ComplexValue::Object),
                ],
            ),
            (
                "{ a null, b true, c false, d null }",
                vec![
                    Event::Start(ComplexValue::Object),
                    Event::PropertyName("a".to_string()),
                    Event::SimpleValue(SimpleValue::Null),
                    Event::PropertyName("b".to_string()),
                    Event::SimpleValue(SimpleValue::Boolean(true)),
                    Event::PropertyName("c".to_string()),
                    Event::SimpleValue(SimpleValue::Boolean(false)),
                    Event::PropertyName("d".to_string()),
                    Event::SimpleValue(SimpleValue::Null),
                    Event::End(ComplexValue::Object),
                ],
            ),
            (
                r#"{ a "Hello", b "world" }"#,
                vec![
                    Event::Start(ComplexValue::Object),
                    Event::PropertyName("a".to_string()),
                    Event::SimpleValue(SimpleValue::String("Hello".to_string())),
                    Event::PropertyName("b".to_string()),
                    Event::SimpleValue(SimpleValue::String("world".to_string())),
                    Event::End(ComplexValue::Object),
                ],
            ),
        ];
        static ref STRINGS: Vec<(&'static str, Vec<Event>)> = vec![
            (
                r#""Test string""#,
                vec![Event::SimpleValue(SimpleValue::String(
                    "Test string".to_string()
                ))],
            ),
            (
                r#""Multi-line
string""#,
                vec![Event::SimpleValue(SimpleValue::String(
                    "Multi-line\nstring".to_string()
                ))],
            ),
            (
                r##"#"String literal"#"##,
                vec![Event::SimpleValue(SimpleValue::String(
                    "String literal".to_string()
                ))],
            ),
            (
                r###"##"String\nliteral"##"###,
                vec![Event::SimpleValue(SimpleValue::String(
                    "String\\nliteral".to_string()
                ))],
            )
        ];
    }

    #[test]
    fn identifier_parsing() {
        const TEST_CASES: &[(&str, SimpleValue)] = &[
            ("null", SimpleValue::Null),
            ("true", SimpleValue::Boolean(true)),
            ("false", SimpleValue::Boolean(false)),
        ];
        for (test_case, v) in TEST_CASES.iter() {
            let mut parser = Utf8Parser::default();
            let mut b = Bytes::from(*test_case);
            let ev = parser.next(&mut b).unwrap();
            assert_eq!(ev, Event::SimpleValue(v.clone()));
        }
    }

    #[test]
    fn simple_objects() {
        for (i, (test_case, events)) in SIMPLE_OBJECTS.iter().enumerate() {
            let mut parser = Utf8Parser::default();
            let mut b = Bytes::copy_from_slice(test_case.as_bytes());
            for (j, expected) in events.iter().enumerate() {
                let actual = parser.next(&mut b).unwrap();
                assert_eq!(actual, *expected, "test case {}, event {}", i, j);
            }
        }
    }

    #[test]
    fn strings() {
        for (i, (test_case, events)) in STRINGS.iter().enumerate() {
            let mut parser = Utf8Parser::default();
            let mut b = Bytes::copy_from_slice(test_case.as_bytes());
            for (j, expected) in events.iter().enumerate() {
                let actual = parser.next(&mut b).unwrap();
                assert_eq!(actual, *expected, "test case {}, event {}", i, j);
            }
        }
    }
}