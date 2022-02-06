//! Data source handling.

use glob::Paths;

use crate::{Error, Value};

/// An iterator producing elements of type `Result<Value, Error>` that are read
/// from a particular data source.
pub enum SourceIter {
    Files { paths: Paths },
}

impl Iterator for SourceIter {
    type Item = Result<Value, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            SourceIter::Files { paths } => {
                let next_path = match paths.next()? {
                    Ok(p) => p,
                    Err(e) => return Some(Err(Error::SourceIter(e))),
                };
                let result = Value::load_from_file(&next_path);
                Some(result)
            }
        }
    }
}

/// A source of data that can be transformed prior to insertion into a
/// collection.
#[derive(Debug)]
pub enum Source {
    /// One or more files from the local file system.
    ///
    /// The parameter can specify a glob-style pattern for matching files.
    Files(String),
}

impl Source {
    /// Returns an iterator that allows one to iterate through values parsed
    /// from the source as they are read.
    ///
    /// Can fail if this source has been incorrectly configured.
    pub fn iter(&self) -> Result<SourceIter, Error> {
        Ok(match self {
            Self::Files(pattern) => glob::glob(&pattern)
                .map(|paths| SourceIter::Files { paths })
                .map_err(|e| Error::SourceFilePattern(pattern.clone(), e))?,
        })
    }
}
