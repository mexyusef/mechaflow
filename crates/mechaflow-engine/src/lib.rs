/*!
 * MechaFlow Engine
 *
 * This crate provides the workflow engine for the MechaFlow automation system,
 * including workflow definitions, state machines, and rule-based automation.
 */

#![warn(missing_docs)]
#![warn(rustdoc::missing_doc_code_examples)]

// Re-export core types
pub use mechaflow_core::prelude;

// Re-export types from mechaflow_core for convenience
pub use mechaflow_core::types::{Id, Value};

pub mod workflow;
pub mod state_machine;
pub mod trigger;
pub mod action;
pub mod condition;
pub mod context;
pub mod error;
pub mod scheduler;
pub mod rules;
pub mod types;

// Re-export main types for convenience
pub use workflow::{Workflow, WorkflowBuilder, WorkflowId};
pub use state_machine::{StateMachine, StateMachineBuilder, State, StateId, Transition, EventId};
pub use trigger::{Trigger, TriggerId};
pub use action::{Action, ActionId};
pub use condition::{Condition, ConditionId};
pub use context::WorkflowContext;
pub use error::{Error, Result};
pub use scheduler::{WorkflowScheduler, WorkflowExecutionStatus};
pub use rules::{Rule, RuleBuilder, RuleEngine, RuleId};
pub use types::{
    Metadata, Identifiable, Named, Described, HasMetadata,
    ExecutionStatus, ExecutionRecord, Executable, Validatable
};

/// MechaFlow engine crate version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the engine
pub fn init() -> Result<()> {
    tracing::info!("MechaFlow Engine {} initialized", VERSION);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}
