use std::{collections::BTreeMap, ffi::OsStr, fs, path::Path, str::FromStr};

use fixed::types::I64F64;
use serde::{
    ser::{SerializeMap, SerializeSeq},
    Serialize,
};
use serde_json::{Map as JsonMap, Number as JsonNumber, Value as JsonValue};
use serde_yaml::{Number as YamlNumber, Value as YamlValue};
use toml::{value::Datetime as TomlDateTime, Value as TomlValue};

use crate::{Date, DateTime, Error};

/// We use [`std::collections::BTreeMap`] as our default map structure.
pub type Map<K, V> = BTreeMap<K, V>;

/// The fixed-point number type that we use for representing floating point
/// values. This is currently a 128-bit number, with 64 bits for representing
/// the integer part and another 64 bits for representing the floating point
/// part.
pub type Fixed = I64F64;

/// Facilitates defining schemas.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ValueType {
    /// For internal use only.
    Unknown,
    /// An optional type that is either `None` or an instance of the inner type.
    Option(Box<ValueType>),
    /// A boolean value (`true` or `false`).
    Bool,
    /// A signed 64-bit integer.
    Signed,
    /// An unsigned 64-bit integer.
    Unsigned,
    /// A fixed value (for reliably representing floating point values). See
    /// [`Fixed`] for details.
    Fixed,
    /// A string of characters.
    String,
    /// A date without time zone.
    Date,
    /// A date and time without time zone.
    DateTime,
    /// An array of the given type.
    Array(Box<ValueType>),
    /// A map of string values to the given type.
    Map(Box<ValueType>),
}

impl std::fmt::Display for ValueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Unknown => "Unknown",
                Self::Option(t) => format!("Option<{}>", t).as_str(),
                Self::Bool => "Bool",
                Self::Signed => "Int",
                Self::Unsigned => "Uint",
                Self::Fixed => "Fixed",
                Self::String => "String",
                Self::Date => "Date",
                Self::DateTime => "DateTime",
                Self::Array(t) => format!("Array<{}>", t).as_str(),
                Self::Map(t) => format!("Map<{}>", t).as_str(),
            }
        )
    }
}

impl FromStr for ValueType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Match all simple types
        Ok(match s {
            "Unknown" => return Err(Error::UnknownValueType),
            "Bool" => Self::Bool,
            "Int" => Self::Signed,
            "Uint" => Self::Unsigned,
            "Fixed" | "Float" => Self::Fixed,
            "String" => Self::String,
            "Date" => Self::Date,
            "DateTime" => Self::DateTime,
            _ => try_parse_complex_value_type(&s)?,
        })
    }
}

fn try_parse_complex_value_type(s: &str) -> Result<ValueType, Error> {
    todo!()
}

/// Allows us to parse a value type from various sources.
impl TryFrom<Value> for ValueType {
    type Error = Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        Ok(match value {
            Value::String(s) => todo!(),
            Value::Map(_) => todo!(),
            _ => return Err(Error::CannotParseTypeFromValue(value.get_type())),
        })
    }
}

impl ValueType {
    /// A relaxed comparison between this type and the given type.
    ///
    /// Because we sometimes don't know the type of a particular field in a
    /// dynamically defined value (e.g. if it's `None` in an optional value or
    /// an empty array), we need to be more flexible than just doing strict
    /// comparisons.
    pub fn relaxed_eq(&self, other: &Self) -> bool {
        // Best case scenario for simple types
        if self == other {
            return true;
        }
        // If we don't know our type, or the other type, assume that the two
        // types are equal (this is the "relaxed" part of the equality
        // operation).
        if let ValueType::Unknown = self {
            return true;
        }
        if let ValueType::Unknown = other {
            return true;
        }
        // Recursively match on nested types.
        match self {
            Self::Option(inner1) => match other {
                // If we know the other type is an option, compare their inner
                // types.
                Self::Option(inner2) => inner1.relaxed_eq(inner2),
                // If we don't know whether the other type is an option, compare
                // whether our inner type is the same as the other type.
                _ => inner1.relaxed_eq(other),
            },
            Self::Array(inner1) => match other {
                Self::Array(inner2) => inner1.relaxed_eq(inner2),
                // Must be an array
                _ => false,
            },
            Self::Map(inner1) => match other {
                Self::Map(inner2) => inner1.relaxed_eq(inner2),
                // Must be a map
                _ => false,
            },
            // Safety: this should never be reached if the comparison in the
            // above if statement does its job correctly.
            _ => panic!(
                "value type equality condition on simple type {} failed",
                self.to_string()
            ),
        }
    }

