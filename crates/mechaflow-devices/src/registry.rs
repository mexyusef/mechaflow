/*!
 * Device registry for MechaFlow.
 *
 * This module provides a registry for managing devices in the MechaFlow system.
 */
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use mechaflow_core::types::Id;

use crate::device::{Device, DeviceCapability, DeviceInfo, DeviceState};
use crate::device::DeviceError;

/// Event types for device registry
#[derive(Debug, Clone)]
pub enum RegistryEvent {
    /// A device was added to the registry
    DeviceAdded(DeviceInfo),
    /// A device was removed from the registry
    DeviceRemoved(Id),
    /// A device state changed
    DeviceStateChanged {
        /// The device ID
        id: Id,
        /// The old state
        old_state: DeviceState,
        /// The new state
        new_state: DeviceState,
    },
}

/// Device registry
#[derive(Debug)]
pub struct DeviceRegistry {
    /// The registered devices
    devices: RwLock<HashMap<Id, Arc<dyn Device>>>,
    /// Event sender for registry events
    event_sender: broadcast::Sender<RegistryEvent>,
}

impl DeviceRegistry {
    /// Create a new device registry
    pub fn new() -> Self {
        let (event_sender, _) = broadcast::channel(100);
        Self {
            devices: RwLock::new(HashMap::new()),
            event_sender,
        }
    }

    /// Register a device with the registry
    pub fn register_device<D: Device + 'static>(&self, device: D) -> Result<(), DeviceError> {
        let id = device.id().clone();
        let info = device.info().clone();
        let device = Arc::new(device);

        let mut devices = self.devices.write().map_err(|_| {
            DeviceError::Other("Failed to acquire write lock on device registry".to_string())
        })?;

        if devices.contains_key(&id) {
            return Err(DeviceError::Other(format!(
                "Device with ID {} already registered",
                id
            )));
        }

        devices.insert(id.clone(), device);
        let _ = self.event_sender.send(RegistryEvent::DeviceAdded(info));
        debug!("Registered device with ID {}", id);

