/*!
 * Workflow execution context.
 *
 * This module defines the context in which workflows execute, providing
 * access to devices, variables, and other resources.
 */
use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;

use mechaflow_core::types::{Id, Value};
use mechaflow_devices::device::Device;
use mechaflow_devices::registry::DeviceRegistry;

use crate::error::Result;

/// Context for workflow execution
#[async_trait]
pub trait WorkflowContext: Send + Sync {
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

    /// Get all variables
    fn get_variables(&self) -> Result<HashMap<String, Value>>;
}

/// A basic workflow context implementation
#[derive(Debug)]
pub struct BasicWorkflowContext {
    /// Device registry
    device_registry: Arc<DeviceRegistry>,
    /// Variables
    variables: tokio::sync::RwLock<HashMap<String, Value>>,
}

impl BasicWorkflowContext {
    /// Create a new basic workflow context
    pub fn new(device_registry: Arc<DeviceRegistry>) -> Self {
        Self {
            device_registry,
            variables: tokio::sync::RwLock::new(HashMap::new()),
        }
    }

    /// Create a new basic workflow context with initial variables
    pub fn with_variables(
        device_registry: Arc<DeviceRegistry>,
        variables: HashMap<String, Value>,
    ) -> Self {
        Self {
            device_registry,
            variables: tokio::sync::RwLock::new(variables),
        }
    }

    /// Initialize the context with variables
    pub async fn init_variables(&self, variables: HashMap<String, Value>) {
        let mut var_lock = self.variables.write().await;
        *var_lock = variables;
    }

    /// Clear all variables
    pub async fn clear_variables(&self) {
        let mut var_lock = self.variables.write().await;
        var_lock.clear();
    }
}

#[async_trait]
impl WorkflowContext for BasicWorkflowContext {
    fn read_property(&self, device_id: &Id, property: &str) -> Result<Value> {
        let device = self.device_registry.get_device(device_id)?;
        device.read_property(property)
    }

    fn write_property(&self, device_id: &Id, property: &str, value: Value) -> Result<()> {
        let device = self.device_registry.get_device(device_id)?;
        device.write_property(property, value)
    }

    fn execute_command(
        &self,
        device_id: &Id,
        command: &str,
        parameters: HashMap<String, Value>,
    ) -> Result<Value> {
        let device = self.device_registry.get_device(device_id)?;
        device.execute_command(command, parameters)
    }

    fn get_device(&self, device_id: &Id) -> Result<Arc<dyn Device>> {
        self.device_registry.get_device(device_id)
    }

    fn get_variable(&self, name: &str) -> Result<Value> {
        let variables = self.variables.blocking_read();
        match variables.get(name) {
            Some(value) => Ok(value.clone()),
            None => Ok(Value::Null),
        }
    }

    fn set_variable(&self, name: &str, value: Value) -> Result<()> {
        let mut variables = self.variables.blocking_write();
        variables.insert(name.to_string(), value);
        Ok(())
    }

    fn get_variables(&self) -> Result<HashMap<String, Value>> {
        Ok(self.variables.blocking_read().clone())
    }
}

/// A workflow context with access to the workflow's previous state
#[derive(Debug)]
pub struct StatefulWorkflowContext {
    /// Inner context
    inner: Arc<dyn WorkflowContext>,
    /// Previous state
    previous_state: HashMap<String, Value>,
}

impl StatefulWorkflowContext {
    /// Create a new stateful workflow context
    pub fn new(inner: Arc<dyn WorkflowContext>, previous_state: HashMap<String, Value>) -> Self {
        Self {
            inner,
            previous_state,
        }
    }

    /// Get the previous state
    pub fn previous_state(&self) -> &HashMap<String, Value> {
        &self.previous_state
    }

    /// Get a value from the previous state
    pub fn get_previous_state(&self, name: &str) -> Option<&Value> {
        self.previous_state.get(name)
    }
}

#[async_trait]
impl WorkflowContext for StatefulWorkflowContext {
    fn read_property(&self, device_id: &Id, property: &str) -> Result<Value> {
        self.inner.read_property(device_id, property)
    }

    fn write_property(&self, device_id: &Id, property: &str, value: Value) -> Result<()> {
        self.inner.write_property(device_id, property, value)
    }

    fn execute_command(
        &self,
        device_id: &Id,
        command: &str,
        parameters: HashMap<String, Value>,
    ) -> Result<Value> {
        self.inner.execute_command(device_id, command, parameters)
    }

    fn get_device(&self, device_id: &Id) -> Result<Arc<dyn Device>> {
        self.inner.get_device(device_id)
    }

    fn get_variable(&self, name: &str) -> Result<Value> {
        self.inner.get_variable(name)
    }

    fn set_variable(&self, name: &str, value: Value) -> Result<()> {
        self.inner.set_variable(name, value)
    }

    fn get_variables(&self) -> Result<HashMap<String, Value>> {
        self.inner.get_variables()
    }
}

