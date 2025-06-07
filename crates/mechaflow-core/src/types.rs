/*!
 * Core data types for MechaFlow.
 *
 * This module defines the fundamental data types used throughout the MechaFlow ecosystem.
 */
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A unique identifier for MechaFlow resources
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Id(String);

impl Id {
    /// Create a new ID with a random UUID
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Create an ID from a string
    pub fn from_string<S: AsRef<str>>(s: S) -> Self {
        Self(s.as_ref().to_string())
    }

    /// Get the string representation of the ID
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for Id {
    fn default() -> Self {
        Self::new()
    }
}

impl From<&str> for Id {
    fn from(s: &str) -> Self {
        Self::from_string(s)
    }
}

impl From<String> for Id {
    fn from(s: String) -> Self {
        Self::from_string(s)
    }
}

impl From<Uuid> for Id {
    fn from(uuid: Uuid) -> Self {
        Self::from_string(uuid.to_string())
    }
}

/// A strongly-typed value that can be used in MechaFlow
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    /// Null value
    Null,
    /// Boolean value
    Bool(bool),
    /// Integer value
    Integer(i64),
    /// Floating-point value
    Float(f64),
    /// String value
    String(String),
    /// Array of values
    Array(Vec<Value>),
    /// Map of string keys to values
    Object(HashMap<String, Value>),
    /// Binary data
    Binary(Vec<u8>),
    /// Timestamp
    Timestamp(DateTime<Utc>),
}

impl Value {
    /// Check if the value is null
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Check if the value is a boolean
    pub fn is_bool(&self) -> bool {
        matches!(self, Value::Bool(_))
    }

    /// Check if the value is an integer
    pub fn is_integer(&self) -> bool {
        matches!(self, Value::Integer(_))
    }

    /// Check if the value is a float
    pub fn is_float(&self) -> bool {
        matches!(self, Value::Float(_))
    }

    /// Check if the value is numeric (integer or float)
    pub fn is_numeric(&self) -> bool {
        self.is_integer() || self.is_float()
    }

    /// Check if the value is a string
    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }

    /// Check if the value is an array
    pub fn is_array(&self) -> bool {
        matches!(self, Value::Array(_))
    }

    /// Check if the value is an object
    pub fn is_object(&self) -> bool {
        matches!(self, Value::Object(_))
    }

    /// Check if the value is binary data
    pub fn is_binary(&self) -> bool {
        matches!(self, Value::Binary(_))
    }

    /// Check if the value is a timestamp
    pub fn is_timestamp(&self) -> bool {
        matches!(self, Value::Timestamp(_))
    }

    /// Try to get a boolean value
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Try to get an integer value
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            Value::Integer(i) => Some(*i),
            Value::Float(f) if *f == (*f as i64) as f64 => Some(*f as i64),
            _ => None,
        }
    }

    /// Try to get a float value
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// Try to get a string value
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// Try to get an array value
    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }

    /// Try to get an object value
    pub fn as_object(&self) -> Option<&HashMap<String, Value>> {
        match self {
            Value::Object(o) => Some(o),
            _ => None,
        }
    }

    /// Try to get binary data
    pub fn as_binary(&self) -> Option<&[u8]> {
        match self {
            Value::Binary(b) => Some(b),
            _ => None,
        }
    }

    /// Try to get a timestamp value
    pub fn as_timestamp(&self) -> Option<&DateTime<Utc>> {
        match self {
            Value::Timestamp(t) => Some(t),
            _ => None,
        }
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::Null
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<i32> for Value {
    fn from(i: i32) -> Self {
        Value::Integer(i as i64)
    }
}

impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Value::Integer(i)
    }
}

impl From<f32> for Value {
    fn from(f: f32) -> Self {
        Value::Float(f as f64)
    }
}

impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Value::Float(f)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_string())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl From<Vec<u8>> for Value {
    fn from(b: Vec<u8>) -> Self {
        Value::Binary(b)
    }
}

impl From<DateTime<Utc>> for Value {
    fn from(t: DateTime<Utc>) -> Self {
        Value::Timestamp(t)
    }
}

