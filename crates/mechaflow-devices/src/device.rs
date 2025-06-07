/*!
 * Device trait and core device abstractions.
 *
 * This module defines the core device traits and abstractions used throughout
 * the MechaFlow device ecosystem.
 */
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::RwLock;

use mechaflow_core::{
    error::Error as CoreError,
    event::{EventBus, SharedEventBus},
    types::{Id, Metadata, Value},
};

/// Error type for device operations
#[derive(Error, Debug)]
pub enum DeviceError {
    /// The device is not connected
    #[error("Device not connected")]
    NotConnected,

    /// The property is not supported by the device
    #[error("Property not supported: {0}")]
    PropertyNotSupported(String),

    /// The value type is not valid for the property
    #[error("Invalid value type for property {0}: expected {1}, got {2}")]
    InvalidValueType(String, String, String),

    /// The value is out of range for the property
    #[error("Value out of range for property {0}: {1}")]
    ValueOutOfRange(String, String),

    /// The device is in an invalid state for the operation
    #[error("Invalid device state: {0}")]
    InvalidState(String),

    /// Communication error with the device
    #[error("Communication error: {0}")]
    CommunicationError(String),

    /// Protocol-specific error
    #[error("Protocol error: {0}")]
    ProtocolError(String),

    /// Timeout error
    #[error("Timeout error: {0}")]
    Timeout(String),

    /// Permission error
    #[error("Permission error: {0}")]
    PermissionDenied(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    /// Other error
    #[error("Other error: {0}")]
    Other(String),

    /// Core error
    #[error("Core error: {0}")]
    CoreError(#[from] CoreError),
}

/// Result type for device operations
pub type Result<T> = std::result::Result<T, DeviceError>;

/// Device capability
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeviceCapability {
    /// The device can be read
    Read,
    /// The device can be written to
    Write,
    /// The device supports streaming data
    Stream,
    /// The device supports commands
    Command,
    /// The device supports configuration
    Configure,
    /// The device supports discovery
    Discover,
    /// The device supports events
    Events,
    /// The device supports direct memory access
    DirectMemoryAccess,
    /// The device supports metadata
    Metadata,
    /// Custom capability
    Custom(String),
}

/// Device state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeviceState {
    /// The device is disconnected
    Disconnected,
    /// The device is connecting
    Connecting,
    /// The device is connected
    Connected,
    /// The device is initializing
    Initializing,
    /// The device is ready for use
    Ready,
    /// The device is in an error state
    Error,
    /// The device is busy
    Busy,
    /// The device is shutting down
    ShuttingDown,
}

/// Device event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceEvent {
    /// The device state has changed
    StateChanged {
        /// The device ID
        device_id: Id,
        /// The old state
        old_state: DeviceState,
        /// The new state
        new_state: DeviceState,
    },
    /// A property value has changed
    PropertyChanged {
        /// The device ID
        device_id: Id,
        /// The property name
        property: String,
        /// The old value
        old_value: Option<Value>,
        /// The new value
        new_value: Value,
    },
    /// A command has been received
    CommandReceived {
        /// The device ID
        device_id: Id,
        /// The command name
        command: String,
        /// The command parameters
        parameters: HashMap<String, Value>,
    },
    /// An error has occurred
    Error {
        /// The device ID
        device_id: Id,
        /// The error message
        message: String,
    },
}

/// Device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// The device ID
    pub id: Id,
    /// The device name
    pub name: String,
    /// The device type
    pub device_type: String,
    /// The device manufacturer
    pub manufacturer: Option<String>,
    /// The device model
    pub model: Option<String>,
    /// The device serial number
    pub serial_number: Option<String>,
    /// The device firmware version
    pub firmware_version: Option<String>,
    /// The device protocol
    pub protocol: String,
    /// The device capabilities
    pub capabilities: Vec<DeviceCapability>,
    /// Additional device metadata
    pub metadata: Metadata,
}

/// The core device trait
///
/// This trait defines the interface for all devices in MechaFlow.
#[async_trait]
pub trait Device: Send + Sync + Debug {
    /// Get the device information
    fn info(&self) -> &DeviceInfo;

    /// Get the device ID
    fn id(&self) -> &Id {
        &self.info().id
    }

    /// Get the device name
    fn name(&self) -> &str {
        &self.info().name
    }

    /// Get the device type
    fn device_type(&self) -> &str {
        &self.info().device_type
    }

    /// Get the device protocol
    fn protocol(&self) -> &str {
        &self.info().protocol
    }

    /// Get the device capabilities
    fn capabilities(&self) -> &[DeviceCapability] {
        &self.info().capabilities
    }

