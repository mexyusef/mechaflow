/*!
 * Error types for the MechaFlow engine crate.
 */
use thiserror::Error;

/// Error type for MechaFlow engine operations
#[derive(Error, Debug)]
pub enum Error {
    /// Workflow error
    #[error("Workflow error: {0}")]
    Workflow(String),

    /// State machine error
    #[error("State machine error: {0}")]
    StateMachine(String),

    /// Trigger error
    #[error("Trigger error: {0}")]
    Trigger(String),

    /// Action error
    #[error("Action error: {0}")]
    Action(String),

    /// Condition error
    #[error("Condition error: {0}")]
    Condition(String),

    /// Rule error
    #[error("Rule error: {0}")]
    Rule(String),

    /// Scheduler error
    #[error("Scheduler error: {0}")]
    Scheduler(String),

    /// Device error
    #[error("Device error: {0}")]
    Device(#[from] mechaflow_devices::DeviceError),

    /// Core error
    #[error("Core error: {0}")]
    Core(#[from] mechaflow_core::error::Error),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// Not found error
    #[error("Not found: {0}")]
    NotFound(String),

    /// Already exists error
    #[error("Already exists: {0}")]
    AlreadyExists(String),

    /// Timeout error
    #[error("Timeout: {0}")]
    Timeout(String),

    /// Other error
    #[error("Other error: {0}")]
    Other(String),
}

/// Result type for MechaFlow engine operations
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Create a new workflow error
    pub fn workflow<S: AsRef<str>>(msg: S) -> Self {
        Error::Workflow(msg.as_ref().to_string())
    }

    /// Create a new state machine error
    pub fn state_machine<S: AsRef<str>>(msg: S) -> Self {
        Error::StateMachine(msg.as_ref().to_string())
    }

    /// Create a new trigger error
    pub fn trigger<S: AsRef<str>>(msg: S) -> Self {
        Error::Trigger(msg.as_ref().to_string())
    }

    /// Create a new action error
    pub fn action<S: AsRef<str>>(msg: S) -> Self {
        Error::Action(msg.as_ref().to_string())
    }

    /// Create a new condition error
    pub fn condition<S: AsRef<str>>(msg: S) -> Self {
        Error::Condition(msg.as_ref().to_string())
    }

    /// Create a new rule error
    pub fn rule<S: AsRef<str>>(msg: S) -> Self {
        Error::Rule(msg.as_ref().to_string())
    }

    /// Create a new scheduler error
    pub fn scheduler<S: AsRef<str>>(msg: S) -> Self {
        Error::Scheduler(msg.as_ref().to_string())
    }

    /// Create a new validation error
    pub fn validation<S: AsRef<str>>(msg: S) -> Self {
        Error::Validation(msg.as_ref().to_string())
    }

    /// Create a new not found error
    pub fn not_found<S: AsRef<str>>(msg: S) -> Self {
        Error::NotFound(msg.as_ref().to_string())
    }

    /// Create a new already exists error
    pub fn already_exists<S: AsRef<str>>(msg: S) -> Self {
        Error::AlreadyExists(msg.as_ref().to_string())
    }

    /// Create a new timeout error
    pub fn timeout<S: AsRef<str>>(msg: S) -> Self {
        Error::Timeout(msg.as_ref().to_string())
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
