/*!
 * Device discovery for MechaFlow.
 *
 * This module provides functionality for discovering devices
 * across various protocols and interfaces.
 */
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures::Stream;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

use mechaflow_core::types::{Id, Value};

use crate::device::{Device, DeviceError, DeviceInfo};
use crate::registry::DeviceRegistry;

/// Discovery options
#[derive(Debug, Clone)]
pub struct DiscoveryOptions {
    /// The timeout duration for discovery
    pub timeout: Duration,
    /// Whether to return the first device found
    pub first_only: bool,
    /// A filter for device types
    pub device_type_filter: Option<String>,
    /// A filter for device protocols
    pub protocol_filter: Option<String>,
    /// Additional protocol-specific options
    pub protocol_options: HashMap<String, Value>,
}

impl Default for DiscoveryOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(10),
            first_only: false,
            device_type_filter: None,
            protocol_filter: None,
            protocol_options: HashMap::new(),
        }
    }
}

/// Discovery result
#[derive(Debug, Clone)]
pub struct DiscoveryResult {
    /// The discovered device information
    pub device_info: DeviceInfo,
    /// The connection string for the device
    pub connection_string: String,
    /// Additional protocol-specific information
    pub protocol_info: HashMap<String, Value>,
}

/// Device discovery trait
///
/// This trait defines the interface for device discovery providers.
#[async_trait]
pub trait DeviceDiscovery: Send + Sync + Debug {
    /// Get the discovery provider name
    fn name(&self) -> &'static str;

    /// Get supported protocols
    fn supported_protocols(&self) -> Vec<&'static str>;

    /// Check if a protocol is supported
    fn supports_protocol(&self, protocol: &str) -> bool {
        self.supported_protocols().contains(&protocol)
    }

    /// Discover devices
    ///
    /// This method should discover devices and return them as a stream.
    /// The options can be used to filter the discovery results.
    async fn discover(
        &self,
        options: DiscoveryOptions,
    ) -> Result<mpsc::Receiver<DiscoveryResult>, DeviceError>;

    /// Create a device from a discovery result
    ///
    /// This method should create a device from a discovery result.
    async fn create_device(&self, result: DiscoveryResult) -> Result<Box<dyn Device>, DeviceError>;
}

/// Device discoverer
///
/// This struct manages discovery providers and provides a unified interface for device discovery.
#[derive(Debug)]
pub struct DeviceDiscoverer {
    /// The registered discovery providers
    providers: Vec<Arc<dyn DeviceDiscovery>>,
}

impl DeviceDiscoverer {
    /// Create a new device discoverer
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    /// Register a discovery provider
    pub fn register_provider<D: DeviceDiscovery + 'static>(&mut self, provider: D) {
        self.providers.push(Arc::new(provider));
    }

    /// Get registered discovery providers
    pub fn providers(&self) -> &[Arc<dyn DeviceDiscovery>] {
        &self.providers
    }

    /// Get discovery providers for a protocol
    pub fn providers_for_protocol(&self, protocol: &str) -> Vec<Arc<dyn DeviceDiscovery>> {
        self.providers
            .iter()
            .filter(|p| p.supports_protocol(protocol))
            .cloned()
            .collect()
    }

    /// Discover devices
    ///
    /// This method discovers devices using all registered providers.
    /// The options can be used to filter the discovery results.
    pub async fn discover(
        &self,
        options: DiscoveryOptions,
    ) -> Result<mpsc::Receiver<DiscoveryResult>, DeviceError> {
        if self.providers.is_empty() {
            return Err(DeviceError::Other("No discovery providers registered".to_string()));
        }

        let (tx, rx) = mpsc::channel(100);

        // Filter providers based on protocol filter
        let providers = match &options.protocol_filter {
            Some(protocol) => self.providers_for_protocol(protocol),
            None => self.providers.clone(),
        };

        if providers.is_empty() {
            return Err(DeviceError::Other(format!(
                "No discovery providers for protocol {:?}",
                options.protocol_filter
            )));
        }

        for provider in providers {
            let provider_name = provider.name();
            let options = options.clone();
            let tx = tx.clone();

            tokio::spawn(async move {
                debug!("Starting discovery with provider {}", provider_name);
                match provider.discover(options.clone()).await {
                    Ok(mut receiver) => {
                        while let Some(result) = receiver.recv().await {
                            // Apply device type filter if specified
                            if let Some(ref filter) = options.device_type_filter {
                                if result.device_info.device_type != *filter {
                                    continue;
                                }
                            }

                            // Send the result to the aggregated channel
                            if tx.send(result).await.is_err() {
                                break;
                            }

                            // Stop after first device if requested
                            if options.first_only {
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Error in discovery provider {}: {}", provider_name, e);
                    }
                }
                debug!("Discovery with provider {} completed", provider_name);
            });
        }

        Ok(rx)
    }

    /// Discover and register devices
    ///
    /// This method discovers devices and registers them with the provided registry.
    pub async fn discover_and_register(
        &self,
        registry: &DeviceRegistry,
        options: DiscoveryOptions,
    ) -> Result<Vec<Id>, DeviceError> {
        let mut receiver = self.discover(options.clone()).await?;
        let timeout_duration = options.timeout;
        let mut registered_ids = Vec::new();

        loop {
            match timeout(timeout_duration, receiver.recv()).await {
                Ok(Some(result)) => {
                    let device_info = &result.device_info;
                    info!(
                        "Discovered device: {} ({}) via {}",
                        device_info.name, device_info.id, device_info.protocol
                    );

                    // Find the appropriate provider for this protocol
                    if let Some(provider) = self
                        .providers
                        .iter()
                        .find(|p| p.supports_protocol(&device_info.protocol))
                    {
                        match provider.create_device(result.clone()).await {
                            Ok(device) => {
                                let id = device.id().clone();
                                match registry.register_device(*device) {
                                    Ok(()) => {
                                        info!("Registered device with ID {}", id);
                                        registered_ids.push(id);

                                        // Stop after first device if requested
                                        if options.first_only {
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        warn!("Failed to register device with ID {}: {}", id, e);
                                    }
                                }
                            }
                            Err(e) => {
                                error!(
                                    "Failed to create device from discovery result: {}",
                                    e
                                );
                            }
                        }
                    } else {
                        warn!(
                            "No provider found for protocol {}",
                            device_info.protocol
                        );
                    }
                }
                Ok(None) => {
                    debug!("No more discovery results");
                    break;
                }
                Err(_) => {
                    debug!("Discovery timeout reached");
                    break;
                }
            }
        }

        Ok(registered_ids)
    }
}

impl Default for DeviceDiscoverer {
    fn default() -> Self {
        Self::new()
    }
}

/// A shared device discoverer that can be cloned
#[derive(Debug, Clone)]
pub struct SharedDeviceDiscoverer(Arc<DeviceDiscoverer>);

impl SharedDeviceDiscoverer {
    /// Create a new shared device discoverer
    pub fn new() -> Self {
        Self(Arc::new(DeviceDiscoverer::new()))
    }

    /// Get a reference to the device discoverer
    pub fn discoverer(&self) -> &DeviceDiscoverer {
        &self.0
    }
}

impl Default for SharedDeviceDiscoverer {
    fn default() -> Self {
        Self::new()
    }
}

impl AsRef<DeviceDiscoverer> for SharedDeviceDiscoverer {
    fn as_ref(&self) -> &DeviceDiscoverer {
        self.discoverer()
    }
}
