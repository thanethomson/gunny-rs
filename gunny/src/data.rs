//! Data is read from the file system and is transformed prior to being rendered
//! through a template.

use std::{
    collections::{BTreeMap, HashMap},
    fs,
    path::Path,
};

use eyre::{Result, WrapErr};
use log::trace;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Number as JsonNumber, Value as JsonValue};
use serde_yaml::{Mapping, Number as YamlNumber, Value as YamlValue};

use crate::Error;

/// An intermediate dynamic type for conversion between different serialization
/// formats.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd)]
pub enum Value {
    Null,
    Boolean(bool),
    String(String),
    Number(Number),
    Array(Vec<Value>),
    Object(BTreeMap<String, Value>),
}

impl Value {
    /// Constructs an empty object.
    pub fn empty_object() -> Self {
        Self::Object(BTreeMap::new())
    }

    /// Load a value from the given file, automatically detecting its type
    /// before loading the value.
    ///
    /// The file type is determined by its extension. Currently supported file
    /// types include JSON (`.js`), YAML (`.yml` or `.yaml`) and Markdown
    /// (`.md`).
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let ext = path
            .extension()
            .ok_or_else(|| Error::CannotDetermineDataFileType(path.to_path_buf()))?
            .to_str()
            .unwrap();
        let id = path
            .file_stem()
            .ok_or_else(|| Error::CannotObtainDataId(path.to_path_buf()))?
            .to_str()
            .unwrap();
        let content = fs::read_to_string(path)?;
        let mut value = Self::from_str(ext, &content)
            .wrap_err_with(|| Error::CannotDetermineDataFileType(path.to_path_buf()))?;
        match value {
            Value::Object(ref mut obj) => {
                obj.insert("id".to_string(), Value::String(id.to_string()));
            }
            _ => return Err(Error::ExpectedDataToBeObject(path.to_path_buf()).into()),
        }
        trace!("Loaded data from {}\n{:#?}", path.display(), value);
        Ok(value)
    }

    pub fn from_str(format: &str, content: &str) -> Result<Self> {
        let value = match format {
            "json" => serde_json::from_str::<JsonValue>(&content)?.into(),
            "yml" | "yaml" => serde_yaml::from_str::<YamlValue>(&content)?.try_into()?,
            "md" => parse_markdown(&content)?,
            _ => return Err(Error::UnsupportedDataExtension(format.to_string()).into()),
        };
        Ok(value)
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(n) => n.as_f64(),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Number(n) => n.as_i64(),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Self::Number(n) => n.as_u64(),
            _ => None,
        }
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Self::Boolean(b)
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Self::Number(Number::Float(f))
    }
}

impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Self::Number(Number::Signed(i))
    }
}

impl From<u64> for Value {
    fn from(u: u64) -> Self {
        Self::Number(Number::Unsigned(u))
    }
}

impl<T> From<Vec<T>> for Value
where
    T: Into<Value>,
{
    fn from(v: Vec<T>) -> Self {
        Self::Array(v.into_iter().map(Into::into).collect())
    }
}

impl<T> From<HashMap<String, T>> for Value
where
    T: Into<Value>,
{
    fn from(m: HashMap<String, T>) -> Self {
        Self::Object(BTreeMap::from_iter(
            m.into_iter().map(|(k, v)| (k, v.into())),
        ))
    }
}

impl<T> From<BTreeMap<String, T>> for Value
where
    T: Into<Value>,
{
    fn from(m: BTreeMap<String, T>) -> Self {
        Self::Object(BTreeMap::from_iter(
            m.into_iter().map(|(k, v)| (k, v.into())),
        ))
    }
}

impl From<JsonValue> for Value {
    fn from(v: JsonValue) -> Self {
        match v {
            JsonValue::Null => Self::Null,
            JsonValue::Bool(b) => Self::Boolean(b),
            JsonValue::Number(n) => Self::Number(n.into()),
            JsonValue::String(s) => Self::String(s),
            JsonValue::Array(arr) => Self::Array(arr.iter().cloned().map(Into::into).collect()),
            JsonValue::Object(obj) => {
                let mut map = BTreeMap::new();
                for (k, v) in obj.iter() {
                    map.insert(k.clone(), v.clone().into());
                }
                Self::Object(map)
            }
        }
    }
}

impl From<Value> for JsonValue {
    fn from(v: Value) -> Self {
        match v {
            Value::Null => Self::Null,
            Value::Boolean(b) => Self::Bool(b),
            Value::String(s) => Self::String(s),
            Value::Number(n) => Self::Number(n.into()),
            Value::Array(v) => Self::Array(v.into_iter().map(Into::into).collect()),
            Value::Object(o) => {
                Self::Object(Map::from_iter(o.into_iter().map(|(k, v)| (k, v.into()))))
            }
        }
    }
}

impl TryFrom<YamlValue> for Value {
    type Error = eyre::Report;

    fn try_from(value: YamlValue) -> Result<Self, Self::Error> {
        Ok(match value {
            YamlValue::Null => Self::Null,
            YamlValue::Bool(b) => Self::Boolean(b),
            YamlValue::Number(n) => Self::Number(n.into()),
            YamlValue::String(s) => Self::String(s),
            YamlValue::Sequence(s) => Self::Array(
                s.into_iter()
                    .map(TryFrom::try_from)
                    .collect::<Result<Vec<Self>>>()?,
            ),
            YamlValue::Mapping(m) => Self::Object(BTreeMap::from_iter(
                m.into_iter()
                    .map(|(k, v)| {
                        let k = k.as_str()
                            .ok_or_else(|| Error::InvalidMarkdownFrontMatter(
                                    format!("YAML keys must be able to be interpreted as strings, but found {:?}", k)
                                )
                            )?.to_string();
                        Ok((k, Self::try_from(v)?))
                    })
                    .collect::<Result<Vec<(String, Self)>>>()?
                    .into_iter(),
            )),
        })
    }
}

