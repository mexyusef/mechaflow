/*!
 * Thermostat Device Implementation for MechaFlow.
 *
 * This module provides an implementation of a thermostat device.
 */

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use mechaflow_core::types::{Id, Value};

use crate::device::{
    BaseDevice, Device, DeviceCapability, DeviceError, DeviceEvent, DeviceInfo, DeviceState, Result,
};
use crate::protocol::{Protocol, ProtocolOptions, ProtocolRegistry};

/// Thermostat operating modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThermostatMode {
    /// Off mode
    Off,
    /// Heat mode
    Heat,
    /// Cool mode
    Cool,
    /// Auto mode (heating or cooling as needed)
    Auto,
    /// Fan only mode
    Fan,
}

impl ThermostatMode {
    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            ThermostatMode::Off => "off",
            ThermostatMode::Heat => "heat",
            ThermostatMode::Cool => "cool",
            ThermostatMode::Auto => "auto",
            ThermostatMode::Fan => "fan",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "off" => Some(ThermostatMode::Off),
            "heat" => Some(ThermostatMode::Heat),
            "cool" => Some(ThermostatMode::Cool),
            "auto" => Some(ThermostatMode::Auto),
            "fan" => Some(ThermostatMode::Fan),
            _ => None,
        }
    }
}

impl From<ThermostatMode> for Value {
    fn from(mode: ThermostatMode) -> Self {
        Value::String(mode.as_str().to_string())
    }
}

impl TryFrom<Value> for ThermostatMode {
    type Error = DeviceError;

    fn try_from(value: Value) -> std::result::Result<Self, Self::Error> {
        match value {
            Value::String(s) => ThermostatMode::from_str(&s).ok_or_else(|| {
                DeviceError::InvalidValue(format!("Invalid thermostat mode: {}", s))
            }),
            _ => Err(DeviceError::InvalidValue(format!(
                "Invalid thermostat mode: {:?}",
                value
            ))),
        }
    }
}

/// Thermostat device implementation
pub struct ThermostatDevice {
    /// Base device implementation
    base: BaseDevice,
    /// Protocol registry for communication
    protocol_registry: Arc<RwLock<ProtocolRegistry>>,
    /// Protocol name
    protocol_name: String,
    /// Connection string
    connection_string: String,
    /// Current temperature
    current_temperature: RwLock<f64>,
    /// Target temperature
    target_temperature: RwLock<f64>,
    /// Current mode
    mode: RwLock<ThermostatMode>,
    /// Fan state
    fan_on: RwLock<bool>,
    /// Humidity
    humidity: RwLock<u8>,
}

impl fmt::Debug for ThermostatDevice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ThermostatDevice")
            .field("id", &self.base.id())
            .field("name", &self.base.info().name)
            .field("protocol", &self.protocol_name)
            .field("connection_string", &self.connection_string)
            .field("current_temperature", &self.current_temperature)
            .field("target_temperature", &self.target_temperature)
            .field("mode", &self.mode)
            .field("fan_on", &self.fan_on)
            .field("humidity", &self.humidity)
            .finish()
    }
}

impl ThermostatDevice {
    /// Create a new thermostat device
    pub fn new(
        id: Option<Id>,
        name: String,
        protocol_registry: Arc<RwLock<ProtocolRegistry>>,
        protocol_name: String,
        connection_string: String,
    ) -> Self {
        let id = id.unwrap_or_else(|| Id::new(Uuid::new_v4().to_string()));

        let info = DeviceInfo {
            id: id.clone(),
            name,
            device_type: "thermostat".to_string(),
            manufacturer: "MechaFlow".to_string(),
            model: "Virtual Thermostat".to_string(),
            protocol: protocol_name.clone(),
            capabilities: vec![
                DeviceCapability::TemperatureSensing,
                DeviceCapability::TemperatureControl,
                DeviceCapability::HumiditySensing,
                DeviceCapability::FanControl,
            ],
        };

        let (event_tx, _) = broadcast::channel(100);

        Self {
            base: BaseDevice::new(id, info, event_tx),
            protocol_registry,
            protocol_name,
            connection_string,
            current_temperature: RwLock::new(22.0),
            target_temperature: RwLock::new(21.0),
            mode: RwLock::new(ThermostatMode::Off),
            fan_on: RwLock::new(false),
            humidity: RwLock::new(50),
        }
    }

