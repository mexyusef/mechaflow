/*!
 * Prelude module for MechaFlow Core.
 *
 * This module re-exports commonly used types and functions from the MechaFlow Core crate
 * to make them easier to import.
 */

// Re-export error types
pub use crate::error::{Error, Result};

// Re-export core types
pub use crate::types::{Id, Value, Metadata, SharedValue, SharedMetadata};

// Re-export event types
pub use crate::event::{
    Event, BasicEvent, TypedEvent, Priority, EventBus, SharedEventBus,
};

// Re-export runtime types
pub use crate::runtime::{
    Runtime, SharedRuntime, Device, Workflow, WorkflowContext,
};

// Re-export config types
pub use crate::config::{Config, SharedConfig, ConfigBuilder};

// Re-export utility functions
pub use crate::utils::{
    with_timeout, with_retry, spawn_task, spawn_and_log,
    duration_to_millis, millis_to_duration, box_future,
};

// Re-export logging macros
pub use crate::log_with_fields;
pub use tracing::{trace, debug, info, warn, error};

// Re-export core initialization
pub use crate::init;
