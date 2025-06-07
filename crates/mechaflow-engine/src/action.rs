/*!
 * Actions that can be executed by workflows.
 *
 * This module defines the actions that can be executed when a workflow runs,
 * including device control, service calls, and variable modifications.
 */
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use mechaflow_core::types::{Id, Value};
use mechaflow_devices::device::Device;

use crate::error::{Error, Result};
use crate::workflow::WorkflowId;

/// Result of executing an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionResult {
    /// Action executed successfully with a value
    Success(Value),
    /// Action executed successfully with no value
    Completed,
    /// Action execution failed
    Failure(String),
    /// Action execution timed out
    Timeout,
}

impl ActionResult {
    /// Check if the action executed successfully
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success(_) | Self::Completed)
    }

    /// Check if the action failed
    pub fn is_failure(&self) -> bool {
        matches!(self, Self::Failure(_) | Self::Timeout)
    }

    /// Get the value if successful
    pub fn value(&self) -> Option<&Value> {
        match self {
            Self::Success(v) => Some(v),
            _ => None,
        }
    }

    /// Get the error message if failed
    pub fn error(&self) -> Option<&str> {
        match self {
            Self::Failure(msg) => Some(msg),
            Self::Timeout => Some("Action execution timed out"),
            _ => None,
        }
    }
}

/// Context for executing actions
pub trait ActionContext {
    /// Get the workflow ID
    fn workflow_id(&self) -> &WorkflowId;

    /// Get the workflow name
    fn workflow_name(&self) -> &str;

    /// Read a device property
    fn read_property(&self, device_id: &Id, property: &str) -> Result<Value>;

    /// Write a device property
    fn write_property(&self, device_id: &Id, property: &str, value: Value) -> Result<()>;

    /// Execute a device command
    fn execute_command(
        &self,
        device_id: &Id,
        command: &str,
        parameters: HashMap<String, Value>,
    ) -> Result<Value>;

    /// Get a device by ID
    fn get_device(&self, device_id: &Id) -> Result<Arc<dyn Device>>;

    /// Get a variable
    fn get_variable(&self, name: &str) -> Result<Value>;

    /// Set a variable
    fn set_variable(&self, name: &str, value: Value) -> Result<()>;
}

/// An action that can be executed by a workflow
#[async_trait]
pub trait Action: fmt::Debug + Send + Sync {
    /// Get the action type
    fn action_type(&self) -> &'static str;

    /// Execute the action
    async fn execute(&self, context: &dyn ActionContext) -> Result<ActionResult>;

    /// Get the action ID
    fn id(&self) -> &str;

    /// Get the action description
    fn description(&self) -> Option<&str>;
}

/// A device action that controls a device
#[derive(Debug, Clone)]
pub struct DeviceAction {
    /// Action ID
    id: String,
    /// Action description
    description: Option<String>,
    /// Device ID
    device_id: Id,
    /// Action type: "property", "command"
    action_type: String,
    /// Property to modify or command to execute
    target: String,
    /// Value to set or command parameters
    value: Value,
    /// Timeout for the action
    timeout: Option<Duration>,
}

impl DeviceAction {
    /// Create a new device action
    pub fn new<S: Into<String>>(id: S, device_id: Id) -> Self {
        Self {
            id: id.into(),
            description: None,
            device_id,
            action_type: "property".to_string(),
            target: String::new(),
            value: Value::Null,
            timeout: None,
        }
    }

    /// Set the action description
    pub fn with_description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set a property to modify
    pub fn set_property<S: Into<String>, V: Into<Value>>(mut self, property: S, value: V) -> Self {
        self.action_type = "property".to_string();
        self.target = property.into();
        self.value = value.into();
        self
    }

    /// Set a command to execute
    pub fn execute_command<S: Into<String>, V: Into<Value>>(mut self, command: S, params: V) -> Self {
        self.action_type = "command".to_string();
        self.target = command.into();
        self.value = params.into();
        self
    }

    /// Set the timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
}