    /// Get the protocol for this device
    async fn get_protocol(&self) -> Result<Box<dyn Protocol>> {
        let registry = self.protocol_registry.read().unwrap();

        // Check if the protocol exists
        match registry.get_protocol(&self.protocol_name) {
            Some(protocol) => Ok(Box::new(protocol)),
            None => {
                // Drop the read lock and acquire a write lock to create the protocol
                drop(registry);
                let mut registry = self.protocol_registry.write().unwrap();

                // Get or create the protocol
                let protocol = registry
                    .get_or_create_protocol(&self.protocol_name)
                    .await?;

                Ok(Box::new(protocol))
            }
        }
    }

    /// Get current temperature
    pub fn current_temperature(&self) -> f64 {
        *self.current_temperature.read().unwrap()
    }

    /// Set current temperature
    pub fn set_current_temperature(&self, temp: f64) -> Result<()> {
        if temp < -50.0 || temp > 100.0 {
            return Err(DeviceError::InvalidValue(format!(
                "Temperature out of range: {}",
                temp
            )));
        }

        let mut current = self.current_temperature.write().unwrap();
        *current = temp;

        // Publish temperature changed event
        self.base
            .publish_event(DeviceEvent::PropertyChanged {
                property: "temperature".to_string(),
                value: Value::Float(temp),
            })
            .map_err(|e| {
                DeviceError::Other(format!("Failed to publish temperature event: {}", e))
            })?;

        Ok(())
    }

    /// Get target temperature
    pub fn target_temperature(&self) -> f64 {
        *self.target_temperature.read().unwrap()
    }

    /// Set target temperature
    pub fn set_target_temperature(&self, temp: f64) -> Result<()> {
        if temp < 10.0 || temp > 35.0 {
            return Err(DeviceError::InvalidValue(format!(
                "Target temperature out of range: {}",
                temp
            )));
        }

        let mut target = self.target_temperature.write().unwrap();
        *target = temp;

        // Publish target temperature changed event
        self.base
            .publish_event(DeviceEvent::PropertyChanged {
                property: "targetTemperature".to_string(),
                value: Value::Float(temp),
            })
            .map_err(|e| {
                DeviceError::Other(format!(
                    "Failed to publish target temperature event: {}",
                    e
                ))
            })?;

        // Start temperature control
        self.control_temperature()?;

        Ok(())
    }

    /// Get current mode
    pub fn mode(&self) -> ThermostatMode {
        *self.mode.read().unwrap()
    }

    /// Set mode
    pub fn set_mode(&self, mode: ThermostatMode) -> Result<()> {
        let mut current_mode = self.mode.write().unwrap();
        *current_mode = mode;

        // Publish mode changed event
        self.base
            .publish_event(DeviceEvent::PropertyChanged {
                property: "mode".to_string(),
                value: Value::String(mode.as_str().to_string()),
            })
            .map_err(|e| DeviceError::Other(format!("Failed to publish mode event: {}", e)))?;

        // Start temperature control
        drop(current_mode);
        self.control_temperature()?;

        Ok(())
    }

    /// Get fan state
    pub fn fan_on(&self) -> bool {
        *self.fan_on.read().unwrap()
    }

    /// Set fan state
    pub fn set_fan_on(&self, on: bool) -> Result<()> {
        let mut fan = self.fan_on.write().unwrap();
        *fan = on;

        // Publish fan state changed event
        self.base
            .publish_event(DeviceEvent::PropertyChanged {
                property: "fanOn".to_string(),
                value: Value::Bool(on),
            })
            .map_err(|e| DeviceError::Other(format!("Failed to publish fan event: {}", e)))?;

        Ok(())
    }