impl From<Vec<Value>> for Value {
    fn from(a: Vec<Value>) -> Self {
        Value::Array(a)
    }
}

impl From<HashMap<String, Value>> for Value {
    fn from(o: HashMap<String, Value>) -> Self {
        Value::Object(o)
    }
}

/// A key-value pair of metadata
pub type Metadata = HashMap<String, Value>;

/// A reference-counted value
pub type SharedValue = Arc<Value>;

/// A reference-counted metadata
pub type SharedMetadata = Arc<Metadata>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_creation() {
        let id = Id::new();
        assert!(!id.as_str().is_empty());

        let id = Id::from_string("test-id");
        assert_eq!(id.as_str(), "test-id");

        let id: Id = "another-id".into();
        assert_eq!(id.as_str(), "another-id");

        let id: Id = String::from("string-id").into();
        assert_eq!(id.as_str(), "string-id");
    }

    #[test]
    fn test_id_display() {
        let id = Id::from_string("test-id");
        assert_eq!(format!("{}", id), "test-id");
    }

    #[test]
    fn test_value_type_checks() {
        let v = Value::Null;
        assert!(v.is_null());

        let v = Value::Bool(true);
        assert!(v.is_bool());

        let v = Value::Integer(42);
        assert!(v.is_integer());
        assert!(v.is_numeric());

        let v = Value::Float(3.14);
        assert!(v.is_float());
        assert!(v.is_numeric());

        let v = Value::String("hello".to_string());
        assert!(v.is_string());

        let v = Value::Array(vec![Value::Integer(1), Value::Integer(2)]);
        assert!(v.is_array());

        let mut map = HashMap::new();
        map.insert("key".to_string(), Value::String("value".to_string()));
        let v = Value::Object(map);
        assert!(v.is_object());

        let v = Value::Binary(vec![1, 2, 3]);
        assert!(v.is_binary());

        let v = Value::Timestamp(Utc::now());
        assert!(v.is_timestamp());
    }

    #[test]
    fn test_value_conversions() {
        let v: Value = true.into();
        assert_eq!(v.as_bool(), Some(true));

        let v: Value = 42i32.into();
        assert_eq!(v.as_integer(), Some(42));

        let v: Value = 42i64.into();
        assert_eq!(v.as_integer(), Some(42));

        let v: Value = 3.14f32.into();
        assert!(v.as_float().unwrap() - 3.14 < 0.0001);

        let v: Value = 3.14f64.into();
        assert_eq!(v.as_float(), Some(3.14));

        let v: Value = "hello".into();
        assert_eq!(v.as_str(), Some("hello"));

        let v: Value = String::from("hello").into();
        assert_eq!(v.as_str(), Some("hello"));

        let binary = vec![1u8, 2, 3];
        let v: Value = binary.clone().into();
        assert_eq!(v.as_binary(), Some(&binary[..]));

        let now = Utc::now();
        let v: Value = now.into();
        assert_eq!(v.as_timestamp().unwrap(), &now);

        let array = vec![Value::Integer(1), Value::Integer(2)];
        let v: Value = array.clone().into();
        assert_eq!(v.as_array().unwrap(), &array[..]);

        let mut map = HashMap::new();
        map.insert("key".to_string(), Value::String("value".to_string()));
        let v: Value = map.clone().into();
        assert_eq!(v.as_object().unwrap(), &map);
    }

    #[test]
    fn test_value_as_methods() {
        // Test numeric conversions
        let v = Value::Integer(42);
        assert_eq!(v.as_integer(), Some(42));
        assert_eq!(v.as_float(), Some(42.0));

        let v = Value::Float(3.0);
        assert_eq!(v.as_integer(), Some(3));
        assert_eq!(v.as_float(), Some(3.0));

        let v = Value::Float(3.14);
        assert_eq!(v.as_integer(), None); // Not an exact integer
        assert_eq!(v.as_float(), Some(3.14));

        // Test other conversions
        let v = Value::Bool(true);
        assert_eq!(v.as_bool(), Some(true));
        assert_eq!(v.as_integer(), None);

        let v = Value::String("hello".to_string());
        assert_eq!(v.as_str(), Some("hello"));
        assert_eq!(v.as_bool(), None);
    }
}
