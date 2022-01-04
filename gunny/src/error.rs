use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to load view {0}: {1}")]
    ViewLoad(String, String),
    #[error("failed to load view from file: {0}")]
    FailedToLoadView(PathBuf),
    #[error("failed to obtain the data selection glob (via `select()`) from view {0}: {1}")]
    ViewSelect(String, String),
    #[error("failed to obtain template file name (via `template()`) from view {0}: {1}")]
    ViewTemplateName(String, String),
    #[error("failed to obtain output pattern (via `outputPattern()`) from view {0}: {1}")]
    ViewOutputPattern(String, String),
    #[error("failed to load template from file: {0}")]
    FailedToLoadTemplate(PathBuf),
    #[error("failed to convert JSON to JavaScript: {0}")]
    JsonToJavaScript(String),
    #[error("failed to execute JavaScript method \"{0}\": {1}")]
    JavaScript(String, String),
    #[error("duplicate view with name \"{0}\"")]
    ViewAlreadyExists(String),
    #[error("duplicate template with name \"{0}\"")]
    TemplateAlreadyExists(String),
    #[error("no such view with name \"{0}\"")]
    NoSuchView(String),
    #[error("cannot determine data file type: {0}")]
    CannotDetermineDataFileType(PathBuf),
    #[error("unsupported file extension for data file: {0}")]
    UnsupportedDataExtension(String),
    #[error("cannot determine file name as ID from data file: {0}")]
    CannotObtainDataId(PathBuf),
    #[error("expected data to be an object, but was not: {0}")]
    ExpectedDataToBeObject(PathBuf),
    #[error("invalid Markdown front matter: {0}")]
    InvalidMarkdownFrontMatter(String),
    #[error("unexpected return value from function {0}: {1}")]
    UnexpectedJavaScriptReturnValue(String, String),
    #[error("no views found")]
    NoViewsFound,
    #[error("unsupported configuration file format \"{0}\"")]
    UnsupportedConfigFileFormat(String),
    #[error("invalid configuration file: {0}")]
    InvalidConfig(String),
    #[error("failed to load configuration file from path: {0}")]
    FailedToLoadConfig(PathBuf),
    #[error("path must have a parent directory: {0}")]
    PathMissingParent(PathBuf),
}
