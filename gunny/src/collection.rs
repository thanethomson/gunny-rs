use rusqlite::Connection;

use crate::{Error, Map, ValueType};

/// A schema allows us to define the structure of a set of results queried from
/// collections.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Schema(Map<String, ValueType>);

/// Allows for loading of schemas from multiple different types of data sources.
impl TryFrom<ValueType> for Schema {
    type Error = Error;

    fn try_from(value: ValueType) -> Result<Self, Self::Error> {
        todo!()
    }
}

/// A collection is a group of items of the same type.
///
/// It is analogous to a table in a database.
#[derive(Debug)]
pub struct Collection {
    schema: Schema,
}

/// Provides an interface for creating and querying collections.
///
/// At present, we embed an in-memory SQLite database to allow for complex
/// querying.
#[derive(Debug)]
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Constructor.
    pub fn new() -> Result<Self, Error> {
        Ok(Self {
            conn: Connection::open_in_memory()?,
        })
    }
}
