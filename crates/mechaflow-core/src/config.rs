/*!
 * Configuration management for MechaFlow.
 *
 * This module provides functionality to load, validate, and access configuration
 * settings for MechaFlow components.
 */
use std::path::Path;
use std::sync::Arc;

use config::{Config as ConfigLib, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::error::{Error, Result};

/// Core configuration for MechaFlow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// General configuration
    #[serde(default)]
    pub general: GeneralConfig,

    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,

    /// Runtime configuration
    #[serde(default)]
    pub runtime: RuntimeConfig,

    /// Security configuration
    #[serde(default)]
    pub security: SecurityConfig,
}

/// General configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// Application name
    #[serde(default = "default_app_name")]
    pub app_name: String,

    /// Application version
    #[serde(default = "default_app_version")]
    pub app_version: String,

    /// Application environment (development, production, etc.)
    #[serde(default = "default_environment")]
    pub environment: String,

    /// Data directory
    #[serde(default = "default_data_dir")]
    pub data_dir: String,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub level: String,

    /// Whether to log to a file
    #[serde(default)]
    pub file_logging: bool,

    /// Log file path (if file_logging is true)
    #[serde(default = "default_log_file")]
    pub log_file: String,

    /// Whether to log to stdout
    #[serde(default = "default_log_stdout")]
    pub stdout: bool,

    /// Whether to use JSON format for logs
    #[serde(default)]
    pub json_format: bool,
}

/// Runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// Number of worker threads (0 means use number of available CPU cores)
    #[serde(default)]
    pub worker_threads: usize,

    /// Maximum number of concurrent tasks
    #[serde(default = "default_max_concurrent_tasks")]
    pub max_concurrent_tasks: usize,

    /// Timeout for operations in milliseconds (0 means no timeout)
    #[serde(default)]
    pub operation_timeout_ms: u64,

    /// Enable debugging features
    #[serde(default)]
    pub debug_mode: bool,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable authentication
    #[serde(default)]
    pub authentication_enabled: bool,

    /// Enable authorization
    #[serde(default)]
    pub authorization_enabled: bool,

    /// Enable TLS
    #[serde(default)]
    pub tls_enabled: bool,

    /// TLS certificate path (if tls_enabled is true)
    #[serde(default = "default_tls_cert")]
    pub tls_cert: String,

    /// TLS key path (if tls_enabled is true)
    #[serde(default = "default_tls_key")]
    pub tls_key: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            logging: LoggingConfig::default(),
            runtime: RuntimeConfig::default(),
            security: SecurityConfig::default(),
        }
    }
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            app_name: default_app_name(),
            app_version: default_app_version(),
            environment: default_environment(),
            data_dir: default_data_dir(),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            file_logging: false,
            log_file: default_log_file(),
            stdout: default_log_stdout(),
            json_format: false,
        }
    }
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            worker_threads: 0, // Use number of available CPU cores
            max_concurrent_tasks: default_max_concurrent_tasks(),
            operation_timeout_ms: 0, // No timeout
            debug_mode: false,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            authentication_enabled: false,
            authorization_enabled: false,
            tls_enabled: false,
            tls_cert: default_tls_cert(),
            tls_key: default_tls_key(),
        }
    }
}

fn default_app_name() -> String {
    "mechaflow".to_string()
}

fn default_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

fn default_environment() -> String {
    "development".to_string()
}

