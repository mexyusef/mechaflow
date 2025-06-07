/*!
 * MechaFlow Device Abstractions
 *
 * This crate provides device abstractions and protocol implementations
 * for the MechaFlow automation system.
 */

// Re-export core modules
pub mod device;
pub mod registry;
pub mod discovery;
pub mod protocol;

// Re-export core types for convenience
pub use device::{Device, DeviceCapability, DeviceError, DeviceEvent, DeviceInfo, DeviceState, Result};
pub use registry::{DeviceRegistry, SharedDeviceRegistry};
pub use discovery::{DeviceDiscoverer, DeviceDiscovery, DiscoveryOptions, DiscoveryResult, SharedDeviceDiscoverer};
pub use protocol::{Protocol, ProtocolOptions, ProtocolProvider, ProtocolRegistry};

/// Protocols module for protocol implementations
pub mod protocols {
    //! Protocol implementations for various device types.
    //!
    //! This module contains implementations of the `Protocol` trait
    //! for different device communication protocols.
}

/// Device types module for specific device implementations
pub mod devices {
    //! Device implementations for various device types.
    //!
    //! This module contains implementations of the `Device` trait
    //! for different types of devices.
}

/// Device drivers module for hardware device drivers
pub mod drivers {
    //! Device drivers for hardware devices.
    //!
    //! This module contains drivers for interfacing with hardware devices
    //! through various protocols and interfaces.
}

/// Utility module for device-related utilities
pub mod util {
    //! Utility functions for device management.
    //!
    //! This module contains utility functions for device management,
    //! connection string parsing, and other device-related operations.

    pub use crate::protocol::util::*;
}
