use bytes::Buf;
use time::{Date, OffsetDateTime};

use crate::encoding::Decoder;
use crate::parser::ComplexValue;
use crate::parser::Event;
use crate::parser::IterableParser;
use crate::parser::SimpleValue;
use crate::parser::Utf8IterableParser;
use crate::prelude::*;
use crate::Error;
use crate::Number;
use crate::ParseError;

/// A value of a particular type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Null,
    Boolean(bool),
    Number(Number),
    String(String),
    Date(Date),
    DateTime(OffsetDateTime),
    Array(Vec<Value>),
    Object(BTreeMap<String, DocValue>),
}

impl Default for Value {
    fn default() -> Self {
        Self::Null
    }
}

impl From<SimpleValue> for Value {
    fn from(v: SimpleValue) -> Self {
        match v {
            SimpleValue::Null => Self::Null,
            SimpleValue::Boolean(b) => Self::Boolean(b),
            SimpleValue::String(s) => Self::String(s),
            SimpleValue::Number(n) => Self::Number(n),
            SimpleValue::Date(d) => Self::Date(d),
            SimpleValue::DateTime(dt) => Self::DateTime(dt),
        }
    }
}

/// A value that optionally has some documentation associated with it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DocValue(Option<String>, Value);

impl DocValue {
    /// Constructor.
    pub fn new(doc: &str, value: Value) -> Self {
        Self(Some(doc.to_string()), value)
    }

    /// Optionally return the documentation associated with this value.
    pub fn doc(&self) -> Option<&str> {
        self.0.as_deref()
    }

    /// Return a reference to the value associated with this documented value.
    pub fn value(&self) -> &Value {
        &self.1
    }

    pub fn parse_utf8<B: Buf>(buf: &mut B) -> Result<Self, Error> {
        let mut parser = Utf8IterableParser::from(buf);
        Self::from_parser(&mut parser)
    }

    fn from_parser<'buf, D: Decoder, B: Buf>(
        parser: &'buf mut IterableParser<'buf, D, B>,
    ) -> Result<Self, Error> {
        let mut doc_comment_lines = Vec::new();
        loop {
            let event = parser.next().ok_or(Error::UnexpectedEof)??;
            let maybe_value = match event {
                Event::DocCommentLine(line) => {
                    doc_comment_lines.push(line);
                    None
                }
                Event::SimpleValue(v) => Some(v.into()),
                Event::Start(ComplexValue::Array) => Some(Self::parse_array(parser)?),
                Event::Start(ComplexValue::Object) => Some(Self::parse_object(parser)?),
                _ => return Err(Error::Parse(ParseError::UnexpectedItem)),
            };
            if let Some(value) = maybe_value {
                let maybe_doc_comment = if doc_comment_lines.is_empty() {
                    None
                } else {
                    Some(doc_comment_lines.join(""))
                };
                return Ok(Self(maybe_doc_comment, value));
            }
        }
    }

    fn parse_array<'buf, D: Decoder, B: Buf>(
        parser: &'buf mut IterableParser<'buf, D, B>,
    ) -> Result<Value, Error> {
        let mut values = Vec::new();
        loop {
            let event = parser.next().ok_or(Error::UnexpectedEof)??;
            match event {
                Event::SimpleValue(v) => values.push(v.into()),
                Event::Start(ComplexValue::Array) => values.push(Self::parse_array(parser)?),
                Event::Start(ComplexValue::Object) => values.push(Self::parse_object(parser)?),
                Event::End(ComplexValue::Array) => return Ok(Value::Array(values)),
                _ => return Err(Error::Parse(ParseError::UnexpectedItem)),
            }
        }
    }

    fn parse_object<'buf, D: Decoder, B: Buf>(
        parser: &'buf mut IterableParser<'buf, D, B>,
    ) -> Result<Value, Error> {
        let mut map = BTreeMap::new();
        let mut maybe_prop_name: Option<String> = None;
        let mut prop_dc = Vec::new();
        loop {
            let event = parser.next().ok_or(Error::UnexpectedEof)??;
            match event {
                Event::DocCommentLine(line) => prop_dc.push(line),
                Event::PropertyName(name) => maybe_prop_name = Some(name),
                Event::SimpleValue(v) => {
                    if let Some(prop_name) = maybe_prop_name.take() {
                        if map.contains_key(&prop_name) {
                            return Err(Error::Parse(ParseError::DuplicatePropertyName(prop_name)));
                        }
                        let maybe_dc = if prop_dc.is_empty() {
                            None
                        } else {
                            Some(prop_dc.join(""))
                        };
                        map.insert(prop_name, Self(maybe_dc, v.into()));
                        prop_dc.clear();
                    } else {
                        return Err(Error::Parse(ParseError::UnexpectedItem));
                    }
                }
                Event::End(ComplexValue::Object) => return Ok(Value::Object(map)),
                _ => return Err(Error::Parse(ParseError::UnexpectedItem)),
            }
        }
    }
}

impl From<SimpleValue> for DocValue {
    fn from(v: SimpleValue) -> Self {
        Self(None, v.into())
    }
}
