//! Rust equivalents of Gunnyscript values.

use alloc::vec::Vec;

pub type MultiLineString<'a> = Vec<&'a str>;

pub struct Document<'a> {
    pub docstring: MultiLineString<'a>,
    pub value: Value<'a>,
}

pub enum Value<'a> {
    Null,
    Bool(bool),
    Number(Number),
    String(ValueString<'a>),
    Date(Date),
    DateTime(DateTime),
    Array(Vec<Value<'a>>),
    Object(DocumentedProperties<'a>),
}

pub enum Number {
    Float(f64),
    Unsigned(u64),
    Signed(i64),
}

pub enum ValueString<'a> {
    Regular(MaybeLiteralString<'a>),
    Dedent(MaybeLiteralString<'a>),
}

pub enum MaybeLiteralString<'a> {
    NonLiteral(MultiLineString<'a>),
    Literal(MultiLineString<'a>),
}

pub struct Date {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

pub struct DateTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub min: u8,
    pub sec: u8,
    pub nanos: u64,
    pub offset_positive: bool,
    pub offset_hours: u8,
    pub offset_mins: u8,
}

pub type DocumentedProperties<'a> = Vec<DocumentedProperty<'a>>;

pub struct DocumentedProperty<'a> {
    pub docstring: MultiLineString<'a>,
    pub id: &'a str,
    pub value: Value<'a>,
}
