//! Parsing functionality for GunnyScript.

use core::marker::PhantomData;

use bytes::Buf;
use time::{format_description::well_known::Rfc3339, Date, Month, OffsetDateTime};

use crate::{
    encoding::{Decoder, Utf8Decoder},
    prelude::*,
    Error, LocatedParseError, Number, ParseError,
};

#[derive(Debug, Clone, Copy)]
pub struct Location {
    pub line: usize,
    pub column: usize,
    pub bytes: usize,
}

impl Default for Location {
    fn default() -> Self {
        // Prefer human-readable locations.
        Self {
            line: 1,
            column: 1,
            bytes: 0,
        }
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
    Error(LocatedParseError),
    Incomplete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Linespace,
    DocCommentLine(String),
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
    just_parsed_doc_comment: bool,
    _decoder: PhantomData<D>,
}

impl<D: Decoder> Parser<D> {
    pub fn location(&self) -> Location {
        self.location
    }

    // Error at a specific location within the buffer.
    #[inline]
    fn err_in_buf(&self, e: ParseError) -> EoI {
        EoI::Error(LocatedParseError::new(
            self.buf_location.line,
            self.buf_location.column,
            e,
        ))
    }

    // Error at the beginning of the buffer.
    #[inline]
    fn err_at_buf(&self, e: ParseError) -> EoI {
        EoI::Error(LocatedParseError::new(
            self.location.line,
            self.location.column,
            e,
        ))
    }

    fn next_char<B: Buf>(&mut self, buf: &mut B) -> Result<char, EoI> {
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

    pub fn next<B: Buf>(&mut self, buf: &mut B) -> Result<Event, EoI> {
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
                    if self.just_parsed_doc_comment {
                        return Err(self.err_in_buf(ParseError::DanglingDocComment));
                    }
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
                    '/' => self.parse_pre_value_comment(buf)?,
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
                    '/' => self.parse_pre_propname_comment(buf)?,
                    ',' => None,
                    _ => return Err(self.err_in_buf(ParseError::InvalidPropertyNameChar(ch))),
                },
            };

            match maybe_ev {
                // If we found a token, and it indicates the end of a value
                // (either simple or complex)...
                Some(Event::SimpleValue(_)) | Some(Event::End(_)) => {
                    // ... and we're inside an object ...
                    if let Some(ComplexValue::Object) = self.nesting.last() {
                        // ... we now expect a property name to follow.
                        self.state = State::ExpectingPropertyName;
                    }
                    self.just_parsed_doc_comment = false;
                }
                Some(Event::DocCommentLine(_)) => {
                    self.just_parsed_doc_comment = true;
                }
                _ => {
                    self.just_parsed_doc_comment = false;
                }
            }