    /// Check if the device has a specific capability
    fn has_capability(&self, capability: &DeviceCapability) -> bool {
        self.capabilities().contains(capability)
    }

    /// Get the device state
    async fn state(&self) -> DeviceState;

    /// Connect to the device
    async fn connect(&self) -> Result<()>;

    /// Disconnect from the device
    async fn disconnect(&self) -> Result<()>;

    /// Initialize the device
    async fn initialize(&self) -> Result<()>;

    /// Shutdown the device
    async fn shutdown(&self) -> Result<()>;

    /// Get available properties for the device
    async fn available_properties(&self) -> Result<Vec<String>>;

    /// Check if a property is supported by the device
    async fn supports_property(&self, property: &str) -> Result<bool> {
        Ok(self.available_properties().await?.contains(&property.to_string()))
    }

    /// Read a property value from the device
    async fn read(&self, property: &str) -> Result<Value>;

    /// Write a property value to the device
    async fn write(&self, property: &str, value: Value) -> Result<()>;

    /// Execute a command on the device
    async fn execute_command(&self, command: &str, parameters: HashMap<String, Value>) -> Result<Value>;

    /// Get available commands for the device
    async fn available_commands(&self) -> Result<Vec<String>>;

    /// Check if a command is supported by the device
    async fn supports_command(&self, command: &str) -> Result<bool> {
        Ok(self.available_commands().await?.contains(&command.to_string()))
    }

    /// Get the device metadata
    async fn get_metadata(&self) -> Result<Metadata>;

    /// Set the device metadata
    async fn set_metadata(&self, metadata: Metadata) -> Result<()>;

    /// Subscribe to device events
    ///
    /// This method should be implemented by devices that support the Events capability.
    /// It returns an EventBus that can be used to subscribe to device events.
    async fn subscribe_events(&self) -> Result<SharedEventBus> {
        Err(DeviceError::PropertyNotSupported("subscribe_events".to_string()))
    }
}

/// Property metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyMetadata {
    /// The property name
    pub name: String,
    /// The property description
    pub description: Option<String>,
    /// The property data type
    pub data_type: PropertyDataType,
    /// The property access mode
    pub access: PropertyAccess,
    /// The property unit
    pub unit: Option<String>,
    /// The property minimum value
    pub min_value: Option<Value>,
    /// The property maximum value
    pub max_value: Option<Value>,
    /// The property precision
    pub precision: Option<f64>,
    /// The property scale factor
    pub scale_factor: Option<f64>,
    /// The property update rate in milliseconds
    pub update_rate_ms: Option<u64>,
    /// Additional property metadata
    pub metadata: Metadata,
}

/// Property data type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PropertyDataType {
    /// Boolean
    Boolean,
    /// Integer
    Integer,
    /// Float
    Float,
    /// String
    String,
    /// Binary
    Binary,
    /// Enum
    Enum(Vec<String>),
    /// Array
    Array(Box<PropertyDataType>),
    /// Object
    Object,
    /// Custom
    Custom(String),
}

/// Property access mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PropertyAccess {
    /// Read-only property
    ReadOnly,
    /// Write-only property
    WriteOnly,
    /// Read-write property
    ReadWrite,
}

/// Base implementation for devices
///
/// This struct provides a base implementation for the Device trait
/// that can be used as a starting point for device implementations.
#[derive(Debug)]
pub struct BaseDevice {
    /// Device information
    info: DeviceInfo,
    /// Device state
    state: RwLock<DeviceState>,
    /// Device properties
    properties: RwLock<HashMap<String, Value>>,
    /// Property metadata
    property_metadata: RwLock<HashMap<String, PropertyMetadata>>,
    /// Device metadata
    metadata: RwLock<Metadata>,
    /// Device event bus
    event_bus: SharedEventBus,
    /// Last connected time
    last_connected: RwLock<Option<Instant>>,
    /// Connection timeout
    connection_timeout: Duration,
}

impl BaseDevice {
    /// Create a new base device
    pub fn new(info: DeviceInfo) -> Self {
        Self {
            info,
            state: RwLock::new(DeviceState::Disconnected),
            properties: RwLock::new(HashMap::new()),
            property_metadata: RwLock::new(HashMap::new()),
            metadata: RwLock::new(HashMap::new()),
            event_bus: Arc::new(EventBus::new()),
            last_connected: RwLock::new(None),
            connection_timeout: Duration::from_secs(30),
        }
    }

