//! This crate provides an API to access Gunny's functionality as a library. The
//! `gunny-cli` crate provides its command line interface.

mod config;
mod context;
pub(crate) mod data;
mod error;
pub(crate) mod hash;
pub(crate) mod js;
mod view;

pub use config::Config;
pub use context::Context;
pub use error::Error;
pub use view::{PartialView, View};
