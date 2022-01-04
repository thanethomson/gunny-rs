//! This crate provides an API to access Gunny's functionality as a library. The
//! `gunny-cli` crate provides its command line interface.

mod context;
pub(crate) mod data;
mod error;
pub(crate) mod fs;
pub(crate) mod hash;
pub(crate) mod js;
pub(crate) mod template;
mod view;

pub use context::Context;
pub use data::Value;
pub use error::Error;
pub use view::{PartialView, View};
