use pyo3::prelude::*;

mod error;
mod utils;
mod device;
mod engine;

/// Python bindings for the MechaFlow automation framework.
#[pymodule]
fn pymechaflow(py: Python, m: &PyModule) -> PyResult<()> {
    // Register modules
    device::register_device_module(py, m)?;
    engine::register_engine_module(py, m)?;

    // Add version info
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    Ok(())
}

/// MechaFlow main class
#[pyclass]
struct MechaFlow {
    /// The runtime context
    #[allow(dead_code)]
    context: tokio::runtime::Runtime,
}

#[pymethods]
impl MechaFlow {
    /// Create a new MechaFlow instance
    #[new]
    fn new() -> PyResult<Self> {
        // Create a new tokio runtime
        let context = tokio::runtime::Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Failed to create runtime: {}", e)))?;

        Ok(Self { context })
    }

    /// Initialize the MechaFlow system
    fn initialize(&self) -> PyResult<()> {
        // Initialize the system
        Ok(())
    }

    /// Get the version of the MechaFlow library
    fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }
}
