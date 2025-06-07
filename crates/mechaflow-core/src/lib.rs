/*!
 * MechaFlow Core
 *
 * This crate provides the core functionality for the MechaFlow system,
 * including the runtime, event system, configuration, and logging.
 */

#![warn(missing_docs)]
#![warn(rustdoc::missing_doc_code_examples)]

pub mod config;
pub mod error;
pub mod event;
pub mod logging;
pub mod prelude;
pub mod runtime;
pub mod types;
pub mod utils;

/// Re-export of dependencies that are part of the public API
pub mod deps {
    pub use anyhow;
    pub use chrono;
    pub use futures;
    pub use serde;
    pub use tokio;
    pub use tracing;
    pub use uuid;
}

/// MechaFlow core crate version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Library initialization
pub fn init() -> Result<(), error::Error> {
    logging::init()?;
    tracing::info!("MechaFlow Core {} initialized", VERSION);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}
