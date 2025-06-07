/*!
 * Workflow definitions and execution for MechaFlow.
 *
 * This module provides the core workflow functionality for defining and executing
 * automation workflows in the MechaFlow system.
 */
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use mechaflow_core::types::{Id, Value};
use mechaflow_devices::device::{Device, DeviceEvent};
use mechaflow_devices::registry::DeviceRegistry;

use crate::action::{Action, ActionContext, ActionResult};
use crate::condition::Condition;
use crate::context::WorkflowContext;
use crate::error::{Error, Result};
use crate::trigger::{Trigger, TriggerEvent};

/// Workflow identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowId(Id);

impl WorkflowId {
    /// Create a new workflow ID
    pub fn new() -> Self {
        Self(Id::from(uuid::Uuid::new_v4().to_string()))
    }

    /// Create a workflow ID from a string
    pub fn from_string<S: AsRef<str>>(s: S) -> Self {
        Self(Id::from(s.as_ref()))
    }

    /// Get the inner ID
    pub fn inner(&self) -> &Id {
        &self.0
    }
}

impl Default for WorkflowId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for WorkflowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A workflow is a collection of triggers, conditions, and actions
#[derive(Debug, Clone)]
pub struct Workflow {
    /// Workflow ID
    id: WorkflowId,
    /// Workflow name
    name: String,
    /// Workflow description
    description: Option<String>,
    /// Triggers that can start the workflow
    triggers: Vec<Arc<dyn Trigger>>,
    /// Conditions that must be met for the workflow to execute
    conditions: Vec<Arc<dyn Condition>>,
    /// Actions to execute when the workflow runs
    actions: Vec<Arc<dyn Action>>,
    /// Workflow metadata
    metadata: HashMap<String, Value>,
    /// Whether the workflow is enabled
    enabled: bool,
    /// Execution timeout
    timeout: Option<Duration>,
    /// Maximum number of concurrent executions
    max_concurrent: Option<usize>,
    /// Execution statistics
    stats: RwLock<WorkflowStats>,
}

/// Workflow execution statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkflowStats {
    /// Number of times the workflow has been triggered
    pub triggered_count: u64,
    /// Number of times the conditions were met
    pub conditions_met_count: u64,
    /// Number of times the workflow was executed
    pub executed_count: u64,
    /// Number of successful executions
    pub success_count: u64,
    /// Number of failed executions
    pub failure_count: u64,
    /// Total execution time in milliseconds
    pub total_execution_time_ms: u64,
    /// Average execution time in milliseconds
    pub avg_execution_time_ms: f64,
    /// Last execution time
    pub last_execution_time: Option<chrono::DateTime<chrono::Utc>>,
    /// Last successful execution time
    pub last_success_time: Option<chrono::DateTime<chrono::Utc>>,
    /// Last failure time
    pub last_failure_time: Option<chrono::DateTime<chrono::Utc>>,
}

impl Workflow {
    /// Create a new workflow with the given name
    pub fn new<S: Into<String>>(name: S) -> Self {
        Self {
            id: WorkflowId::new(),
            name: name.into(),
            description: None,
            triggers: Vec::new(),
            conditions: Vec::new(),
            actions: Vec::new(),
            metadata: HashMap::new(),
            enabled: true,
            timeout: None,
            max_concurrent: None,
            stats: RwLock::new(WorkflowStats::default()),
        }
    }

    /// Get the workflow ID
    pub fn id(&self) -> &WorkflowId {
        &self.id
    }

    /// Get the workflow name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the workflow description
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Set the workflow description
    pub fn set_description<S: Into<String>>(&mut self, description: S) {
        self.description = Some(description.into());
    }