    /// Get humidity
    pub fn humidity(&self) -> u8 {
        *self.humidity.read().unwrap()
    }

    /// Set humidity
    pub fn set_humidity(&self, humidity: u8) -> Result<()> {
        if humidity > 100 {
            return Err(DeviceError::InvalidValue(format!(
                "Humidity out of range: {}",
                humidity
            )));
        }

        let mut current = self.humidity.write().unwrap();
        *current = humidity;

        // Publish humidity changed event
        self.base
            .publish_event(DeviceEvent::PropertyChanged {
                property: "humidity".to_string(),
                value: Value::Integer(humidity as i64),
            })
            .map_err(|e| DeviceError::Other(format!("Failed to publish humidity event: {}", e)))?;

        Ok(())
    }

    /// Control temperature based on mode and target
    fn control_temperature(&self) -> Result<()> {
        let mode = self.mode();
        let current = self.current_temperature();
        let target = self.target_temperature();

        let fan_on = match mode {
            ThermostatMode::Off => false,
            ThermostatMode::Fan => true,
            ThermostatMode::Heat => current < target,
            ThermostatMode::Cool => current > target,
            ThermostatMode::Auto => current < target - 0.5 || current > target + 0.5,
        };

        self.set_fan_on(fan_on)?;

        // Update state based on mode and activity
        let state = match mode {
            ThermostatMode::Off => DeviceState::Standby,
            ThermostatMode::Fan => DeviceState::Active,
            ThermostatMode::Heat => {
                if current < target {
                    DeviceState::Active
                } else {
                    DeviceState::Idle
                }
            }
            ThermostatMode::Cool => {
                if current > target {
                    DeviceState::Active
                } else {
                    DeviceState::Idle
                }
            }
            ThermostatMode::Auto => {
                if current < target - 0.5 || current > target + 0.5 {
                    DeviceState::Active
                } else {
                    DeviceState::Idle
                }
            }
        };

        self.base.set_state(state);

        Ok(())
    }

    /// Start the thermostat control loop
    async fn start_control_loop(self: Arc<Self>) -> Result<()> {
        // In a real implementation, this would run a continuous control loop
        // For this example, we'll just simulate some temperature changes

        let device = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));

            loop {
                interval.tick().await;

                // Skip if disconnected
                if device.base.state() != DeviceState::Connected
                    && device.base.state() != DeviceState::Active
                    && device.base.state() != DeviceState::Idle
                {
                    continue;
                }

                // Update temperature based on mode and fan state
                let mode = device.mode();
                let target = device.target_temperature();
                let current = device.current_temperature();
                let fan_on = device.fan_on();

                let new_temp = match mode {
                    ThermostatMode::Off => {
                        // Temperature drifts towards ambient (20°C)
                        if current > 20.0 {
                            current - 0.1
                        } else if current < 20.0 {
                            current + 0.1
                        } else {
                            current
                        }
                    }
                    ThermostatMode::Fan => {
                        // Fan alone doesn't change temperature much
                        current
                    }
                    ThermostatMode::Heat => {
                        if fan_on && current < target {
                            // Heating up
                            current + 0.3
                        } else if current > target {
                            // Cooling down naturally
                            current - 0.1
                        } else {
                            current
                        }
                    }
                    ThermostatMode::Cool => {
                        if fan_on && current > target {
                            // Cooling down
                            current - 0.3
                        } else if current < target {
                            // Warming up naturally
                            current + 0.1
                        } else {
                            current
                        }
                    }
                    ThermostatMode::Auto => {
                        if fan_on {
                            if current < target - 0.5 {
                                // Heating
                                current + 0.3
                            } else if current > target + 0.5 {
                                // Cooling
                                current - 0.3
                            } else {
                                current
                            }
                        } else if current > target {
                            // Cooling down naturally
                            current - 0.1
                        } else if current < target {
                            // Warming up naturally
                            current + 0.1
                        } else {
                            current
                        }
                    }
                };

                // Apply the temperature change
                if let Err(e) = device.set_current_temperature(new_temp) {
                    error!("Failed to update temperature: {}", e);
                }

                // Adjust the state and fan
                if let Err(e) = device.control_temperature() {
                    error!("Failed to control temperature: {}", e);
                }

                // Update humidity with a small random variation
                let current_humidity = device.humidity();
                let humidity_change = (rand::random::<f32>() * 2.0 - 1.0) as i8;
                let new_humidity = (current_humidity as i16 + humidity_change as i16)
                    .clamp(30, 70) as u8;

                if let Err(e) = device.set_humidity(new_humidity) {
                    error!("Failed to update humidity: {}", e);
                }
            }
        });

        Ok(())
    }
}