#[async_trait]
impl Action for DeviceAction {
    fn action_type(&self) -> &'static str {
        "device"
    }

    async fn execute(&self, context: &dyn ActionContext) -> Result<ActionResult> {
        match self.action_type.as_str() {
            "property" => {
                context.write_property(&self.device_id, &self.target, self.value.clone())?;
                Ok(ActionResult::Completed)
            }
            "command" => {
                let params = match &self.value {
                    Value::Object(map) => {
                        // Convert to HashMap<String, Value>
                        map.clone()
                    }
                    _ => {
                        return Err(Error::action(format!(
                            "Command parameters must be an object, got {:?}",
                            self.value
                        )));
                    }
                };

                let result = context.execute_command(&self.device_id, &self.target, params)?;
                Ok(ActionResult::Success(result))
            }
            _ => Err(Error::action(format!("Unknown device action type: {}", self.action_type))),
        }
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

/// A variable action that sets a variable
#[derive(Debug, Clone)]
pub struct VariableAction {
    /// Action ID
    id: String,
    /// Action description
    description: Option<String>,
    /// Variable name
    variable: String,
    /// Value to set
    value: Value,
}

impl VariableAction {
    /// Create a new variable action
    pub fn new<S1: Into<String>, S2: Into<String>, V: Into<Value>>(id: S1, variable: S2, value: V) -> Self {
        Self {
            id: id.into(),
            description: None,
            variable: variable.into(),
            value: value.into(),
        }
    }

    /// Set the action description
    pub fn with_description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }
}

#[async_trait]
impl Action for VariableAction {
    fn action_type(&self) -> &'static str {
        "variable"
    }

    async fn execute(&self, context: &dyn ActionContext) -> Result<ActionResult> {
        context.set_variable(&self.variable, self.value.clone())?;
        Ok(ActionResult::Completed)
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

/// A delay action that waits for a specified duration
#[derive(Debug, Clone)]
pub struct DelayAction {
    /// Action ID
    id: String,
    /// Action description
    description: Option<String>,
    /// Duration to wait
    duration: Duration,
}

impl DelayAction {
    /// Create a new delay action
    pub fn new<S: Into<String>>(id: S, duration: Duration) -> Self {
        Self {
            id: id.into(),
            description: None,
            duration,
        }
    }

    /// Set the action description
    pub fn with_description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }
}

#[async_trait]
impl Action for DelayAction {
    fn action_type(&self) -> &'static str {
        "delay"
    }

    async fn execute(&self, _context: &dyn ActionContext) -> Result<ActionResult> {
        tokio::time::sleep(self.duration).await;
        Ok(ActionResult::Completed)
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

/// A log action that logs a message
#[derive(Debug, Clone)]
pub struct LogAction {
    /// Action ID
    id: String,
    /// Action description
    description: Option<String>,
    /// Log level
    level: LogLevel,
    /// Message to log
    message: String,
    /// Additional values to include
    values: HashMap<String, Value>,
}

/// Log levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    /// Debug level
    Debug,
    /// Info level
    Info,
    /// Warning level
    Warn,
    /// Error level
    Error,
}

impl LogAction {
    /// Create a new log action
    pub fn new<S1: Into<String>, S2: Into<String>>(id: S1, level: LogLevel, message: S2) -> Self {
        Self {
            id: id.into(),
            description: None,
            level,
            message: message.into(),
            values: HashMap::new(),
        }
    }

    /// Set the action description
    pub fn with_description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add a value to include in the log
    pub fn add_value<K: Into<String>, V: Into<Value>>(mut self, key: K, value: V) -> Self {
        self.values.insert(key.into(), value.into());
        self
    }
}

#[async_trait]
impl Action for LogAction {
    fn action_type(&self) -> &'static str {
        "log"
    }

    async fn execute(&self, context: &dyn ActionContext) -> Result<ActionResult> {
        // Format message with workflow info
        let message = format!(
            "[Workflow: {}] {}",
            context.workflow_name(),
            self.message
        );

        // Log message with appropriate level
        match self.level {
            LogLevel::Debug => debug!("{} {:?}", message, self.values),
            LogLevel::Info => info!("{} {:?}", message, self.values),
            LogLevel::Warn => warn!("{} {:?}", message, self.values),
            LogLevel::Error => error!("{} {:?}", message, self.values),
        }

        Ok(ActionResult::Completed)
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

/// A notification action that sends a notification
#[derive(Debug, Clone)]
pub struct NotificationAction {
    /// Action ID
    id: String,
    /// Action description
    description: Option<String>,
    /// Notification title
    title: String,
    /// Notification message
    message: String,
    /// Notification level
    level: NotificationLevel,
    /// Target recipients
    recipients: Vec<String>,
    /// Additional data
    data: HashMap<String, Value>,
}

/// Notification levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationLevel {
    /// Info level
    Info,
    /// Warning level
    Warning,
    /// Error level
    Error,
    /// Critical level
    Critical,
}