    /// Add a trigger to the workflow
    pub fn add_trigger<T: Trigger + 'static>(&mut self, trigger: T) {
        self.triggers.push(Arc::new(trigger));
    }

    /// Get all triggers
    pub fn triggers(&self) -> &[Arc<dyn Trigger>] {
        &self.triggers
    }

    /// Add a condition to the workflow
    pub fn add_condition<C: Condition + 'static>(&mut self, condition: C) {
        self.conditions.push(Arc::new(condition));
    }

    /// Get all conditions
    pub fn conditions(&self) -> &[Arc<dyn Condition>] {
        &self.conditions
    }

    /// Add an action to the workflow
    pub fn add_action<A: Action + 'static>(&mut self, action: A) {
        self.actions.push(Arc::new(action));
    }

    /// Get all actions
    pub fn actions(&self) -> &[Arc<dyn Action>] {
        &self.actions
    }

    /// Get workflow metadata
    pub fn metadata(&self) -> &HashMap<String, Value> {
        &self.metadata
    }

    /// Set a metadata value
    pub fn set_metadata<K: Into<String>, V: Into<Value>>(&mut self, key: K, value: V) {
        self.metadata.insert(key.into(), value.into());
    }

    /// Check if the workflow is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Enable the workflow
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable the workflow
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Get the execution timeout
    pub fn timeout(&self) -> Option<Duration> {
        self.timeout
    }

    /// Set the execution timeout
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = Some(timeout);
    }

    /// Clear the execution timeout
    pub fn clear_timeout(&mut self) {
        self.timeout = None;
    }

    /// Get the maximum number of concurrent executions
    pub fn max_concurrent(&self) -> Option<usize> {
        self.max_concurrent
    }

    /// Set the maximum number of concurrent executions
    pub fn set_max_concurrent(&mut self, max: usize) {
        self.max_concurrent = Some(max);
    }

    /// Clear the maximum number of concurrent executions
    pub fn clear_max_concurrent(&mut self) {
        self.max_concurrent = None;
    }

    /// Get a snapshot of the workflow statistics
    pub async fn stats(&self) -> WorkflowStats {
        self.stats.read().await.clone()
    }

    /// Process a trigger event
    ///
    /// This method checks if the workflow should be triggered by the event,
    /// and if so, whether the conditions are met to execute the workflow.
    pub async fn process_event(
        &self,
        event: &TriggerEvent,
        context: &dyn WorkflowContext,
    ) -> Result<bool> {
        if !self.enabled {
            return Ok(false);
        }

        // Check if any trigger matches this event
        let mut triggered = false;
        for trigger in &self.triggers {
            if trigger.matches_event(event).await? {
                triggered = true;
                break;
            }
        }

        if !triggered {
            return Ok(false);
        }

        // Update triggered count
        {
            let mut stats = self.stats.write().await;
            stats.triggered_count += 1;
        }

        // Check conditions
        let conditions_met = self.check_conditions(context).await?;
        if !conditions_met {
            return Ok(false);
        }

        // Execute the workflow
        self.execute(context).await?;

        Ok(true)
    }

    /// Check if all conditions are met
    async fn check_conditions(&self, context: &dyn WorkflowContext) -> Result<bool> {
        for condition in &self.conditions {
            if !condition.evaluate(context).await? {
                return Ok(false);
            }
        }

        // Update conditions met count
        {
            let mut stats = self.stats.write().await;
            stats.conditions_met_count += 1;
        }

        Ok(true)
    }

    /// Execute the workflow
    pub async fn execute(&self, context: &dyn WorkflowContext) -> Result<Vec<ActionResult>> {
        if !self.enabled {
            return Err(Error::workflow(format!("Workflow {} is disabled", self.name)));
        }

        let start_time = std::time::Instant::now();
        let mut results = Vec::new();
        let mut success = true;

        // Update execution count
        {
            let mut stats = self.stats.write().await;
            stats.executed_count += 1;
            stats.last_execution_time = Some(chrono::Utc::now());
        }

        // Create action context
        let action_context = ActionContextImpl {
            workflow_id: self.id.clone(),
            workflow_name: self.name.clone(),
            context,
        };

        // Execute actions
        for action in &self.actions {
            match action.execute(&action_context).await {
                Ok(result) => {
                    results.push(result);
                }
                Err(e) => {
                    error!("Failed to execute action: {}", e);
                    success = false;
                    results.push(ActionResult::Failure(e.to_string()));
                }
            }
        }

        // Update statistics
        let execution_time = start_time.elapsed();
        let execution_time_ms = execution_time.as_millis() as u64;

        {
            let mut stats = self.stats.write().await;
            stats.total_execution_time_ms += execution_time_ms;
            stats.avg_execution_time_ms = stats.total_execution_time_ms as f64 / stats.executed_count as f64;

            if success {
                stats.success_count += 1;
                stats.last_success_time = Some(chrono::Utc::now());
            } else {
                stats.failure_count += 1;
                stats.last_failure_time = Some(chrono::Utc::now());
            }
        }

        Ok(results)
    }
}