#[async_trait]
impl Device for ThermostatDevice {
    fn id(&self) -> &Id {
        self.base.id()
    }

    fn info(&self) -> &DeviceInfo {
        self.base.info()
    }

    fn state(&self) -> DeviceState {
        self.base.state()
    }

    fn capabilities(&self) -> &[DeviceCapability] {
        &self.base.info().capabilities
    }

    fn has_capability(&self, capability: DeviceCapability) -> bool {
        self.base.has_capability(capability)
    }

    async fn connect(&self) -> Result<()> {
        info!("Connecting thermostat device: {}", self.base.info().name);

        // Transition to connecting state
        self.base.set_state(DeviceState::Connecting);

        // Get the protocol
        let protocol = self.get_protocol().await?;

        // Connect using the protocol
        protocol
            .connect(&self.connection_string, ProtocolOptions::default())
            .await?;

        // Update state
        self.base.set_state(DeviceState::Connected);

        // Start the control loop
        let device = Arc::new(self.clone());
        device.start_control_loop().await?;

        // Publish connected event
        self.base.publish_event(DeviceEvent::Connected).map_err(|e| {
            DeviceError::Other(format!("Failed to publish connected event: {}", e))
        })?;

        Ok(())
    }

    async fn disconnect(&self) -> Result<()> {
        info!("Disconnecting thermostat device: {}", self.base.info().name);

        // Transition to disconnecting state
        self.base.set_state(DeviceState::Disconnecting);

        // Get the protocol
        let protocol = self.get_protocol().await?;

        // Disconnect using the protocol
        protocol.disconnect(&self.connection_string).await?;

        // Update state
        self.base.set_state(DeviceState::Disconnected);

        // Publish disconnected event
        self.base
            .publish_event(DeviceEvent::Disconnected)
            .map_err(|e| {
                DeviceError::Other(format!("Failed to publish disconnected event: {}", e))
            })?;

        Ok(())
    }

    async fn read_property(&self, name: &str) -> Result<Value> {
        debug!("Reading property {} from thermostat", name);

        match name {
            "temperature" => Ok(Value::Float(self.current_temperature())),
            "targetTemperature" => Ok(Value::Float(self.target_temperature())),
            "mode" => Ok(Value::String(self.mode().as_str().to_string())),
            "fanOn" => Ok(Value::Bool(self.fan_on())),
            "humidity" => Ok(Value::Integer(self.humidity() as i64)),
            _ => {
                // Try to read from the protocol
                let protocol = self.get_protocol().await?;
                protocol
                    .read_property(
                        &self.connection_string,
                        name,
                        ProtocolOptions::default(),
                    )
                    .await
            }
        }
    }

