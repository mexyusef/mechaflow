/*!
 * Protocol definitions for MechaFlow.
 *
 * This module provides traits and structures for implementing
 * device communication protocols.
 */
use std::collections::HashMap;
use std::fmt::Debug;
use std::time::Duration;

use async_trait::async_trait;
use mechaflow_core::types::{Id, Value};

use crate::device::{DeviceError, Result};

/// Protocol options for configuring protocol behavior
#[derive(Debug, Clone, Default)]
pub struct ProtocolOptions {
    /// The timeout for protocol operations
    pub timeout: Option<Duration>,
    /// Additional protocol-specific options
    pub options: HashMap<String, Value>,
}

impl ProtocolOptions {
    /// Creates a new instance of protocol options
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the timeout for protocol operations
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Adds a protocol-specific option
    pub fn with_option<K: Into<String>, V: Into<Value>>(mut self, key: K, value: V) -> Self {
        self.options.insert(key.into(), value.into());
        self
    }

    /// Gets a protocol-specific option
    pub fn get_option<T: TryFrom<Value>>(&self, key: &str) -> Option<T>
    where
        T::Error: Debug,
    {
        self.options.get(key).and_then(|v| {
            T::try_from(v.clone()).map_err(|e| {
                tracing::warn!("Failed to convert option {}: {:?}", key, e);
                e
            }).ok()
        })
    }

    /// Gets a string option
    pub fn get_string(&self, key: &str) -> Option<String> {
        self.get_option::<String>(key)
    }

    /// Gets an integer option
    pub fn get_integer(&self, key: &str) -> Option<i64> {
        self.get_option::<i64>(key)
    }

    /// Gets a float option
    pub fn get_float(&self, key: &str) -> Option<f64> {
        self.get_option::<f64>(key)
    }

    /// Gets a boolean option
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get_option::<bool>(key)
    }

    /// Gets the timeout or a default value
    pub fn timeout_or(&self, default: Duration) -> Duration {
        self.timeout.unwrap_or(default)
    }
}

/// Protocol trait for implementing device communication protocols
#[async_trait]
pub trait Protocol: Send + Sync + Debug {
    /// Get the protocol name
    fn name(&self) -> &'static str;

    /// Get the protocol version
    fn version(&self) -> &'static str;

    /// Initialize the protocol
    async fn initialize(&self, options: ProtocolOptions) -> Result<()>;

    /// Connect to a device
    async fn connect(&self, connection_string: &str, options: ProtocolOptions) -> Result<()>;

    /// Disconnect from a device
    async fn disconnect(&self, connection_string: &str) -> Result<()>;

    /// Read a property from a device
    async fn read_property(
        &self,
        connection_string: &str,
        property: &str,
        options: ProtocolOptions,
    ) -> Result<Value>;

    /// Write a property to a device
    async fn write_property(
        &self,
        connection_string: &str,
        property: &str,
        value: Value,
        options: ProtocolOptions,
    ) -> Result<()>;

    /// Execute a command on a device
    async fn execute_command(
        &self,
        connection_string: &str,
        command: &str,
        parameters: HashMap<String, Value>,
        options: ProtocolOptions,
    ) -> Result<Value>;

    /// Check if the protocol supports a specific feature
    fn supports_feature(&self, feature: &str) -> bool;

    /// Get supported features
    fn supported_features(&self) -> Vec<&'static str>;
}

/// Protocol provider trait for protocol implementations
#[async_trait]
pub trait ProtocolProvider: Send + Sync + Debug {
    /// Get the provider name
    fn name(&self) -> &'static str;

    /// Get supported protocols
    fn supported_protocols(&self) -> Vec<&'static str>;

    /// Check if a protocol is supported
    fn supports_protocol(&self, protocol: &str) -> bool {
        self.supported_protocols().contains(&protocol)
    }

    /// Create a protocol instance
    async fn create_protocol(&self, protocol: &str) -> Result<Box<dyn Protocol>>;
}

/// ProtocolRegistry manages protocol providers and protocols
#[derive(Debug, Default)]
pub struct ProtocolRegistry {
    /// The registered protocol providers
    providers: HashMap<String, Box<dyn ProtocolProvider>>,
    /// The created protocol instances
    protocols: HashMap<String, Box<dyn Protocol>>,
}

impl ProtocolRegistry {
    /// Create a new protocol registry
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            protocols: HashMap::new(),
        }
    }

    /// Register a protocol provider
    pub fn register_provider<P: ProtocolProvider + 'static>(&mut self, provider: P) {
        let name = provider.name().to_string();
        self.providers.insert(name, Box::new(provider));
    }

    /// Get a protocol provider by name
    pub fn get_provider(&self, name: &str) -> Option<&dyn ProtocolProvider> {
        self.providers.get(name).map(|p| p.as_ref())
    }

    /// Get all protocol providers
    pub fn providers(&self) -> Vec<&dyn ProtocolProvider> {
        self.providers.values().map(|p| p.as_ref()).collect()
    }

    /// Get a protocol instance by name
    pub fn get_protocol(&self, name: &str) -> Option<&dyn Protocol> {
        self.protocols.get(name).map(|p| p.as_ref())
    }

    /// Get or create a protocol instance
    pub async fn get_or_create_protocol(&mut self, protocol_name: &str) -> Result<&dyn Protocol> {
        if !self.protocols.contains_key(protocol_name) {
            // Find a provider that supports this protocol
            let provider = self
                .providers
                .values()
                .find(|p| p.supports_protocol(protocol_name))
                .ok_or_else(|| {
                    DeviceError::UnsupportedProtocol(protocol_name.to_string())
                })?;

            // Create the protocol
            let protocol = provider.create_protocol(protocol_name).await?;

            // Initialize the protocol with default options
            protocol.initialize(ProtocolOptions::default()).await?;

            // Store the protocol
            self.protocols.insert(protocol_name.to_string(), protocol);
        }

        Ok(self.protocols.get(protocol_name).unwrap().as_ref())
    }
}

/// Protocol utilities
pub mod util {
    use super::*;

    /// Parse a connection string into a map of key-value pairs
    ///
    /// Connection string format: "protocol://param1=value1;param2=value2"
    pub fn parse_connection_string(conn_str: &str) -> Result<(String, HashMap<String, String>)> {
        // Split protocol and parameters
        let parts: Vec<&str> = conn_str.splitn(2, "://").collect();
        if parts.len() != 2 {
            return Err(DeviceError::Other(format!(
                "Invalid connection string format: {}",
                conn_str
            )));
        }

        let protocol = parts[0].to_string();
        let params_str = parts[1];

        // Parse parameters
        let mut params = HashMap::new();
        for param in params_str.split(';') {
            if param.is_empty() {
                continue;
            }

            let kv: Vec<&str> = param.splitn(2, '=').collect();
            if kv.len() != 2 {
                return Err(DeviceError::Other(format!(
                    "Invalid parameter format in connection string: {}",
                    param
                )));
            }

            params.insert(kv[0].to_string(), kv[1].to_string());
        }

        Ok((protocol, params))
    }

    /// Build a connection string from protocol and parameters
    pub fn build_connection_string(
        protocol: &str,
        params: &HashMap<String, String>,
    ) -> String {
        let params_str = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<String>>()
            .join(";");

        format!("{}://{}", protocol, params_str)
    }
}
