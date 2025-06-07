/*!
 * Error types for the MechaFlow core crate.
 */
use std::fmt;
use thiserror::Error;

/// Core error type for MechaFlow
#[derive(Error, Debug)]
pub enum Error {
    /// Configuration errors
    #[error("Configuration error: {0}")]
    Config(String),

    /// I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization/deserialization errors
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Runtime errors
    #[error("Runtime error: {0}")]
    Runtime(String),

    /// Event system errors
    #[error("Event error: {0}")]
    Event(String),

    /// Device errors
    #[error("Device error: {0}")]
    Device(String),

    /// Protocol errors
    #[error("Protocol error: {0}")]
    Protocol(String),

    /// Workflow errors
    #[error("Workflow error: {0}")]
    Workflow(String),

    /// Authentication errors
    #[error("Authentication error: {0}")]
    Auth(String),

    /// Authorization errors
    #[error("Authorization error: {0}")]
    Authorization(String),

    /// Validation errors
    #[error("Validation error: {0}")]
    Validation(String),

    /// Resource not found error
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Timeout error
    #[error("Timeout error: {0}")]
    Timeout(String),

    /// Conversion errors
    #[error("Conversion error: {0}")]
    Conversion(String),

    /// Other errors
    #[error("Other error: {0}")]
    Other(String),
}

/// Alias for Result with the MechaFlow Error type
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Create a new configuration error
    pub fn config<S: AsRef<str>>(msg: S) -> Self {
        Error::Config(msg.as_ref().to_string())
    }

    /// Create a new runtime error
    pub fn runtime<S: AsRef<str>>(msg: S) -> Self {
        Error::Runtime(msg.as_ref().to_string())
    }

    /// Create a new event error
    pub fn event<S: AsRef<str>>(msg: S) -> Self {
        Error::Event(msg.as_ref().to_string())
    }

    /// Create a new device error
    pub fn device<S: AsRef<str>>(msg: S) -> Self {
        Error::Device(msg.as_ref().to_string())
    }

    /// Create a new validation error
    pub fn validation<S: AsRef<str>>(msg: S) -> Self {
        Error::Validation(msg.as_ref().to_string())
    }

    /// Create a new not found error
    pub fn not_found<S: AsRef<str>>(msg: S) -> Self {
        Error::NotFound(msg.as_ref().to_string())
    }

    /// Create a new timeout error
    pub fn timeout<S: AsRef<str>>(msg: S) -> Self {
        Error::Timeout(msg.as_ref().to_string())
    }

    /// Create a new serialization error
    pub fn serialization<S: AsRef<str>>(msg: S) -> Self {
        Error::Serialization(msg.as_ref().to_string())
    }

    /// Create a new other error
    pub fn other<S: AsRef<str>>(msg: S) -> Self {
        Error::Other(msg.as_ref().to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Serialization(err.to_string())
    }
}

impl From<toml::de::Error> for Error {
    fn from(err: toml::de::Error) -> Self {
        Error::Serialization(err.to_string())
    }
}

impl From<toml::ser::Error> for Error {
    fn from(err: toml::ser::Error) -> Self {
        Error::Serialization(err.to_string())
    }
}

impl From<config::ConfigError> for Error {
    fn from(err: config::ConfigError) -> Self {
        Error::Config(err.to_string())
    }
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Error::Other(s)
    }
}

impl From<&str> for Error {
    fn from(s: &str) -> Self {
        Error::Other(s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = Error::config("test");
        assert!(matches!(err, Error::Config(_)));

        let err = Error::runtime("test");
        assert!(matches!(err, Error::Runtime(_)));

        let err = Error::event("test");
        assert!(matches!(err, Error::Event(_)));

        let err = Error::device("test");
        assert!(matches!(err, Error::Device(_)));

        let err = Error::validation("test");
        assert!(matches!(err, Error::Validation(_)));

        let err = Error::not_found("test");
        assert!(matches!(err, Error::NotFound(_)));

        let err = Error::timeout("test");
        assert!(matches!(err, Error::Timeout(_)));

        let err = Error::serialization("test");
        assert!(matches!(err, Error::Serialization(_)));

        let err = Error::other("test");
        assert!(matches!(err, Error::Other(_)));
    }

    #[test]
    fn test_error_from_string() {
        let err: Error = "test".into();
        assert!(matches!(err, Error::Other(_)));

        let err: Error = String::from("test").into();
        assert!(matches!(err, Error::Other(_)));
    }

    #[test]
    fn test_error_display() {
        let err = Error::config("test");
        assert_eq!(err.to_string(), "Configuration error: test");

        let err = Error::runtime("test");
        assert_eq!(err.to_string(), "Runtime error: test");

        let err = Error::event("test");
        assert_eq!(err.to_string(), "Event error: test");

        let err = Error::device("test");
        assert_eq!(err.to_string(), "Device error: test");

        let err = Error::validation("test");
        assert_eq!(err.to_string(), "Validation error: test");

        let err = Error::not_found("test");
        assert_eq!(err.to_string(), "Resource not found: test");

        let err = Error::timeout("test");
        assert_eq!(err.to_string(), "Timeout error: test");

        let err = Error::serialization("test");
        assert_eq!(err.to_string(), "Serialization error: test");

        let err = Error::other("test");
        assert_eq!(err.to_string(), "Other error: test");
    }
}
