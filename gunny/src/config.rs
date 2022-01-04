//! Configuration-related functionality for Gunny.

use std::collections::HashMap;

use eyre::Result;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Gunny project configuration.
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Config(HashMap<String, Value>);

impl Config {
    /// Convenience method for constructing a configuration object.
    pub fn with<K, V>(mut self, key: K, value: V) -> Result<Self>
    where
        K: AsRef<str>,
        V: Serialize,
    {
        let _ = self.set(key, value)?;
        Ok(self)
    }

    /// Returns whether or not the configuration contains a value associated
    /// with the given key.
    pub fn contains_key<K: AsRef<str>>(&self, key: K) -> bool {
        self.0.contains_key(key.as_ref())
    }

    /// Set the value associated with the given key.
    pub fn set<K, V>(&mut self, key: K, value: V) -> Result<Option<Value>>
    where
        K: AsRef<str>,
        V: Serialize,
    {
        let value = serde_json::to_value(value)?;
        let key = key.as_ref().to_string();
        let maybe_prev = self.0.insert(key, value);
        Ok(maybe_prev)
    }

    /// Get a reference to the value associated with the given key.
    pub fn get<K: AsRef<str>>(&self, key: K) -> Option<&Value> {
        self.0.get(key.as_ref())
    }

    /// Remove the value associated with the given key, if it exists.
    pub fn remove<K: AsRef<str>>(&mut self, key: K) -> Option<Value> {
        self.0.remove(key.as_ref())
    }

    /// An iterator visiting all key/value pairs in arbitrary order.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Value)> {
        self.0.iter()
    }
}

impl From<Config> for Value {
    fn from(cfg: Config) -> Self {
        Value::Object(Map::from_iter(cfg.0.into_iter()))
    }
}

#[cfg(test)]
mod test {
    use crate::js::register_json_var;

    use super::*;
    use boa::JsValue;

    #[test]
    fn use_config_from_js() {
        let config = Config::default()
            .with("projectName", "Test")
            .unwrap()
            .with("magicNumber", 42)
            .unwrap()
            .with("arr", vec![1, 2, 3, 4])
            .unwrap();
        let mut ctx = boa::Context::new();
        register_json_var(&mut ctx, "config", &config).unwrap();
        let result = ctx
            .eval(r#"config.projectName + " " + config.magicNumber + " " + config.arr.join()"#)
            .unwrap();
        match &result {
            JsValue::String(s) => assert_eq!(s.to_string(), "Test 42 1,2,3,4"),
            _ => panic!("expected result to be a string, but got {:?}", result),
        }

        // Use from within a function
        let result = ctx
            .eval(
                r#"
                function configTest() {
                    return config.projectName;
                }
                configTest()
            "#,
            )
            .unwrap();
        match &result {
            JsValue::String(s) => assert_eq!(s.to_string(), "Test"),
            _ => panic!(
                "expected result from configTest() call to be a string, but got {:?}",
                result
            ),
        }
    }
}