    /// Set the device state
    pub async fn set_state(&self, new_state: DeviceState) {
        let mut state = self.state.write().await;
        let old_state = *state;
        *state = new_state;

        if old_state != new_state {
            let event = DeviceEvent::StateChanged {
                device_id: self.info.id.clone(),
                old_state,
                new_state,
            };

            // Publish event
            let _ = self.event_bus.publish(event);
        }
    }

    /// Set a property value
    pub async fn set_property(&self, property: &str, value: Value) -> Result<()> {
        let mut properties = self.properties.write().await;
        let old_value = properties.get(property).cloned();
        properties.insert(property.to_string(), value.clone());

        let event = DeviceEvent::PropertyChanged {
            device_id: self.info.id.clone(),
            property: property.to_string(),
            old_value,
            new_value: value,
        };

        // Publish event
        let _ = self.event_bus.publish(event);

        Ok(())
    }

    /// Get a property value
    pub async fn get_property(&self, property: &str) -> Result<Value> {
        let properties = self.properties.read().await;
        properties
            .get(property)
            .cloned()
            .ok_or_else(|| DeviceError::PropertyNotSupported(property.to_string()))
    }

    /// Set the property metadata
    pub async fn set_property_metadata(&self, metadata: PropertyMetadata) -> Result<()> {
        let mut property_metadata = self.property_metadata.write().await;
        property_metadata.insert(metadata.name.clone(), metadata);
        Ok(())
    }

    /// Get the property metadata
    pub async fn get_property_metadata(&self, property: &str) -> Result<PropertyMetadata> {
        let property_metadata = self.property_metadata.read().await;
        property_metadata
            .get(property)
            .cloned()
            .ok_or_else(|| DeviceError::PropertyNotSupported(property.to_string()))
    }

    /// Check if value type matches property data type
    pub fn validate_value_type(&self, metadata: &PropertyMetadata, value: &Value) -> Result<()> {
        match (&metadata.data_type, value) {
            (PropertyDataType::Boolean, Value::Bool(_)) => Ok(()),
            (PropertyDataType::Integer, Value::Integer(_)) => Ok(()),
            (PropertyDataType::Float, Value::Float(_)) => Ok(()),
            (PropertyDataType::String, Value::String(_)) => Ok(()),
            (PropertyDataType::Binary, Value::Binary(_)) => Ok(()),
            (PropertyDataType::Array(_), Value::Array(_)) => Ok(()),
            (PropertyDataType::Object, Value::Object(_)) => Ok(()),
            (PropertyDataType::Enum(variants), Value::String(s)) => {
                if variants.contains(s) {
                    Ok(())
                } else {
                    Err(DeviceError::InvalidValueType(
                        metadata.name.clone(),
                        format!("enum with variants {:?}", variants),
                        s.clone(),
                    ))
                }
            }
            _ => Err(DeviceError::InvalidValueType(
                metadata.name.clone(),
                format!("{:?}", metadata.data_type),
                format!("{:?}", value),
            )),
        }
    }

    /// Validate value against property constraints
    pub fn validate_value(&self, metadata: &PropertyMetadata, value: &Value) -> Result<()> {
        // Check value type
        self.validate_value_type(metadata, value)?;

        // Check min/max constraints
        match (value, &metadata.min_value, &metadata.max_value) {
            (Value::Integer(i), Some(Value::Integer(min)), _) if i < min => {
                return Err(DeviceError::ValueOutOfRange(
                    metadata.name.clone(),
                    format!("Value {} is less than minimum {}", i, min),
                ));
            }
            (Value::Integer(i), _, Some(Value::Integer(max))) if i > max => {
                return Err(DeviceError::ValueOutOfRange(
                    metadata.name.clone(),
                    format!("Value {} is greater than maximum {}", i, max),
                ));
            }
            (Value::Float(f), Some(Value::Float(min)), _) if f < min => {
                return Err(DeviceError::ValueOutOfRange(
                    metadata.name.clone(),
                    format!("Value {} is less than minimum {}", f, min),
                ));
            }
            (Value::Float(f), _, Some(Value::Float(max))) if f > max => {
                return Err(DeviceError::ValueOutOfRange(
                    metadata.name.clone(),
                    format!("Value {} is greater than maximum {}", f, max),
                ));
            }
            _ => {}
        }

        Ok(())
    }

    /// Publish a device error event
    pub fn publish_error(&self, message: String) {
        let event = DeviceEvent::Error {
            device_id: self.info.id.clone(),
            message,
        };

        let _ = self.event_bus.publish(event);
    }
}

#[async_trait]
impl Device for BaseDevice {
    fn info(&self) -> &DeviceInfo {
        &self.info
    }

    async fn state(&self) -> DeviceState {
        *self.state.read().await
    }

