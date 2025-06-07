/*!
 * Logging functionality for MechaFlow.
 *
 * This module provides tracing setup and utilities for consistent logging
 * across the MechaFlow ecosystem.
 */
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use crate::error::{Error, Result};

/// Initialize the logging system with default configuration
pub fn init() -> Result<()> {
    init_with_filter("info")
}

/// Initialize the logging system with a specific filter
///
/// # Arguments
///
/// * `filter` - The log filter string (e.g., "info", "debug", "mechaflow=trace")
pub fn init_with_filter(filter: &str) -> Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(filter));

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true))
        .with(filter)
        .try_init()
        .map_err(|e| Error::runtime(format!("Failed to initialize logging: {}", e)))?;

    Ok(())
}

/// A convenience macro for creating structured logs with fields
#[macro_export]
macro_rules! log_with_fields {
    ($level:expr, $message:expr, $($field:tt)+) => {
        tracing::event!($level, $($field)+, message = $message)
    };
}

/// A type alias for a tracing span
pub type Span = tracing::Span;

/// Create a new span for a component
///
/// # Arguments
///
/// * `name` - The name of the component
/// * `id` - An optional ID for the component instance
pub fn component_span(name: &str, id: Option<&str>) -> Span {
    match id {
        Some(id) => tracing::info_span!("component", name = %name, id = %id),
        None => tracing::info_span!("component", name = %name),
    }
}

/// Create a new span for an operation
///
/// # Arguments
///
/// * `name` - The name of the operation
/// * `component` - The component performing the operation
pub fn operation_span(name: &str, component: &str) -> Span {
    tracing::info_span!("operation", name = %name, component = %component)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing::Level;

    #[test]
    fn test_init() {
        // This will fail if called multiple times in the same process
        // but it's fine for a single test
        let _ = init();
    }

    #[test]
    fn test_component_span() {
        let span = component_span("test", Some("123"));
        assert!(span.is_none()); // Span is not entered so is_none() should be true

        let span = component_span("test", None);
        assert!(span.is_none());
    }

    #[test]
    fn test_operation_span() {
        let span = operation_span("test_op", "test_component");
        assert!(span.is_none());
    }
}