    /// Returns the SQLite type associated with this value type.
    pub fn to_sqlite(&self) -> Result<String, Error> {
        self.to_sqlite_nullable(false)
    }

    fn to_sqlite_nullable(&self, nullable: bool) -> Result<String, Error> {
        Ok(format!(
            "{}{}",
            match self {
                Self::Unknown => return Err(Error::UnknownValueType),
                Self::Option(inner) => inner.to_sqlite_nullable(true)?.as_str(),
                Self::Bool => "BOOL",
                Self::Signed => "INT",
                Self::Unsigned => "INT",
                Self::Fixed => "REAL",
                Self::String => "TEXT",
                Self::Date => "DATE",
                Self::DateTime => "DATETIME",
                Self::Array(_) => "TEXT",
                Self::Map(_) => "TEXT",
            },
            if nullable { "" } else { " NOT NULL" }
        ))
    }
}

/// The supported file formats from which we can load [`Value`] instances.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SupportedFormat {
    Json,
    Yaml,
    Toml,
    Markdown,
}

impl FromStr for SupportedFormat {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower = s.to_lowercase();
        Ok(match lower.as_ref() {
            "json" => Self::Json,
            "yaml" | "yml" => Self::Yaml,
            "toml" => Self::Toml,
            "md" | "markdown" => Self::Markdown,
            _ => return Err(Error::UnsupportedFileType(s.to_string())),
        })
    }
}

/// An intermediate type for facilitating conversions between different formats.
#[derive(Debug, PartialEq, PartialOrd)]
pub enum Value {
    Option(Option<Box<Value>>),
    Bool(bool),
    Signed(i64),
    Unsigned(u64),
    Fixed(Fixed),
    String(String),
    Date(Date),
    DateTime(DateTime),
    Array(Vec<Value>),
    Map(Map<String, Value>),
}

impl Value {
    /// Attempts to create a new value by parsing it from a string using the
    /// given format.
    pub fn load_as(fmt: SupportedFormat, content: &str) -> Result<Self, Error> {
        match fmt {
            SupportedFormat::Json => Self::try_from(serde_json::from_str::<JsonValue>(content)?),
            SupportedFormat::Yaml => Self::try_from(serde_yaml::from_str::<YamlValue>(content)?),
            SupportedFormat::Toml => Self::try_from(toml::from_str::<TomlValue>(content)?),
            _ => unimplemented!(),
        }
    }

