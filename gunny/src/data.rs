//! Data is read from the file system and is transformed prior to being rendered
//! through a template.

use std::{collections::HashMap, fs, path::Path};

use eyre::Result;
use serde_json::{Map, Number as JsonNumber, Value as JsonValue};
use serde_yaml::{Mapping, Number as YamlNumber, Value as YamlValue};

use crate::Error;

/// An intermediate dynamic type for conversion between different serialization
/// formats.
#[derive(Debug)]
pub enum Value {
    Null,
    Boolean(bool),
    String(String),
    Float(f64),
    Signed(i64),
    Unsigned(u64),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
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
        Self::Float(f)
    }
}

impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Self::Signed(i)
    }
}

impl From<u64> for Value {
    fn from(u: u64) -> Self {
        Self::Unsigned(u)
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
        Self::Object(HashMap::from_iter(
            m.into_iter().map(|(k, v)| (k, v.into())),
        ))
    }
}

impl From<JsonValue> for Value {
    fn from(v: JsonValue) -> Self {
        match v {
            JsonValue::Null => Self::Null,
            JsonValue::Bool(b) => Self::Boolean(b),
            JsonValue::Number(n) => {
                if n.is_f64() {
                    Self::Float(n.as_f64().unwrap())
                } else if let Some(u) = n.as_u64() {
                    Self::Unsigned(u)
                } else {
                    Self::Signed(n.as_i64().unwrap())
                }
            }
            JsonValue::String(s) => Self::String(s),
            JsonValue::Array(arr) => Self::Array(arr.iter().cloned().map(Into::into).collect()),
            JsonValue::Object(obj) => {
                let mut map = HashMap::new();
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
            Value::Float(f) => Self::Number(JsonNumber::from_f64(f).unwrap()),
            Value::Signed(i) => Self::Number(JsonNumber::from(i)),
            Value::Unsigned(u) => Self::Number(JsonNumber::from(u)),
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
            YamlValue::Number(n) => {
                if n.is_f64() {
                    Self::Float(n.as_f64().unwrap())
                } else if n.is_i64() {
                    Self::Signed(n.as_i64().unwrap())
                } else {
                    Self::Unsigned(n.as_u64().unwrap())
                }
            }
            YamlValue::String(s) => Self::String(s),
            YamlValue::Sequence(s) => Self::Array(
                s.into_iter()
                    .map(TryFrom::try_from)
                    .collect::<Result<Vec<Self>>>()?,
            ),
            YamlValue::Mapping(m) => Self::Object(HashMap::from_iter(
                m.into_iter()
                    .map(|(k, v)| {
                        let k = serde_yaml::to_string(&k)?;
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
            Value::Float(f) => Self::Number(YamlNumber::from(f)),
            Value::Signed(i) => Self::Number(YamlNumber::from(i)),
            Value::Unsigned(u) => Self::Number(YamlNumber::from(u)),
            Value::Array(a) => Self::Sequence(a.into_iter().map(Into::into).collect()),
            Value::Object(o) => Self::Mapping(Mapping::from_iter(
                o.into_iter().map(|(k, v)| (k.into(), v.into())),
            )),
        }
    }
}

/// Load arbitrary structured data from the given text file.
pub fn load_data<P: AsRef<Path>>(path: P) -> Result<Value> {
    let path = path.as_ref();
    let ext = path
        .extension()
        .ok_or_else(|| Error::CannotDetermineDataFileType(path.to_path_buf()))?
        .to_str()
        .unwrap();
    let content = fs::read_to_string(path)?;
    match ext {
        "json" => Ok(serde_json::from_str::<JsonValue>(&content)?.into()),
        "yml" | "yaml" => serde_yaml::from_str::<YamlValue>(&content)?.try_into(),
        "md" => parse_markdown(&content),
        _ => Err(Error::CannotDetermineDataFileType(path.to_path_buf()).into()),
    }
}

fn split_front_matter(content: &str) -> (Option<&str>, &str) {
    const DELIMITERS: &[&str] = &["---\n", "---\r\n"];
    for delim in DELIMITERS {
        if content.starts_with(delim) {
            let parts = content.split(delim).collect::<Vec<&str>>();
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
        None => Value::Object(HashMap::new()),
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
