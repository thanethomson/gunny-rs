//! File system-related utilities.

use std::path::{Path, PathBuf};

use eyre::Result;

/// Canonicalize the given path if it exists. If it does not exist, returns
/// `Ok(None)`.
pub fn maybe_canonicalize<P>(path: P) -> Result<Option<PathBuf>>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    if path.exists() {
        Ok(Some(path.canonicalize()?))
    } else {
        Ok(None)
    }
}
