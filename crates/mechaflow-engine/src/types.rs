//! Common types used throughout the MechaFlow engine.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use mechaflow_core::types::{Id, Value};

/// Metadata for various engine components.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Metadata(HashMap<String, Value>);

impl Metadata {
    /// Creates a new empty metadata collection.
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Creates a new metadata collection from a HashMap.
    pub fn from_map(map: HashMap<String, Value>) -> Self {
        Self(map)
    }

    /// Inserts a value into the metadata collection.
    pub fn insert(&mut self, key: impl Into<String>, value: Value) -> Option<Value> {
        self.0.insert(key.into(), value)
    }

    /// Gets a value from the metadata collection.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.0.get(key)
    }

    /// Gets a mutable reference to a value from the metadata collection.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Value> {
        self.0.get_mut(key)
    }

    /// Removes a value from the metadata collection.
    pub fn remove(&mut self, key: &str) -> Option<Value> {
        self.0.remove(key)
    }

    /// Returns all metadata as a reference to a HashMap.
    pub fn as_map(&self) -> &HashMap<String, Value> {
        &self.0
    }

    /// Returns all metadata as a mutable reference to a HashMap.
    pub fn as_map_mut(&mut self) -> &mut HashMap<String, Value> {
        &mut self.0
    }

    /// Returns whether the metadata collection is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the number of entries in the metadata collection.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns an iterator over the metadata key-value pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Value)> {
        self.0.iter()
    }
}

/// A trait for components that can be identified.
pub trait Identifiable {
    /// Gets the ID of the component.
    fn get_id(&self) -> &Id;
}

/// A trait for components that have a name.
pub trait Named {
    /// Gets the name of the component.
    fn get_name(&self) -> &str;
}

/// A trait for components that have a description.
pub trait Described {
    /// Gets the description of the component.
    fn get_description(&self) -> Option<&str>;
}

/// A trait for components that have metadata.
pub trait HasMetadata {
    /// Gets the metadata of the component.
    fn get_metadata(&self) -> &Metadata;

    /// Gets mutable access to the metadata of the component.
    fn get_metadata_mut(&mut self) -> &mut Metadata;
}

/// An enumeration of execution status values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    /// The execution has not started yet.
    NotStarted,
    /// The execution is currently running.
    Running,
    /// The execution completed successfully.
    Completed,
    /// The execution failed.
    Failed,
    /// The execution was cancelled.
    Cancelled,
    /// The execution timed out.
    TimedOut,
}

impl fmt::Display for ExecutionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotStarted => write!(f, "Not Started"),
            Self::Running => write!(f, "Running"),
            Self::Completed => write!(f, "Completed"),
            Self::Failed => write!(f, "Failed"),
            Self::Cancelled => write!(f, "Cancelled"),
            Self::TimedOut => write!(f, "Timed Out"),
        }
    }
}

/// A record of an execution operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    /// Unique identifier for this execution.
    pub id: Id,
    /// The ID of the component that was executed.
    pub component_id: Id,
    /// The type of component that was executed.
    pub component_type: String,
    /// The name of the component that was executed.
    pub component_name: String,
    /// The time when the execution started.
    pub start_time: DateTime<Utc>,
    /// The time when the execution ended, if it has ended.
    pub end_time: Option<DateTime<Utc>>,
    /// The status of the execution.
    pub status: ExecutionStatus,
    /// An error message, if the execution failed.
    pub error: Option<String>,
    /// Additional metadata about the execution.
    pub metadata: Metadata,
}

impl ExecutionRecord {
    /// Creates a new execution record.
    pub fn new(component_id: Id, component_type: impl Into<String>, component_name: impl Into<String>) -> Self {
        Self {
            id: Id::new(Uuid::new_v4().to_string()),
            component_id,
            component_type: component_type.into(),
            component_name: component_name.into(),
            start_time: Utc::now(),
            end_time: None,
            status: ExecutionStatus::NotStarted,
            error: None,
            metadata: Metadata::new(),
        }
    }

    /// Marks the execution as started.
    pub fn mark_started(&mut self) {
        self.status = ExecutionStatus::Running;
        self.start_time = Utc::now();
    }

    /// Marks the execution as completed.
    pub fn mark_completed(&mut self) {
        self.status = ExecutionStatus::Completed;
        self.end_time = Some(Utc::now());
    }

    /// Marks the execution as failed with an error message.
    pub fn mark_failed(&mut self, error: impl Into<String>) {
        self.status = ExecutionStatus::Failed;
        self.error = Some(error.into());
        self.end_time = Some(Utc::now());
    }

    /// Marks the execution as cancelled.
    pub fn mark_cancelled(&mut self) {
        self.status = ExecutionStatus::Cancelled;
        self.end_time = Some(Utc::now());
    }

    /// Marks the execution as timed out.
    pub fn mark_timed_out(&mut self) {
        self.status = ExecutionStatus::TimedOut;
        self.end_time = Some(Utc::now());
    }

    /// Gets the duration of the execution, if it has ended.
    pub fn duration(&self) -> Option<chrono::Duration> {
        self.end_time.map(|end| end - self.start_time)
    }

    /// Adds metadata to the execution record.
    pub fn add_metadata(&mut self, key: impl Into<String>, value: Value) {
        self.metadata.insert(key, value);
    }
}

/// A trait for components that can be executed within a context.
#[async_trait]
pub trait Executable<C> {
    /// Executes the component within the given context.
    async fn execute(&self, context: &C) -> crate::error::Result<Value>;
}

/// A trait for components that can be validated.
#[async_trait]
pub trait Validatable<C> {
    /// Validates the component within the given context.
    async fn validate(&self, context: &C) -> crate::error::Result<()>;
}
