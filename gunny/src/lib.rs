//! Gunny aims to be a utility that transforms static content and data into
//! static output files.
//!
//! This crate provides an API that allows for embedding Gunny into another
//! application. For Gunny's command line interface, see the `gunny-cli` crate.

mod collection;
mod datetime;
mod error;
mod project;
mod source;
mod template;
mod transform;
mod value;
mod view;

pub use collection::Collection;
pub use datetime::{Date, DateTime};
pub use error::Error;
pub use project::Project;
pub use source::{Source, SourceIter};
pub use template::Templates;
pub use transform::Transform;
pub use value::{Fixed, Map, Value, ValueType};
pub use view::View;