            // By this point we've successfully parsed an optional token.
            self.location = self.buf_location;
        }
        Ok(maybe_ev.unwrap())
    }

    pub fn expecting_more(&self) -> bool {
        !self.nesting.is_empty()
    }

    fn consume_until_not<B: Buf>(
        &mut self,
        buf: &mut B,
        keep_matching: &[char],
    ) -> Result<(), EoI> {
        while buf.has_remaining() {
            let ch = self.next_char(buf)?;
            if !keep_matching.contains(&ch) {
                self.maybe_peek = Some(ch);
                return Ok(());
            }
        }
        Err(EoI::Incomplete)
    }

    fn skip_whitespace<B: Buf>(&mut self, buf: &mut B) -> Result<(), EoI> {
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

    fn parse_non_string_simple_value<B: Buf>(
        &mut self,
        first_char: char,
        buf: &mut B,
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
                '0'..='9' | 'a'..='z' | 'A'..='Z' | '-' | ':' | '.' => {
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

    fn parse_number_or_date<B: Buf>(
        &mut self,
        first_char: char,
        buf: &mut B,
    ) -> Result<Event, EoI> {
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
                    let num = Number::from_str(&value).map_err(|e| self.err_at_buf(e))?;
                    Ok(Event::SimpleValue(SimpleValue::Number(num)))
                }
            }
        }
    }

    fn parse_number<B: Buf>(&mut self, first_char: char, buf: &mut B) -> Result<Event, EoI> {
        let value = self.parse_non_string_simple_value(first_char, buf)?;
        let num = Number::from_str(&value).map_err(|e| self.err_at_buf(e))?;
        Ok(Event::SimpleValue(SimpleValue::Number(num)))
    }

    fn parse_bool<B: Buf>(&mut self, first_char: char, buf: &mut B) -> Result<Event, EoI> {
        let value = self.parse_non_string_simple_value(first_char, buf)?;
        match value.as_str() {
            "true" => Ok(Event::SimpleValue(SimpleValue::Boolean(true))),
            "false" => Ok(Event::SimpleValue(SimpleValue::Boolean(false))),
            _ => Err(self.err_at_buf(ParseError::InvalidIdentifier(value))),
        }
    }

    fn parse_null<B: Buf>(&mut self, first_char: char, buf: &mut B) -> Result<Event, EoI> {
        let value = self.parse_non_string_simple_value(first_char, buf)?;
        match value.as_str() {
            "null" => Ok(Event::SimpleValue(SimpleValue::Null)),
            _ => Err(self.err_at_buf(ParseError::InvalidIdentifier(value))),
        }
    }

    fn parse_string<B: Buf>(&mut self, buf: &mut B) -> Result<Event, EoI> {
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

    fn parse_escape_seq<B: Buf>(&mut self, buf: &mut B) -> Result<char, EoI> {
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

    fn parse_string_literal<B: Buf>(&mut self, buf: &mut B) -> Result<Event, EoI> {
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

    fn parse_pre_value_comment<B: Buf>(&mut self, buf: &mut B) -> Result<Option<Event>, EoI> {
        let ch1 = self.next_char(buf)?;
        match ch1 {
            '/' => {
                let ch2 = self.next_char(buf)?;
                match ch2 {
                    '/' => {
                        if self.nesting.is_empty() {
                            Ok(Some(self.parse_doc_comment_line(buf)?))
                        } else {
                            Err(self.err_at_buf(ParseError::UnexpectedDocComment))
                        }
                    }
                    _ => self.parse_single_line_comment(ch2, buf),
                }
            }
            '*' => self.parse_multiline_comment(buf),
            _ => Err(self.err_in_buf(ParseError::InvalidCommentDelimiter(ch1))),
        }
    }

    fn parse_doc_comment_line<B: Buf>(&mut self, buf: &mut B) -> Result<Event, EoI> {
        let mut ch: char;
        let mut value = String::new();

        while buf.has_remaining() {
            ch = self.next_char(buf)?;
            value.push(ch);
            if ch == '\n' {
                self.maybe_peek = Some(ch);
                return Ok(Event::DocCommentLine(value));
            }
        }
        Err(EoI::Incomplete)
    }

    fn parse_single_line_comment<B: Buf>(
        &mut self,
        first_char: char,
        buf: &mut B,
    ) -> Result<Option<Event>, EoI> {
        if first_char == '\n' {
            return Ok(None);
        }
        let mut ch: char;
        while buf.has_remaining() {
            ch = self.next_char(buf)?;
            if ch == '\n' {
                self.maybe_peek = Some(ch);
                return Ok(None);
            }
        }
        Err(EoI::Incomplete)
    }

    fn parse_multiline_comment<B: Buf>(&mut self, buf: &mut B) -> Result<Option<Event>, EoI> {
        let mut ch: char;
        let mut lookahead = ['\0'; 2];
        while buf.has_remaining() {
            ch = self.next_char(buf)?;
            lookahead[0] = lookahead[1];
            lookahead[1] = ch;
            if lookahead[0] == '*' && lookahead[1] == '/' {
                return Ok(None);
            }
        }
        Err(EoI::Incomplete)
    }

    fn parse_pre_propname_comment<B: Buf>(&mut self, buf: &mut B) -> Result<Option<Event>, EoI> {
        let ch1 = self.next_char(buf)?;
        match ch1 {
            '/' => {
                let ch2 = self.next_char(buf)?;
                match ch2 {
                    '/' => Ok(Some(self.parse_doc_comment_line(buf)?)),
                    _ => self.parse_single_line_comment(ch2, buf),
                }
            }
            '*' => self.parse_multiline_comment(buf),
            _ => Err(self.err_in_buf(ParseError::InvalidCommentDelimiter(ch1))),
        }
    }

    fn parse_property_name<B: Buf>(&mut self, first_char: char, buf: &mut B) -> Result<Event, EoI> {
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
    let datetime = OffsetDateTime::parse(s, &Rfc3339).map_err(ParseError::InvalidDateTime)?;
    Ok(datetime)
}

fn parse_date(s: &str) -> Result<Date, ParseError> {
    let mut ymd = s.split('-');
    let year = ymd
        .next()
        .ok_or(ParseError::MissingYearInDate)?
        .parse::<i32>()
        .map_err(ParseError::InvalidDateYear)?;
    let month = ymd
        .next()
        .ok_or(ParseError::MissingMonthInDate)?
        .trim_start_matches('0')
        .parse::<u8>()
        .map_err(ParseError::InvalidDateMonth)?;
    let day = ymd
        .next()
        .ok_or(ParseError::MissingDayInDate)?
        .trim_start_matches('0')
        .parse::<u8>()
        .map_err(ParseError::InvalidDateDay)?;
    Date::from_calendar_date(
        year,
        Month::try_from(month).map_err(ParseError::InvalidDate)?,
        day,
    )
    .map_err(ParseError::InvalidDate)
}

/// An owned iterable parser that parses events from UTF-8 data.
pub type Utf8IterableParser<'buf, B> = IterableParser<'buf, Utf8Decoder, B>;

/// A parser that maintains its own reference to its buffer, such that it can
/// facilitate iteration.
pub struct IterableParser<'buf, D, B> {
    parser: Parser<D>,
    buf: &'buf mut B,
}

impl<'buf, D: Decoder, B: Buf> IterableParser<'buf, D, B> {
    #[inline]
    pub fn location(&self) -> Location {
        self.parser.location()
    }

    #[inline]
    pub fn expecting_more(&self) -> bool {
        self.parser.expecting_more()
    }
}

impl<'buf, D: Decoder + Default, B: Buf> From<&'buf mut B> for IterableParser<'buf, D, B> {
    fn from(buf: &'buf mut B) -> Self {
        Self {
            parser: Parser::default(),
            buf,
        }
    }
}

impl<'buf, D: Decoder, B: Buf> Iterator for IterableParser<'buf, D, B> {
    type Item = Result<Event, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.parser.next(self.buf) {
            Ok(event) => Some(Ok(event)),
            Err(EoI::Incomplete) => None,
            Err(EoI::Error(e)) => Some(Err(Error::LocatedParse(e))),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bytes::Bytes;
    use fixed_macro::fixed;
    use lazy_static::lazy_static;
    use time::macros::{date, datetime};

    lazy_static! {
        static ref IDENTIFIERS: Vec<(&'static str, Vec<Event>)> = vec![
            ("null", vec![Event::SimpleValue(SimpleValue::Null)],),
            ("true", vec![Event::SimpleValue(SimpleValue::Boolean(true))],),
            (
                "false",
                vec![Event::SimpleValue(SimpleValue::Boolean(false))],
            ),
        ];
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
            (
                r#"{ a "Hello", b { c true } }"#,
                vec![
                    Event::Start(ComplexValue::Object),
                    Event::PropertyName("a".to_string()),
                    Event::SimpleValue(SimpleValue::String("Hello".to_string())),
                    Event::PropertyName("b".to_string()),
                    Event::Start(ComplexValue::Object),
                    Event::PropertyName("c".to_string()),
                    Event::SimpleValue(SimpleValue::Boolean(true)),
                    Event::End(ComplexValue::Object),
                    Event::End(ComplexValue::Object),
                ],
            )
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
        static ref COMMENTS: Vec<(&'static str, Vec<Event>)> = vec![
            (
                r#"// Single-line comment
// across multiple lines.
/* Followed by a
 * multi-line
 * comment. */
/// Followed by a doc comment.
"Hello""#,
                vec![
                    Event::DocCommentLine(" Followed by a doc comment.\n".to_string()),
                    Event::SimpleValue(SimpleValue::String("Hello".to_string())),
                ]
            ),
            (
                r#"/// Doc comment.

"Hello""#,
                vec![
                    Event::DocCommentLine(" Doc comment.\n".to_string()),
                    Event::Linespace,
                    Event::SimpleValue(SimpleValue::String("Hello".to_string())),
                ]
            ),
            (
                r#"{
    /// Something about a.
    a 123
}"#,
                vec![
                    Event::Start(ComplexValue::Object),
                    Event::DocCommentLine(" Something about a.\n".to_string()),
                    Event::PropertyName("a".to_string()),
                    Event::SimpleValue(SimpleValue::Number(Number::Unsigned(123))),
                    Event::End(ComplexValue::Object),
                ],
            ),
        ];
        static ref NUMBERS: Vec<(&'static str, Vec<Event>)> = vec![(
            r#"{
    a 1
    b -1
    c 3.14159
    d -3.14159
    e 0xDEADBEEF
    f 0755
}"#,
            vec![
                Event::Start(ComplexValue::Object),
                Event::PropertyName("a".to_string()),
                Event::SimpleValue(SimpleValue::Number(Number::Unsigned(1))),
                Event::PropertyName("b".to_string()),
                Event::SimpleValue(SimpleValue::Number(Number::Signed(-1))),
                Event::PropertyName("c".to_string()),
                Event::SimpleValue(SimpleValue::Number(Number::Fixed(fixed!(3.14159: I64F64)))),
                Event::PropertyName("d".to_string()),
                Event::SimpleValue(SimpleValue::Number(Number::Fixed(fixed!(-3.14159: I64F64)))),
                Event::PropertyName("e".to_string()),
                Event::SimpleValue(SimpleValue::Number(Number::Unsigned(0xDEADBEEF))),
                Event::PropertyName("f".to_string()),
                Event::SimpleValue(SimpleValue::Number(Number::Unsigned(493))),
                Event::End(ComplexValue::Object),
            ]
        ),];
        static ref DATES: Vec<(&'static str, Vec<Event>)> = vec![
            (
                "2020-01-02",
                vec![Event::SimpleValue(SimpleValue::Date(date!(2020 - 01 - 02)))],
            ),
            (
                "2020-01-02T12:54:00Z",
                vec![Event::SimpleValue(SimpleValue::DateTime(
                    datetime!(2020-01-02 12:54 UTC)
                ))],
            ),
            (
                "{ a 2020-01-02T12:54:00-05:00 }",
                vec![
                    Event::Start(ComplexValue::Object),
                    Event::PropertyName("a".to_string()),
                    Event::SimpleValue(SimpleValue::DateTime(datetime!(2020-01-02 12:54 -05:00))),
                    Event::End(ComplexValue::Object),
                ]
            )
        ];
        static ref ARRAYS: Vec<(&'static str, Vec<Event>)> = vec![
            (
                "[ 1, 2, 3 ]",
                vec![
                    Event::Start(ComplexValue::Array),
                    Event::SimpleValue(SimpleValue::Number(Number::Unsigned(1))),
                    Event::SimpleValue(SimpleValue::Number(Number::Unsigned(2))),
                    Event::SimpleValue(SimpleValue::Number(Number::Unsigned(3))),
                    Event::End(ComplexValue::Array),
                ]
            ),
            (
                "{ a [ 1, 2, 3 ], b 4 }",
                vec![
                    Event::Start(ComplexValue::Object),
                    Event::PropertyName("a".to_string()),
                    Event::Start(ComplexValue::Array),
                    Event::SimpleValue(SimpleValue::Number(Number::Unsigned(1))),
                    Event::SimpleValue(SimpleValue::Number(Number::Unsigned(2))),
                    Event::SimpleValue(SimpleValue::Number(Number::Unsigned(3))),
                    Event::End(ComplexValue::Array),
                    Event::PropertyName("b".to_string()),
                    Event::SimpleValue(SimpleValue::Number(Number::Unsigned(4))),
                    Event::End(ComplexValue::Object),
                ],
            ),
        ];
    }

    #[test]
    fn identifiers() {
        parse_tests(&IDENTIFIERS);
    }

    #[test]
    fn simple_objects() {
        parse_tests(&SIMPLE_OBJECTS);
    }

    #[test]
    fn strings() {
        parse_tests(&STRINGS);
    }

    #[test]
    fn comments() {
        parse_tests(&COMMENTS);
    }

    #[test]
    fn numbers() {
        parse_tests(&NUMBERS);
    }

    #[test]
    fn dates() {
        parse_tests(&DATES);
    }

    #[test]
    fn arrays() {
        parse_tests(&ARRAYS);
    }

    fn parse_tests(test_cases: &[(&'static str, Vec<Event>)]) {
        for (i, (test_case, events)) in test_cases.iter().enumerate() {
            let mut parser = Utf8Parser::default();
            let mut b = Bytes::from_static(test_case.as_bytes());
            for (j, expected) in events.iter().enumerate() {
                let actual = parser.next(&mut b).unwrap();
                assert_eq!(actual, *expected, "test case {}, event {}", i, j);
            }
        }
    }
}
