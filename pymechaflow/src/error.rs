use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

/// Convert a Rust Result to a Python Result
pub fn to_pyresult<T, E: std::fmt::Display>(result: Result<T, E>) -> PyResult<T> {
    result.map_err(|e| PyErr::new::<PyRuntimeError, _>(format!("{}", e)))
}

/// Define MechaFlow error types for Python
#[pyclass]
#[derive(Debug, Clone)]
pub struct MechaFlowError {
    message: String,
    kind: String,
}

#[pymethods]
impl MechaFlowError {
    /// Create a new MechaFlow error
    #[new]
    fn new(message: String, kind: String) -> Self {
        Self { message, kind }
    }

    /// Get the error message
    #[getter]
    fn message(&self) -> &str {
        &self.message
    }

    /// Get the error kind
    #[getter]
    fn kind(&self) -> &str {
        &self.kind
    }

    fn __str__(&self) -> String {
        format!("{}: {}", self.kind, self.message)
    }

    fn __repr__(&self) -> String {
        format!("MechaFlowError({}, {})", self.message, self.kind)
    }
}

impl From<mechaflow_core::error::Error> for MechaFlowError {
    fn from(error: mechaflow_core::error::Error) -> Self {
        Self {
            message: error.to_string(),
            kind: "CoreError".to_string(),
        }
    }
}

impl From<mechaflow_devices::DeviceError> for MechaFlowError {
    fn from(error: mechaflow_devices::DeviceError) -> Self {
        Self {
            message: error.to_string(),
            kind: "DeviceError".to_string(),
        }
    }
}

impl From<mechaflow_engine::error::Error> for MechaFlowError {
    fn from(error: mechaflow_engine::error::Error) -> Self {
        Self {
            message: error.to_string(),
            kind: "EngineError".to_string(),
        }
    }
}

impl From<std::io::Error> for MechaFlowError {
    fn from(error: std::io::Error) -> Self {
        Self {
            message: error.to_string(),
            kind: "IoError".to_string(),
        }
    }
}

impl From<serde_json::Error> for MechaFlowError {
    fn from(error: serde_json::Error) -> Self {
        Self {
            message: error.to_string(),
            kind: "SerializationError".to_string(),
        }
    }
}

impl From<&str> for MechaFlowError {
    fn from(error: &str) -> Self {
        Self {
            message: error.to_string(),
            kind: "GeneralError".to_string(),
        }
    }
}

impl From<String> for MechaFlowError {
    fn from(error: String) -> Self {
        Self {
            message: error,
            kind: "GeneralError".to_string(),
        }
    }
}