impl NotificationAction {
    /// Create a new notification action
    pub fn new<S1: Into<String>, S2: Into<String>, S3: Into<String>>(
        id: S1,
        title: S2,
        message: S3,
        level: NotificationLevel,
    ) -> Self {
        Self {
            id: id.into(),
            description: None,
            title: title.into(),
            message: message.into(),
            level,
            recipients: Vec::new(),
            data: HashMap::new(),
        }
    }

    /// Set the action description
    pub fn with_description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add a recipient
    pub fn add_recipient<S: Into<String>>(mut self, recipient: S) -> Self {
        self.recipients.push(recipient.into());
        self
    }

    /// Add additional data
    pub fn add_data<K: Into<String>, V: Into<Value>>(mut self, key: K, value: V) -> Self {
        self.data.insert(key.into(), value.into());
        self
    }
}

#[async_trait]
impl Action for NotificationAction {
    fn action_type(&self) -> &'static str {
        "notification"
    }

    async fn execute(&self, context: &dyn ActionContext) -> Result<ActionResult> {
        // For now, just log the notification
        // In a real implementation, this would send the notification to the appropriate service

        info!(
            "Notification from workflow {}: [{}] {} - {} (Level: {:?}, Recipients: {:?}, Data: {:?})",
            context.workflow_name(),
            self.id,
            self.title,
            self.message,
            self.level,
            self.recipients,
            self.data
        );

        Ok(ActionResult::Completed)
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

/// A composite action that executes multiple actions
#[derive(Debug, Clone)]
pub struct CompositeAction {
    /// Action ID
    id: String,
    /// Action description
    description: Option<String>,
    /// Actions to execute
    actions: Vec<Arc<dyn Action>>,
    /// Whether to continue on failure
    continue_on_failure: bool,
}

impl CompositeAction {
    /// Create a new composite action
    pub fn new<S: Into<String>>(id: S) -> Self {
        Self {
            id: id.into(),
            description: None,
            actions: Vec::new(),
            continue_on_failure: false,
        }
    }

    /// Set the action description
    pub fn with_description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add an action
    pub fn add_action<A: Action + 'static>(mut self, action: A) -> Self {
        self.actions.push(Arc::new(action));
        self
    }

    /// Set whether to continue on failure
    pub fn continue_on_failure(mut self, continue_on_failure: bool) -> Self {
        self.continue_on_failure = continue_on_failure;
        self
    }
}

#[async_trait]
impl Action for CompositeAction {
    fn action_type(&self) -> &'static str {
        "composite"
    }

    async fn execute(&self, context: &dyn ActionContext) -> Result<ActionResult> {
        let mut results = Vec::new();
        let mut success = true;

        for action in &self.actions {
            match action.execute(context).await {
                Ok(result) => {
                    if result.is_failure() {
                        success = false;
                        if !self.continue_on_failure {
                            return Ok(result);
                        }
                    }
                    results.push(result);
                }
                Err(e) => {
                    success = false;
                    if !self.continue_on_failure {
                        return Err(e);
                    }
                    results.push(ActionResult::Failure(e.to_string()));
                }
            }
        }

        if success {
            Ok(ActionResult::Completed)
        } else {
            let errors: Vec<String> = results
                .iter()
                .filter_map(|r| match r {
                    ActionResult::Failure(msg) => Some(msg.clone()),
                    _ => None,
                })
                .collect();

            Ok(ActionResult::Failure(format!(
                "One or more actions failed: {}",
                errors.join(", ")
            )))
        }
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

/// A custom action with custom execution logic
#[derive(Debug, Clone)]
pub struct CustomAction<F>
where
    F: Fn(&dyn ActionContext) -> Result<ActionResult> + Send + Sync + 'static,
{
    /// Action ID
    id: String,
    /// Action description
    description: Option<String>,
    /// Custom execute function
    execute_fn: F,
}

impl<F> CustomAction<F>
where
    F: Fn(&dyn ActionContext) -> Result<ActionResult> + Send + Sync + 'static,
{
    /// Create a new custom action
    pub fn new<S: Into<String>>(id: S, execute_fn: F) -> Self {
        Self {
            id: id.into(),
            description: None,
            execute_fn,
        }
    }

    /// Set the action description
    pub fn with_description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }
}

#[async_trait]
impl<F> Action for CustomAction<F>
where
    F: Fn(&dyn ActionContext) -> Result<ActionResult> + Send + Sync + 'static,
{
    fn action_type(&self) -> &'static str {
        "custom"
    }

    async fn execute(&self, context: &dyn ActionContext) -> Result<ActionResult> {
        (self.execute_fn)(context)
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}
