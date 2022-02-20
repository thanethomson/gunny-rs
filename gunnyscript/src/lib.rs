//! GunnyScript aims to be a simple, human-readable data interchange format.
//! Similar to JSON/YAML, but more concise.

#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

pub mod encoding;
mod error;
mod number;
pub mod parser;
mod prelude;
mod value;

pub use error::{EncodingError, Error, ParseError};
pub use number::{Fixed, Number};
pub use value::Value;