    /// Attempts to create a new value by loading it from the given file.
    /// Automatically detects the file format and parses/converts it
    /// accordingly.
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let path = path.as_ref();
        let ext = path
            .extension()
            .map(OsStr::to_str)
            .flatten()
            .ok_or_else(|| Error::CannotDetermineFileType(path.to_path_buf()))?;
        let fmt = SupportedFormat::from_str(ext)
            .map_err(|e| Error::LoadFromFile(path.to_path_buf(), Box::new(e)))?;
        let filename = path
            .file_name()
            .map(OsStr::to_str)
            .flatten()
            .ok_or_else(|| Error::CannotExtractFileName(path.to_path_buf()))?;
        let content = fs::read_to_string(path)
            .map_err(|e| Error::Io(format!("while trying to read from {}", path.display()), e))?;
        let mut value = Self::load_as(fmt, &content)?;
        // Automatically set the "id" field for objects that don't provide their
        // own ID to the file name of the file from which they were loaded.
        if let Self::Map(mut m) = value {
            let id_field = "id".to_string();
            if !m.contains_key(&id_field) {
                m.insert(id_field, Self::String(filename.to_string()));
            }
        }
        Ok(value)
    }

    /// Attempt to get the type of this value.
    ///
    /// This does a best-effort guess. Values are loaded from loosely typed
    /// sources, like JSON/YAML files, where `null` values' types cannot be
    /// accurately determined. In cases where we have `null` values, or empty
    /// arrays or maps, we mark these types as [`ValueType::Unknown`].
    pub fn get_type(&self) -> ValueType {
        match self {
            Self::Option(inner) => ValueType::Option(Box::new(
                inner.map(|t| t.get_type()).unwrap_or(ValueType::Unknown),
            )),
            Self::Bool(_) => ValueType::Bool,
            Self::Signed(_) => ValueType::Signed,
            Self::Unsigned(_) => ValueType::Unsigned,
            Self::Fixed(_) => ValueType::Fixed,
            Self::String(_) => ValueType::String,
            Self::Date(_) => ValueType::Date,
            Self::DateTime(_) => ValueType::DateTime,
            Self::Array(inner) => ValueType::Array(Box::new(
                inner
                    .get(0)
                    .map(Self::get_type)
                    .unwrap_or(ValueType::Unknown),
            )),
            Self::Map(inner) => ValueType::Map(Box::new(
                inner
                    .values()
                    .nth(0)
                    .map(Self::get_type)
                    .unwrap_or(ValueType::Unknown),
            )),
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Option(opt) => opt.map(|inner| inner.as_bool()).flatten(),
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_signed(&self) -> Option<i64> {
        match self {
            Self::Option(opt) => opt.map(|inner| inner.as_signed()).flatten(),
            Self::Signed(i) => Some(*i),
            Self::Unsigned(u) => {
                let u = *u;
                if u < (i64::MAX as u64) {
                    Some(u as i64)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn as_unsigned(&self) -> Option<u64> {
        match self {
            Self::Option(opt) => opt.map(|inner| inner.as_unsigned()).flatten(),
            Self::Unsigned(u) => Some(*u),
            Self::Signed(i) => {
                let i = *i;
                if i >= 0 {
                    Some(i as u64)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn as_fixed(&self) -> Option<Fixed> {
        match self {
            Self::Option(opt) => opt.map(|inner| inner.as_fixed()).flatten(),
            Self::Fixed(f) => Some(*f),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        self.as_fixed().map(|f| f.to_num())
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Option(opt) => opt.map(|inner| inner.as_str()).flatten(),
            Self::String(s) => Some(s.as_ref()),
            _ => None,
        }
    }
}

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Value::Option(maybe_inner) => match maybe_inner {
                Some(inner) => serializer.serialize_some(inner),
                None => serializer.serialize_none(),
            },
            Value::Bool(b) => serializer.serialize_bool(*b),
            Value::Signed(i) => serializer.serialize_i64(*i),
            Value::Unsigned(u) => serializer.serialize_u64(*u),
            Value::Fixed(f) => serializer.serialize_f64(f.to_num::<f64>()),
            Value::String(s) => serializer.serialize_str(s),
            Value::Date(d) => serializer.serialize_str(&d.to_string()),
            Value::DateTime(dt) => serializer.serialize_str(&dt.to_string()),
            Value::Array(arr) => {
                let mut seq = serializer.serialize_seq(Some(arr.len()))?;
                for el in arr {
                    seq.serialize_element(el)?;
                }
                seq.end()
            }
            Value::Map(m) => {
                let mut sm = serializer.serialize_map(Some(m.len()))?;
                for (k, v) in m {
                    sm.serialize_entry(k, v)?;
                }
                sm.end()
            }
        }
    }
}

impl From<Value> for JsonValue {
    fn from(v: Value) -> Self {
        match v {
            Value::Option(maybe_inner) => match maybe_inner {
                Some(inner) => JsonValue::from(*inner),
                None => Self::Null,
            },
            Value::Bool(b) => JsonValue::Bool(b),
            Value::Signed(i) => i.into(),
            Value::Unsigned(u) => u.into(),
            Value::Fixed(f) => f.to_num::<f64>().into(),
            Value::String(s) => JsonValue::String(s),
            Value::Date(d) => JsonValue::String(d.to_string()),
            Value::DateTime(dt) => JsonValue::String(dt.to_string()),
            Value::Array(arr) => JsonValue::Array(arr.into_iter().map(Into::into).collect()),
            Value::Map(m) => JsonValue::Object(JsonMap::from_iter(
                m.into_iter().map(|(k, v)| (k, v.into())),
            )),
        }
    }
}

impl TryFrom<JsonValue> for Value {
    type Error = Error;

    fn try_from(value: JsonValue) -> Result<Self, Self::Error> {
        Ok(match value {
            JsonValue::Null => Self::Option(None),
            JsonValue::Bool(b) => Self::Bool(b),
            JsonValue::Number(n) => Self::from(n),
            JsonValue::String(s) => Self::String(s),
            JsonValue::Array(arr) => Self::Array(
                arr.into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<Self>, Error>>()?,
            ),
            JsonValue::Object(obj) => Self::Map(Map::from_iter(
                obj.into_iter()
                    .map(|(k, v)| Ok((k, v.try_into()?)))
                    .collect::<Result<Vec<(String, Self)>, Error>>()?
                    .into_iter(),
            )),
        })
    }
}

impl From<JsonNumber> for Value {
    fn from(value: JsonNumber) -> Self {
        if value.is_f64() {
            Self::Fixed(Fixed::from_num(value.as_f64().unwrap()))
        } else if value.is_i64() {
            Self::Signed(value.as_i64().unwrap())
        } else {
            Self::Unsigned(value.as_u64().unwrap())
        }
    }
}

impl TryFrom<YamlValue> for Value {
    type Error = Error;

    fn try_from(value: YamlValue) -> Result<Self, Self::Error> {
        Ok(match value {
            YamlValue::Null => Self::Option(None),
            YamlValue::Bool(b) => Self::Bool(b),
            YamlValue::Number(n) => Self::from(n),
            YamlValue::String(s) => Self::String(s),
            YamlValue::Sequence(seq) => Self::Array(
                seq.into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<Self>, Error>>()?,
            ),
            YamlValue::Mapping(m) => Self::Map(Map::from_iter(
                m.into_iter()
                    .map(|(k, v)| {
                        if !k.is_string() {
                            return Err(Error::ObjectKeysMustBeStrings);
                        }
                        Ok((k.as_str().unwrap().to_string(), v.try_into()?))
                    })
                    .collect::<Result<Vec<(String, Self)>, Error>>()?
                    .into_iter(),
            )),
        })
    }
}

impl From<YamlNumber> for Value {
    fn from(value: YamlNumber) -> Self {
        if value.is_f64() {
            Self::Fixed(Fixed::from_num(value.as_f64().unwrap()))
        } else if value.is_i64() {
            Self::Signed(value.as_i64().unwrap())
        } else {
            Self::Unsigned(value.as_u64().unwrap())
        }
    }
}

impl TryFrom<TomlValue> for Value {
    type Error = Error;

    fn try_from(value: TomlValue) -> Result<Self, Self::Error> {
        Ok(match value {
            TomlValue::String(s) => Self::String(s),
            TomlValue::Integer(i) => Self::Signed(i),
            TomlValue::Float(f) => Self::Fixed(Fixed::from_num(f)),
            TomlValue::Boolean(b) => Self::Bool(b),
            TomlValue::Datetime(dt) => Self::try_from(dt)?,
            TomlValue::Array(arr) => Self::Array(
                arr.into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<Self>, Error>>()?,
            ),
            TomlValue::Table(t) => Self::Map(Map::from_iter(
                t.into_iter()
                    .map(|(k, v)| Ok((k, Self::try_from(v)?)))
                    .collect::<Result<Vec<(String, Self)>, Error>>()?
                    .into_iter(),
            )),
        })
    }
}

impl TryFrom<TomlDateTime> for Value {
    type Error = Error;

    fn try_from(value: TomlDateTime) -> Result<Self, Self::Error> {
        let s = value.to_string();
        Ok(match DateTime::from_str(&s) {
            Ok(dt) => Self::DateTime(dt),
            Err(_) => match Date::from_str(&s) {
                Ok(d) => Self::Date(d),
                Err(e) => return Err(e),
            },
        })
    }
}