    async fn connect(&self) -> Result<()> {
        let current_state = *self.state.read().await;
        match current_state {
            DeviceState::Connected | DeviceState::Ready => {
                return Ok(());
            }
            DeviceState::Connecting => {
                return Err(DeviceError::InvalidState(
                    "Device is already connecting".to_string(),
                ));
            }
            _ => {}
        }

        self.set_state(DeviceState::Connecting).await;
        *self.last_connected.write().await = Some(Instant::now());
        self.set_state(DeviceState::Connected).await;
        Ok(())
    }

    async fn disconnect(&self) -> Result<()> {
        let current_state = *self.state.read().await;
        match current_state {
            DeviceState::Disconnected => {
                return Ok(());
            }
            DeviceState::ShuttingDown => {
                return Err(DeviceError::InvalidState(
                    "Device is already shutting down".to_string(),
                ));
            }
            _ => {}
        }

        self.set_state(DeviceState::ShuttingDown).await;
        self.set_state(DeviceState::Disconnected).await;
        Ok(())
    }

    async fn initialize(&self) -> Result<()> {
        let current_state = *self.state.read().await;
        match current_state {
            DeviceState::Ready => {
                return Ok(());
            }
            DeviceState::Initializing => {
                return Err(DeviceError::InvalidState(
                    "Device is already initializing".to_string(),
                ));
            }
            DeviceState::Disconnected => {
                return Err(DeviceError::NotConnected);
            }
            _ => {}
        }

        self.set_state(DeviceState::Initializing).await;
        self.set_state(DeviceState::Ready).await;
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        self.disconnect().await
    }

    async fn available_properties(&self) -> Result<Vec<String>> {
        let properties = self.property_metadata.read().await;
        Ok(properties.keys().cloned().collect())
    }

    async fn read(&self, property: &str) -> Result<Value> {
        // Check if device is in a valid state for reading
        let state = *self.state.read().await;
        match state {
            DeviceState::Ready => {}
            DeviceState::Connected => {}
            _ => {
                return Err(DeviceError::InvalidState(format!(
                    "Device is not ready for reading: {:?}",
                    state
                )));
            }
        }

        // Check if property exists
        let property_metadata = self.property_metadata.read().await;
        let metadata = property_metadata
            .get(property)
            .ok_or_else(|| DeviceError::PropertyNotSupported(property.to_string()))?;

        // Check if property is readable
        match metadata.access {
            PropertyAccess::ReadOnly | PropertyAccess::ReadWrite => {}
            PropertyAccess::WriteOnly => {
                return Err(DeviceError::PermissionDenied(format!(
                    "Property {} is write-only",
                    property
                )));
            }
        }

        self.get_property(property).await
    }

    async fn write(&self, property: &str, value: Value) -> Result<()> {
        // Check if device is in a valid state for writing
        let state = *self.state.read().await;
        match state {
            DeviceState::Ready => {}
            DeviceState::Connected => {}
            _ => {
                return Err(DeviceError::InvalidState(format!(
                    "Device is not ready for writing: {:?}",
                    state
                )));
            }
        }

        // Check if property exists and validate value
        let property_metadata = self.property_metadata.read().await;
        let metadata = property_metadata
            .get(property)
            .ok_or_else(|| DeviceError::PropertyNotSupported(property.to_string()))?;

        // Check if property is writable
        match metadata.access {
            PropertyAccess::WriteOnly | PropertyAccess::ReadWrite => {}
            PropertyAccess::ReadOnly => {
                return Err(DeviceError::PermissionDenied(format!(
                    "Property {} is read-only",
                    property
                )));
            }
        }

        // Validate value
        self.validate_value(metadata, &value)?;

        // Write the value
        self.set_property(property, value).await
    }

    async fn execute_command(&self, command: &str, parameters: HashMap<String, Value>) -> Result<Value> {
        Err(DeviceError::PropertyNotSupported(format!(
            "Command {} not supported",
            command
        )))
    }

    async fn available_commands(&self) -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    async fn get_metadata(&self) -> Result<Metadata> {
        let metadata = self.metadata.read().await;
        Ok(metadata.clone())
    }

    async fn set_metadata(&self, metadata: Metadata) -> Result<()> {
        let mut current_metadata = self.metadata.write().await;
        *current_metadata = metadata;
        Ok(())
    }

    async fn subscribe_events(&self) -> Result<SharedEventBus> {
        if self.has_capability(&DeviceCapability::Events) {
            Ok(self.event_bus.clone())
        } else {
            Err(DeviceError::PropertyNotSupported("Events".to_string()))
        }
    }
}