        Ok(())
    }

    /// Unregister a device from the registry
    pub fn unregister_device(&self, id: &Id) -> Result<(), DeviceError> {
        let mut devices = self.devices.write().map_err(|_| {
            DeviceError::Other("Failed to acquire write lock on device registry".to_string())
        })?;

        if devices.remove(id).is_none() {
            return Err(DeviceError::Other(format!(
                "Device with ID {} not registered",
                id
            )));
        }

        let _ = self.event_sender.send(RegistryEvent::DeviceRemoved(id.clone()));
        debug!("Unregistered device with ID {}", id);

        Ok(())
    }

    /// Get a device by ID
    pub fn get_device(&self, id: &Id) -> Result<Arc<dyn Device>, DeviceError> {
        let devices = self.devices.read().map_err(|_| {
            DeviceError::Other("Failed to acquire read lock on device registry".to_string())
        })?;

        devices
            .get(id)
            .cloned()
            .ok_or_else(|| DeviceError::Other(format!("Device with ID {} not found", id)))
    }

    /// Get all registered devices
    pub fn get_devices(&self) -> Result<Vec<Arc<dyn Device>>, DeviceError> {
        let devices = self.devices.read().map_err(|_| {
            DeviceError::Other("Failed to acquire read lock on device registry".to_string())
        })?;

        Ok(devices.values().cloned().collect())
    }

    /// Get all device IDs
    pub fn get_device_ids(&self) -> Result<Vec<Id>, DeviceError> {
        let devices = self.devices.read().map_err(|_| {
            DeviceError::Other("Failed to acquire read lock on device registry".to_string())
        })?;

        Ok(devices.keys().cloned().collect())
    }

    /// Get devices by type
    pub fn get_devices_by_type(&self, device_type: &str) -> Result<Vec<Arc<dyn Device>>, DeviceError> {
        let devices = self.devices.read().map_err(|_| {
            DeviceError::Other("Failed to acquire read lock on device registry".to_string())
        })?;

        Ok(devices
            .values()
            .filter(|device| device.device_type() == device_type)
            .cloned()
            .collect())
    }

    /// Get devices by capability
    pub fn get_devices_by_capability(
        &self,
        capability: &DeviceCapability,
    ) -> Result<Vec<Arc<dyn Device>>, DeviceError> {
        let devices = self.devices.read().map_err(|_| {
            DeviceError::Other("Failed to acquire read lock on device registry".to_string())
        })?;

        Ok(devices
            .values()
            .filter(|device| device.has_capability(capability))
            .cloned()
            .collect())
    }

    /// Get devices by protocol
    pub fn get_devices_by_protocol(&self, protocol: &str) -> Result<Vec<Arc<dyn Device>>, DeviceError> {
        let devices = self.devices.read().map_err(|_| {
            DeviceError::Other("Failed to acquire read lock on device registry".to_string())
        })?;

        Ok(devices
            .values()
            .filter(|device| device.protocol() == protocol)
            .cloned()
            .collect())
    }

    /// Subscribe to registry events
    pub fn subscribe(&self) -> broadcast::Receiver<RegistryEvent> {
        self.event_sender.subscribe()
    }

    /// Initialize all devices
    pub async fn initialize_all_devices(&self) -> Result<(), DeviceError> {
        let devices = self.devices.read().map_err(|_| {
            DeviceError::Other("Failed to acquire read lock on device registry".to_string())
        })?;

        for (id, device) in devices.iter() {
            debug!("Initializing device with ID {}", id);
            if let Err(e) = device.initialize().await {
                error!("Failed to initialize device with ID {}: {}", id, e);
                return Err(e);
            }
        }

        info!("Initialized {} devices", devices.len());
        Ok(())
    }

    /// Connect all devices
    pub async fn connect_all_devices(&self) -> Result<(), DeviceError> {
        let devices = self.devices.read().map_err(|_| {
            DeviceError::Other("Failed to acquire read lock on device registry".to_string())
        })?;

        for (id, device) in devices.iter() {
            debug!("Connecting to device with ID {}", id);
            if let Err(e) = device.connect().await {
                error!("Failed to connect to device with ID {}: {}", id, e);
                return Err(e);
            }
        }

        info!("Connected to {} devices", devices.len());
        Ok(())
    }

    /// Disconnect all devices
    pub async fn disconnect_all_devices(&self) -> Result<(), DeviceError> {
        let devices = self.devices.read().map_err(|_| {
            DeviceError::Other("Failed to acquire read lock on device registry".to_string())
        })?;

        for (id, device) in devices.iter() {
            debug!("Disconnecting from device with ID {}", id);
            if let Err(e) = device.disconnect().await {
                warn!("Failed to disconnect from device with ID {}: {}", id, e);
                // Continue with other devices even if one fails
            }
        }

        info!("Disconnected from {} devices", devices.len());
        Ok(())
    }

    /// Shutdown all devices
    pub async fn shutdown_all_devices(&self) -> Result<(), DeviceError> {
        let devices = self.devices.read().map_err(|_| {
            DeviceError::Other("Failed to acquire read lock on device registry".to_string())
        })?;

        for (id, device) in devices.iter() {
            debug!("Shutting down device with ID {}", id);
            if let Err(e) = device.shutdown().await {
                warn!("Failed to shut down device with ID {}: {}", id, e);
                // Continue with other devices even if one fails
            }
        }

        info!("Shut down {} devices", devices.len());
        Ok(())
    }

    /// Count registered devices
    pub fn count_devices(&self) -> Result<usize, DeviceError> {
        let devices = self.devices.read().map_err(|_| {
            DeviceError::Other("Failed to acquire read lock on device registry".to_string())
        })?;

        Ok(devices.len())
    }

    /// Check if a device is registered
    pub fn has_device(&self, id: &Id) -> Result<bool, DeviceError> {
        let devices = self.devices.read().map_err(|_| {
            DeviceError::Other("Failed to acquire read lock on device registry".to_string())
        })?;

        Ok(devices.contains_key(id))
    }
}

impl Default for DeviceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// A shared device registry that can be cloned
#[derive(Debug, Clone)]
pub struct SharedDeviceRegistry(Arc<DeviceRegistry>);

impl SharedDeviceRegistry {
    /// Create a new shared device registry
    pub fn new() -> Self {
        Self(Arc::new(DeviceRegistry::new()))
    }

    /// Get a reference to the device registry
    pub fn registry(&self) -> &DeviceRegistry {
        &self.0
    }
}

impl Default for SharedDeviceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl AsRef<DeviceRegistry> for SharedDeviceRegistry {
    fn as_ref(&self) -> &DeviceRegistry {
        self.registry()
    }
}