/// Action context implementation
struct ActionContextImpl<'a> {
    workflow_id: WorkflowId,
    workflow_name: String,
    context: &'a dyn WorkflowContext,
}

impl<'a> ActionContext for ActionContextImpl<'a> {
    fn workflow_id(&self) -> &WorkflowId {
        &self.workflow_id
    }

    fn workflow_name(&self) -> &str {
        &self.workflow_name
    }

    fn read_property(&self, device_id: &Id, property: &str) -> Result<Value> {
        self.context.read_property(device_id, property)
    }

    fn write_property(&self, device_id: &Id, property: &str, value: Value) -> Result<()> {
        self.context.write_property(device_id, property, value)
    }

    fn execute_command(
        &self,
        device_id: &Id,
        command: &str,
        parameters: HashMap<String, Value>,
    ) -> Result<Value> {
        self.context.execute_command(device_id, command, parameters)
    }

    fn get_device(&self, device_id: &Id) -> Result<Arc<dyn Device>> {
        self.context.get_device(device_id)
    }

    fn get_variable(&self, name: &str) -> Result<Value> {
        self.context.get_variable(name)
    }

    fn set_variable(&self, name: &str, value: Value) -> Result<()> {
        self.context.set_variable(name, value)
    }
}

/// Builder for creating workflows
#[derive(Debug, Default)]
pub struct WorkflowBuilder {
    /// Workflow ID
    id: Option<WorkflowId>,
    /// Workflow name
    name: Option<String>,
    /// Workflow description
    description: Option<String>,
    /// Triggers
    triggers: Vec<Arc<dyn Trigger>>,
    /// Conditions
    conditions: Vec<Arc<dyn Condition>>,
    /// Actions
    actions: Vec<Arc<dyn Action>>,
    /// Metadata
    metadata: HashMap<String, Value>,
    /// Whether the workflow is enabled
    enabled: bool,
    /// Execution timeout
    timeout: Option<Duration>,
    /// Maximum number of concurrent executions
    max_concurrent: Option<usize>,
}

impl WorkflowBuilder {
    /// Create a new workflow builder
    pub fn new() -> Self {
        Self {
            enabled: true,
            ..Default::default()
        }
    }

    /// Set the workflow ID
    pub fn with_id(mut self, id: WorkflowId) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the workflow name
    pub fn with_name<S: Into<String>>(mut self, name: S) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the workflow description
    pub fn with_description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add a trigger
    pub fn with_trigger<T: Trigger + 'static>(mut self, trigger: T) -> Self {
        self.triggers.push(Arc::new(trigger));
        self
    }

    /// Add a condition
    pub fn with_condition<C: Condition + 'static>(mut self, condition: C) -> Self {
        self.conditions.push(Arc::new(condition));
        self
    }

    /// Add an action
    pub fn with_action<A: Action + 'static>(mut self, action: A) -> Self {
        self.actions.push(Arc::new(action));
        self
    }

    /// Set a metadata value
    pub fn with_metadata<K: Into<String>, V: Into<Value>>(mut self, key: K, value: V) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Set whether the workflow is enabled
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set the execution timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set the maximum number of concurrent executions
    pub fn with_max_concurrent(mut self, max: usize) -> Self {
        self.max_concurrent = Some(max);
        self
    }

    /// Build the workflow
    pub fn build(self) -> Result<Workflow> {
        let name = self.name.ok_or_else(|| Error::validation("Workflow name is required"))?;

        Ok(Workflow {
            id: self.id.unwrap_or_else(WorkflowId::new),
            name,
            description: self.description,
            triggers: self.triggers,
            conditions: self.conditions,
            actions: self.actions,
            metadata: self.metadata,
            enabled: self.enabled,
            timeout: self.timeout,
            max_concurrent: self.max_concurrent,
            stats: RwLock::new(WorkflowStats::default()),
        })
    }
}