impl From<Value> for YamlValue {
    fn from(v: Value) -> Self {
        match v {
            Value::Null => Self::Null,
            Value::Boolean(b) => Self::Bool(b),
            Value::String(s) => Self::String(s),
            Value::Number(n) => Self::Number(n.into()),
            Value::Array(a) => Self::Sequence(a.into_iter().map(Into::into).collect()),
            Value::Object(o) => Self::Mapping(Mapping::from_iter(
                o.into_iter().map(|(k, v)| (k.into(), v.into())),
            )),
        }
    }
}

fn split_front_matter(content: &str) -> (Option<&str>, &str) {
    const DELIMITERS: &[&str] = &["---\n", "---\r\n"];
    for delim in DELIMITERS {
        if content.starts_with(delim) {
            let parts = content
                .split(delim)
                .filter(|part| !part.is_empty())
                .collect::<Vec<&str>>();
            if parts.len() == 2 {
                return (Some(parts[0]), parts[1]);
            }
        }
    }
    (None, content)
}

// Given a Markdown file that looks as follows:
//
// ```
// ---
// title: The Title
// published: 2022-01-02
// ---
// Raw markdown content goes **here**.
// ```
//
// this method parses a Markdown file into an object whose JSON representation
// looks like:
//
// {
//   "title": "The Title",
//   "published": "2022-01-02",
//   "content": "Raw markdown content goes **here**",
// }
fn parse_markdown(content: &str) -> Result<Value> {
    let (maybe_front_matter, content) = split_front_matter(content);
    let mut obj = match maybe_front_matter {
        Some(front_matter) => serde_yaml::from_str::<YamlValue>(front_matter)?.try_into()?,
        None => Value::Object(BTreeMap::new()),
    };
    match obj {
        Value::Object(ref mut o) => {
            // TODO: Warn if "content" field is being overwritten.
            o.insert("content".to_string(), Value::String(content.to_string()));
        }
        _ => {
            return Err(Error::InvalidMarkdownFrontMatter(
                "front matter must be an object".to_string(),
            )
            .into())
        }
    }
    Ok(obj)
}

/// A floating point, signed or unsigned number.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialOrd)]
pub enum Number {
    Float(f64),
    Signed(i64),
    Unsigned(u64),
}

impl Number {
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Float(f) => Some(*f),
            Self::Signed(i) => f64::try_from(i32::try_from(*i).ok()?).ok(),
            Self::Unsigned(u) => f64::try_from(u32::try_from(*u).ok()?).ok(),
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Float(_) => None,
            Self::Signed(i) => Some(*i),
            Self::Unsigned(u) => i64::try_from(*u).ok(),
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Self::Float(_) => None,
            Self::Signed(i) => u64::try_from(*i).ok(),
            Self::Unsigned(u) => Some(*u),
        }
    }
}

impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Self::Float(f1) => match other {
                // TODO: Look into replacing the f64 with a fixed-point number
                // system, like from the `fixed` crate.
                Self::Float(f2) => f1 == f2,
                _ => false,
            },
            Self::Signed(i1) => match other {
                Self::Signed(i2) => i1 == i2,
                _ => false,
            },
            Self::Unsigned(u1) => match other {
                Self::Unsigned(u2) => u1 == u2,
                _ => false,
            },
        }
    }
}

impl From<JsonNumber> for Number {
    fn from(n: JsonNumber) -> Self {
        if n.is_f64() {
            Self::Float(n.as_f64().unwrap())
        } else if n.is_i64() {
            Self::Signed(n.as_i64().unwrap())
        } else {
            Self::Unsigned(n.as_u64().unwrap())
        }
    }
}

impl From<Number> for JsonNumber {
    fn from(n: Number) -> Self {
        match n {
            Number::Float(f) => Self::from_f64(f).unwrap(),
            Number::Signed(i) => Self::from(i),
            Number::Unsigned(u) => Self::from(u),
        }
    }
}

impl From<YamlNumber> for Number {
    fn from(n: YamlNumber) -> Self {
        if n.is_f64() {
            Self::Float(n.as_f64().unwrap())
        } else if n.is_i64() {
            Self::Signed(n.as_i64().unwrap())
        } else {
            Self::Unsigned(n.as_u64().unwrap())
        }
    }
}

impl From<Number> for YamlNumber {
    fn from(n: Number) -> Self {
        match n {
            Number::Float(f) => Self::from(f),
            Number::Signed(i) => Self::from(i),
            Number::Unsigned(u) => Self::from(u),
        }
    }
}

impl PartialEq<i64> for Number {
    fn eq(&self, other: &i64) -> bool {
        match self {
            Self::Signed(i) => *i == *other,
            _ => false,
        }
    }
}

impl PartialEq<u64> for Number {
    fn eq(&self, other: &u64) -> bool {
        match self {
            Self::Unsigned(u) => *u == *other,
            _ => false,
        }
    }
}

impl PartialEq<f64> for Number {
    fn eq(&self, other: &f64) -> bool {
        match self {
            Self::Float(f) => *f == *other,
            _ => false,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn markdown_parsing() {
        const WITH_PREAMBLE: &str = r#"---
title: Blog post
description: My first blog post
---
Content goes **here**.

And here.
"#;
        let parsed = Value::from_str("md", WITH_PREAMBLE).unwrap();
        match parsed {
            Value::Object(obj) => {
                assert_eq!(obj.get("title").unwrap().as_str().unwrap(), "Blog post");
            }
            _ => panic!(
                "unexpected value parsed from Markdown content: {:#?}",
                parsed
            ),
        }
    }
}
