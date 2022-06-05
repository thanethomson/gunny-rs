//! Gunnyscript is a simple, strictly structured markup language that supports
//! everything that JSON does, but also:
//!
//! - Built-in date and date/time support
//! - Docstrings for capturing descriptions of values that can be made accessible
//!   to users after processing
//! - Comments for people reading the markup itself
//!
//! This crate provides a parser for Gunnyscript.

#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

mod error;
mod parser;
mod value;

pub use error::{located_err, Error, Located};
pub use parser::{Lexer, SimpleValue, Token};
pub use value::{
    Date, DateTime, Document, DocumentedProperties, DocumentedProperty, MaybeLiteralString,
    MultiLineString, Number, Value, ValueString,
};
