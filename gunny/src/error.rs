use std::path::PathBuf;

use thiserror::Error;

use crate::ValueType;

/// The primary error type that can be produced by Gunny.
#[derive(Debug, Error)]
pub enum Error {
    #[error("the \"Unknown\" type is for internal use only and cannot be used to specify schemas")]
    UnknownValueType,
    #[error("object property names must be strings")]
    ObjectKeysMustBeStrings,
    #[error("I/O error {0}: {1}")]
    Io(String, std::io::Error),
    #[error("failed to load data from file {0}: {1}")]
    LoadFromFile(PathBuf, Box<Error>),
    #[error("unsupported file type: {0}")]
    UnsupportedFileType(String),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("TOML serialization error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),
    #[error("TOML deserialization error: {0}")]
    TomlDeserialize(#[from] toml::de::Error),
    #[error("cannot determine file type of file: {0}")]
    CannotDetermineFileType(PathBuf),
    #[error("cannot extract file name from path: {0}")]
    CannotExtractFileName(PathBuf),
    #[error("source files iteration failed: {0}")]
    SourceIter(#[from] glob::GlobError),
    #[error("failed to parse source file pattern \"{0}\": {1}")]
    SourceFilePattern(String, glob::PatternError),
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("cannot parse type from value of type {0} - types can only be parsed from strings and objects")]
    CannotParseTypeFromValue(ValueType),
    #[error("failed to render template \"{0}\": {1}")]
    TemplateRender(String, handlebars::RenderError),
}