fn default_data_dir() -> String {
    "./data".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_file() -> String {
    "./logs/mechaflow.log".to_string()
}

fn default_log_stdout() -> bool {
    true
}

fn default_max_concurrent_tasks() -> usize {
    100
}

fn default_tls_cert() -> String {
    "./certs/cert.pem".to_string()
}

fn default_tls_key() -> String {
    "./certs/key.pem".to_string()
}

/// A builder for creating a configuration
#[derive(Debug, Default)]
pub struct ConfigBuilder {
    config_file: Option<String>,
    environment_prefix: Option<String>,
    override_with: Option<Config>,
}

impl ConfigBuilder {
    /// Create a new ConfigBuilder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the config file path
    pub fn with_config_file<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.config_file = Some(path.as_ref().to_string_lossy().to_string());
        self
    }

    /// Set the environment variable prefix for configuration
    pub fn with_environment_prefix<S: AsRef<str>>(mut self, prefix: S) -> Self {
        self.environment_prefix = Some(prefix.as_ref().to_string());
        self
    }

    /// Override with an existing config
    pub fn override_with(mut self, config: Config) -> Self {
        self.override_with = Some(config);
        self
    }

    /// Build the configuration
    pub fn build(self) -> Result<Config> {
        let mut config_builder = ConfigLib::builder();

        // Start with default values
        let default_config = Config::default();
        config_builder = config_builder
            .add_source(config::Config::try_from(&default_config)
                .map_err(|e| Error::config(format!("Failed to create default config: {}", e)))?);

        // Add configuration from file if specified
        if let Some(config_file) = self.config_file {
            let path = Path::new(&config_file);
            if path.exists() {
                debug!("Loading configuration from {}", config_file);
                config_builder = config_builder.add_source(File::with_name(&config_file));
            } else {
                debug!("Configuration file {} does not exist, using defaults", config_file);
            }
        }

        // Add configuration from environment variables if prefix is specified
        if let Some(prefix) = self.environment_prefix {
            debug!("Loading configuration from environment variables with prefix {}", prefix);
            config_builder = config_builder.add_source(
                Environment::with_prefix(&prefix)
                    .separator("__")
                    .try_parsing(true)
            );
        }

        // Build the config
        let config_lib = config_builder.build()
            .map_err(|e| Error::config(format!("Failed to build configuration: {}", e)))?;

        // Convert to our config type
        let mut config: Config = config_lib.try_deserialize()
            .map_err(|e| Error::config(format!("Failed to deserialize configuration: {}", e)))?;

        // Override with provided config if specified
        if let Some(override_config) = self.override_with {
            config = override_config;
        }

        info!("Configuration loaded successfully");
        Ok(config)
    }
}

/// A thread-safe reference to a configuration
#[derive(Debug, Clone)]
pub struct SharedConfig(Arc<Config>);

impl SharedConfig {
    /// Create a new SharedConfig
    pub fn new(config: Config) -> Self {
        Self(Arc::new(config))
    }

    /// Get a reference to the config
    pub fn get(&self) -> &Config {
        &self.0
    }
}

impl From<Config> for SharedConfig {
    fn from(config: Config) -> Self {
        Self::new(config)
    }
}

impl AsRef<Config> for SharedConfig {
    fn as_ref(&self) -> &Config {
        self.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.general.app_name, "mechaflow");
        assert_eq!(config.logging.level, "info");
        assert_eq!(config.runtime.max_concurrent_tasks, 100);
        assert!(!config.security.authentication_enabled);
    }

    #[test]
    fn test_config_builder_defaults() {
        let config = ConfigBuilder::new().build().unwrap();
        assert_eq!(config.general.app_name, "mechaflow");
        assert_eq!(config.logging.level, "info");
    }

    #[test]
    fn test_config_builder_with_file() -> Result<()> {
        let dir = tempdir().map_err(|e| Error::other(e.to_string()))?;
        let file_path = dir.path().join("config.toml");

        {
            let mut file = File::create(&file_path).map_err(|e| Error::other(e.to_string()))?;
            file.write_all(br#"
                [general]
                app_name = "test-app"
                environment = "testing"

                [logging]
                level = "debug"
                file_logging = true
            "#).map_err(|e| Error::other(e.to_string()))?;
        }

        let config = ConfigBuilder::new()
            .with_config_file(file_path)
            .build()?;

        assert_eq!(config.general.app_name, "test-app");
        assert_eq!(config.general.environment, "testing");
        assert_eq!(config.logging.level, "debug");
        assert!(config.logging.file_logging);

        Ok(())
    }

    #[test]
    fn test_config_builder_with_env() -> Result<()> {
        env::set_var("MECHAFLOW__GENERAL__APP_NAME", "env-app");
        env::set_var("MECHAFLOW__LOGGING__LEVEL", "trace");

        let config = ConfigBuilder::new()
            .with_environment_prefix("mechaflow")
            .build()?;

        assert_eq!(config.general.app_name, "env-app");
        assert_eq!(config.logging.level, "trace");

        // Clean up
        env::remove_var("MECHAFLOW__GENERAL__APP_NAME");
        env::remove_var("MECHAFLOW__LOGGING__LEVEL");

        Ok(())
    }

    #[test]
    fn test_shared_config() {
        let config = Config::default();
        let shared = SharedConfig::new(config);

        assert_eq!(shared.get().general.app_name, "mechaflow");

        let shared2 = shared.clone();
        assert_eq!(shared2.get().general.app_name, "mechaflow");
    }
}