/// A sandbox workflow context that simulates operations
#[derive(Debug)]
pub struct SandboxWorkflowContext {
    /// Device properties
    device_properties: tokio::sync::RwLock<HashMap<Id, HashMap<String, Value>>>,
    /// Variables
    variables: tokio::sync::RwLock<HashMap<String, Value>>,
    /// Operations log
    operations_log: tokio::sync::RwLock<Vec<SandboxOperation>>,
}

/// An operation performed in the sandbox
#[derive(Debug, Clone)]
pub enum SandboxOperation {
    /// Read property
    ReadProperty {
        /// Device ID
        device_id: Id,
        /// Property name
        property: String,
        /// Value read
        value: Value,
    },
    /// Write property
    WriteProperty {
        /// Device ID
        device_id: Id,
        /// Property name
        property: String,
        /// Value written
        value: Value,
    },
    /// Execute command
    ExecuteCommand {
        /// Device ID
        device_id: Id,
        /// Command name
        command: String,
        /// Command parameters
        parameters: HashMap<String, Value>,
        /// Command result
        result: Value,
    },
    /// Get variable
    GetVariable {
        /// Variable name
        name: String,
        /// Variable value
        value: Value,
    },
    /// Set variable
    SetVariable {
        /// Variable name
        name: String,
        /// Variable value
        value: Value,
    },
}

impl SandboxWorkflowContext {
    /// Create a new sandbox workflow context
    pub fn new() -> Self {
        Self {
            device_properties: tokio::sync::RwLock::new(HashMap::new()),
            variables: tokio::sync::RwLock::new(HashMap::new()),
            operations_log: tokio::sync::RwLock::new(Vec::new()),
        }
    }

    /// Set initial device properties
    pub async fn set_device_properties(
        &self,
        device_id: Id,
        properties: HashMap<String, Value>,
    ) {
        let mut device_properties = self.device_properties.write().await;
        device_properties.insert(device_id, properties);
    }

    /// Set initial variables
    pub async fn set_variables(&self, variables: HashMap<String, Value>) {
        let mut var_lock = self.variables.write().await;
        *var_lock = variables;
    }

    /// Get the operations log
    pub async fn get_operations_log(&self) -> Vec<SandboxOperation> {
        self.operations_log.read().await.clone()
    }

    /// Clear the operations log
    pub async fn clear_operations_log(&self) {
        let mut log = self.operations_log.write().await;
        log.clear();
    }
}

#[async_trait]
impl WorkflowContext for SandboxWorkflowContext {
    fn read_property(&self, device_id: &Id, property: &str) -> Result<Value> {
        let device_properties = self.device_properties.blocking_read();

        let value = match device_properties.get(device_id) {
            Some(props) => props.get(property).cloned().unwrap_or(Value::Null),
            None => Value::Null,
        };

        // Log the operation
        let mut log = self.operations_log.blocking_write();
        log.push(SandboxOperation::ReadProperty {
            device_id: device_id.clone(),
            property: property.to_string(),
            value: value.clone(),
        });

        Ok(value)
    }

    fn write_property(&self, device_id: &Id, property: &str, value: Value) -> Result<()> {
        let mut device_properties = self.device_properties.blocking_write();

        // Get or create the device properties map
        let props = device_properties
            .entry(device_id.clone())
            .or_insert_with(HashMap::new);

        // Set the property
        props.insert(property.to_string(), value.clone());

        // Log the operation
        let mut log = self.operations_log.blocking_write();
        log.push(SandboxOperation::WriteProperty {
            device_id: device_id.clone(),
            property: property.to_string(),
            value,
        });

        Ok(())
    }

    fn execute_command(
        &self,
        device_id: &Id,
        command: &str,
        parameters: HashMap<String, Value>,
    ) -> Result<Value> {
        // In a sandbox, we just return a dummy result
        let result = Value::Object(parameters.clone());

        // Log the operation
        let mut log = self.operations_log.blocking_write();
        log.push(SandboxOperation::ExecuteCommand {
            device_id: device_id.clone(),
            command: command.to_string(),
            parameters,
            result: result.clone(),
        });

        Ok(result)
    }

    fn get_device(&self, _device_id: &Id) -> Result<Arc<dyn Device>> {
        // In a sandbox, we don't return actual devices
        Err(crate::error::Error::not_found("Device not available in sandbox mode"))
    }

    fn get_variable(&self, name: &str) -> Result<Value> {
        let variables = self.variables.blocking_read();
        let value = variables.get(name).cloned().unwrap_or(Value::Null);

        // Log the operation
        let mut log = self.operations_log.blocking_write();
        log.push(SandboxOperation::GetVariable {
            name: name.to_string(),
            value: value.clone(),
        });

        Ok(value)
    }

    fn set_variable(&self, name: &str, value: Value) -> Result<()> {
        let mut variables = self.variables.blocking_write();
        variables.insert(name.to_string(), value.clone());

        // Log the operation
        let mut log = self.operations_log.blocking_write();
        log.push(SandboxOperation::SetVariable {
            name: name.to_string(),
            value,
        });

        Ok(())
    }

    fn get_variables(&self) -> Result<HashMap<String, Value>> {
        Ok(self.variables.blocking_read().clone())
    }
}
