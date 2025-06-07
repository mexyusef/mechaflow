/*!
 * MechaFlow Devices
 *
 * This crate provides device abstractions and protocol implementations
 * for the MechaFlow automation system.
 */

#![warn(missing_docs)]
#![warn(rustdoc::missing_doc_code_examples)]

// Re-export core types
pub use mechaflow_core::prelude;

pub mod device;
pub mod discovery;
pub mod registry;
pub mod protocols;

// Re-export device trait and basic implementations
pub use device::{Device, DeviceCapability, DeviceState, DeviceError, DeviceEvent, DeviceInfo};

/// MechaFlow devices crate version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// Conditional compilation for protocol-specific modules
#[cfg(feature = "modbus")]
pub mod modbus;

#[cfg(feature = "mqtt")]
pub mod mqtt;

#[cfg(feature = "opcua")]
pub mod opcua;

#[cfg(feature = "serial")]
pub mod serial;

#[cfg(feature = "gpio")]
pub mod gpio;

/// Initialize the device system
pub fn init() -> Result<(), mechaflow_core::error::Error> {
    tracing::info!("MechaFlow Devices {} initialized", VERSION);
    Ok(())
}

/// Information about available protocols
pub fn available_protocols() -> Vec<&'static str> {
    let mut protocols = Vec::new();

    #[cfg(feature = "modbus")]
    protocols.push("modbus");

    #[cfg(feature = "mqtt")]
    protocols.push("mqtt");

    #[cfg(feature = "opcua")]
    protocols.push("opcua");

    #[cfg(feature = "serial")]
    protocols.push("serial");

    #[cfg(feature = "gpio")]
    protocols.push("gpio");

    protocols
}