    async fn write_property(&self, name: &str, value: Value) -> Result<()> {
        debug!("Writing property {} = {:?} to thermostat", name, value);

        match name {
            "targetTemperature" => {
                let temp = match value {
                    Value::Float(f) => f,
                    Value::Integer(i) => i as f64,
                    _ => {
                        return Err(DeviceError::InvalidValue(format!(
                            "Invalid temperature value: {:?}",
                            value
                        )))
                    }
                };
                self.set_target_temperature(temp)
            }
            "mode" => {
                let mode = ThermostatMode::try_from(value)?;
                self.set_mode(mode)
            }
            "fanOn" => {
                let on = match value {
                    Value::Bool(b) => b,
                    _ => {
                        return Err(DeviceError::InvalidValue(format!(
                            "Invalid fan state value: {:?}",
                            value
                        )))
                    }
                };
                self.set_fan_on(on)
            }
            _ => {
                // Try to write to the protocol
                let protocol = self.get_protocol().await?;
                protocol
                    .write_property(
                        &self.connection_string,
                        name,
                        value,
                        ProtocolOptions::default(),
                    )
                    .await
            }
        }
    }

    async fn execute_command(
        &self,
        command: &str,
        parameters: HashMap<String, Value>,
    ) -> Result<Value> {
        debug!(
            "Executing command {} with parameters {:?} on thermostat",
            command, parameters
        );

        match command {
            "setTemperature" => {
                if let Some(Value::Float(temp)) = parameters.get("temperature") {
                    self.set_target_temperature(*temp)?;
                    Ok(Value::Bool(true))
                } else if let Some(Value::Integer(temp)) = parameters.get("temperature") {
                    self.set_target_temperature(*temp as f64)?;
                    Ok(Value::Bool(true))
                } else {
                    Err(DeviceError::MissingParameter("temperature".to_string()))
                }
            }
            "setMode" => {
                if let Some(value) = parameters.get("mode") {
                    let mode = ThermostatMode::try_from(value.clone())?;
                    self.set_mode(mode)?;
                    Ok(Value::Bool(true))
                } else {
                    Err(DeviceError::MissingParameter("mode".to_string()))
                }
            }
            "setFan" => {
                if let Some(Value::Bool(on)) = parameters.get("on") {
                    self.set_fan_on(*on)?;
                    Ok(Value::Bool(true))
                } else {
                    Err(DeviceError::MissingParameter("on".to_string()))
                }
            }
            "getStatus" => {
                let status = HashMap::from([
                    (
                        "temperature".to_string(),
                        Value::Float(self.current_temperature()),
                    ),
                    (
                        "targetTemperature".to_string(),
                        Value::Float(self.target_temperature()),
                    ),
                    (
                        "mode".to_string(),
                        Value::String(self.mode().as_str().to_string()),
                    ),
                    ("fanOn".to_string(), Value::Bool(self.fan_on())),
                    (
                        "humidity".to_string(),
                        Value::Integer(self.humidity() as i64),
                    ),
                    (
                        "state".to_string(),
                        Value::String(format!("{:?}", self.state())),
                    ),
                ]);

                Ok(Value::Object(status))
            }
            _ => {
                // Try to execute the command through the protocol
                let protocol = self.get_protocol().await?;
                protocol
                    .execute_command(
                        &self.connection_string,
                        command,
                        parameters,
                        ProtocolOptions::default(),
                    )
                    .await
            }
        }
    }

    async fn subscribe_events(&self) -> Result<broadcast::Receiver<DeviceEvent>> {
        self.base.subscribe_events()
    }
}

impl Clone for ThermostatDevice {
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
            protocol_registry: self.protocol_registry.clone(),
            protocol_name: self.protocol_name.clone(),
            connection_string: self.connection_string.clone(),
            current_temperature: RwLock::new(*self.current_temperature.read().unwrap()),
            target_temperature: RwLock::new(*self.target_temperature.read().unwrap()),
            mode: RwLock::new(*self.mode.read().unwrap()),
            fan_on: RwLock::new(*self.fan_on.read().unwrap()),
            humidity: RwLock::new(*self.humidity.read().unwrap()),
        }
    }
}
