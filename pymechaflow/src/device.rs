use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::sync::{Arc, RwLock};

use mechaflow_core::types::Value;
use mechaflow_devices::{
    device::{Device, DeviceEvent, DeviceId, DeviceInfo, DeviceProperty, DeviceType},
    registry::SharedDeviceRegistry,
};

use crate::error::to_pyresult;
use crate::utils::{hashmap_to_pydict, pyobject_to_value, value_to_pyobject};

#[pyclass(name = "DeviceId")]
#[derive(Clone)]
pub struct PyDeviceId {
    pub inner: DeviceId,
}

#[pymethods]
impl PyDeviceId {
    #[new]
    fn new(id: String) -> Self {
        PyDeviceId {
            inner: DeviceId::new(id),
        }
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("DeviceId('{}')", self.inner)
    }
}

#[pyclass(name = "DeviceInfo")]
#[derive(Clone)]
pub struct PyDeviceInfo {
    pub inner: DeviceInfo,
}

#[pymethods]
impl PyDeviceInfo {
    #[new]
    fn new(
        id: String,
        name: String,
        device_type: String,
        manufacturer: Option<String>,
        model: Option<String>,
        firmware_version: Option<String>,
    ) -> Self {
        PyDeviceInfo {
            inner: DeviceInfo {
                id: DeviceId::new(id),
                name,
                device_type: DeviceType::new(device_type),
                manufacturer,
                model,
                firmware_version,
            },
        }
    }

    #[getter]
    fn id(&self, py: Python) -> PyResult<PyObject> {
        let py_id = PyDeviceId {
            inner: self.inner.id.clone(),
        };
        Ok(py_id.into_py(py))
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    #[getter]
    fn device_type(&self) -> String {
        self.inner.device_type.to_string()
    }

    #[getter]
    fn manufacturer(&self) -> Option<String> {
        self.inner.manufacturer.clone()
    }

    #[getter]
    fn model(&self) -> Option<String> {
        self.inner.model.clone()
    }

    #[getter]
    fn firmware_version(&self) -> Option<String> {
        self.inner.firmware_version.clone()
    }

    fn __str__(&self) -> String {
        format!(
            "DeviceInfo(id={}, name='{}', type='{}')",
            self.inner.id, self.inner.name, self.inner.device_type
        )
    }

    fn __repr__(&self) -> String {
        format!(
            "DeviceInfo(id={}, name='{}', type='{}', manufacturer={:?}, model={:?}, firmware_version={:?})",
            self.inner.id, self.inner.name, self.inner.device_type, self.inner.manufacturer,
            self.inner.model, self.inner.firmware_version
        )
    }
}

#[pyclass(name = "DeviceProperty")]
#[derive(Clone)]
pub struct PyDeviceProperty {
    pub inner: DeviceProperty,
}

#[pymethods]
impl PyDeviceProperty {
    #[new]
    fn new(
        name: String,
        value: &PyAny,
        read_only: Option<bool>,
        description: Option<String>,
    ) -> PyResult<Self> {
        let value = pyobject_to_value(value)?;
        Ok(PyDeviceProperty {
            inner: DeviceProperty {
                name,
                value,
                read_only: read_only.unwrap_or(false),
                description,
            },
        })
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    #[getter]
    fn value(&self, py: Python) -> PyResult<PyObject> {
        value_to_pyobject(py, &self.inner.value)
    }

    #[getter]
    fn read_only(&self) -> bool {
        self.inner.read_only
    }

    #[getter]
    fn description(&self) -> Option<String> {
        self.inner.description.clone()
    }

    fn __str__(&self) -> String {
        format!(
            "DeviceProperty(name='{}', value={}, read_only={})",
            self.inner.name, self.inner.value, self.inner.read_only
        )
    }

    fn __repr__(&self) -> String {
        format!(
            "DeviceProperty(name='{}', value={}, read_only={}, description={:?})",
            self.inner.name, self.inner.value, self.inner.read_only, self.inner.description
        )
    }
}

#[pyclass(name = "DeviceEvent")]
#[derive(Clone)]
pub struct PyDeviceEvent {
    pub inner: DeviceEvent,
}

#[pymethods]
impl PyDeviceEvent {
    #[staticmethod]
    fn property_changed(
        device_id: PyDeviceId,
        property_name: String,
        value: &PyAny,
    ) -> PyResult<Self> {
        let value = pyobject_to_value(value)?;
        Ok(PyDeviceEvent {
            inner: DeviceEvent::PropertyChanged {
                device_id: device_id.inner,
                property_name,
                value,
            },
        })
    }

    #[staticmethod]
    fn state_changed(device_id: PyDeviceId, connected: bool) -> Self {
        PyDeviceEvent {
            inner: DeviceEvent::StateChanged {
                device_id: device_id.inner,
                connected,
            },
        }
    }

    #[staticmethod]
    fn command_executed(
        device_id: PyDeviceId,
        command: String,
        args: Option<&PyDict>,
        result: &PyAny,
    ) -> PyResult<Self> {
        let parsed_args = if let Some(args_dict) = args {
            let mut map = std::collections::HashMap::new();
            for (k, v) in args_dict.iter() {
                let key = k.extract::<String>()?;
                let value = pyobject_to_value(v)?;
                map.insert(key, value);
            }
            Some(map)
        } else {
            None
        };

        let result_value = pyobject_to_value(result)?;

        Ok(PyDeviceEvent {
            inner: DeviceEvent::CommandExecuted {
                device_id: device_id.inner,
                command,
                args: parsed_args,
                result: result_value,
            },
        })
    }

    fn get_device_id(&self) -> PyDeviceId {
        match &self.inner {
            DeviceEvent::PropertyChanged { device_id, .. } => PyDeviceId {
                inner: device_id.clone(),
            },
            DeviceEvent::StateChanged { device_id, .. } => PyDeviceId {
                inner: device_id.clone(),
            },
            DeviceEvent::CommandExecuted { device_id, .. } => PyDeviceId {
                inner: device_id.clone(),
            },
        }
    }

    fn is_property_changed(&self) -> bool {
        matches!(self.inner, DeviceEvent::PropertyChanged { .. })
    }

    fn is_state_changed(&self) -> bool {
        matches!(self.inner, DeviceEvent::StateChanged { .. })
    }

    fn is_command_executed(&self) -> bool {
        matches!(self.inner, DeviceEvent::CommandExecuted { .. })
    }

    fn get_property_name(&self) -> Option<String> {
        match &self.inner {
            DeviceEvent::PropertyChanged { property_name, .. } => Some(property_name.clone()),
            _ => None,
        }
    }

    fn get_property_value(&self, py: Python) -> PyResult<Option<PyObject>> {
        match &self.inner {
            DeviceEvent::PropertyChanged { value, .. } => {
                Ok(Some(value_to_pyobject(py, value)?))
            }
            _ => Ok(None),
        }
    }

    fn get_connected_state(&self) -> Option<bool> {
        match &self.inner {
            DeviceEvent::StateChanged { connected, .. } => Some(*connected),
            _ => None,
        }
    }

    fn get_command_info(&self, py: Python) -> PyResult<Option<(String, Option<Py<PyDict>>, PyObject)>> {
        match &self.inner {
            DeviceEvent::CommandExecuted {
                command, args, result, ..
            } => {
                let py_args = if let Some(args_map) = args {
                    Some(hashmap_to_pydict(py, args_map)?)
                } else {
                    None
                };
                let py_result = value_to_pyobject(py, result)?;
                Ok(Some((command.clone(), py_args, py_result)))
            }
            _ => Ok(None),
        }
    }

    fn __str__(&self) -> String {
        match &self.inner {
            DeviceEvent::PropertyChanged {
                device_id,
                property_name,
                value,
            } => {
                format!(
                    "DeviceEvent::PropertyChanged(device_id={}, property_name='{}', value={})",
                    device_id, property_name, value
                )
            }
            DeviceEvent::StateChanged {
                device_id, connected, ..
            } => {
                format!(
                    "DeviceEvent::StateChanged(device_id={}, connected={})",
                    device_id, connected
                )
            }
            DeviceEvent::CommandExecuted {
                device_id,
                command,
                args,
                result,
            } => {
                format!(
                    "DeviceEvent::CommandExecuted(device_id={}, command='{}', args={:?}, result={})",
                    device_id, command, args, result
                )
            }
        }
    }
}

#[pyclass(name = "DeviceRegistry")]
pub struct PyDeviceRegistry {
    pub inner: SharedDeviceRegistry,
}

#[pymethods]
impl PyDeviceRegistry {
    #[new]
    fn new() -> Self {
        PyDeviceRegistry {
            inner: SharedDeviceRegistry::new(),
        }
    }

    fn register_device(&self, py_device: PyObject) -> PyResult<()> {
        // This will need to be implemented with a callback mechanism
        // from Rust to Python for device methods
        // For now, we'll implement a placeholder
        Err(PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(
            "Device registration from Python not yet implemented",
        ))
    }

    fn get_device(&self, device_id: PyDeviceId) -> PyResult<Option<PyObject>> {
        // This would require a Python wrapper for the Rust Device trait
        // For now, we'll implement a placeholder
        Err(PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(
            "Getting devices from Python not yet implemented",
        ))
    }

    fn get_all_devices(&self) -> PyResult<Vec<PyObject>> {
        // This would require Python wrappers for all Rust Device implementations
        // For now, we'll implement a placeholder
        Err(PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(
            "Getting all devices from Python not yet implemented",
        ))
    }

    fn get_device_ids(&self, py: Python) -> PyResult<Py<PyList>> {
        let devices = self.inner.get_devices();
        let ids = PyList::empty(py);
        for device in devices {
            let py_id = PyDeviceId {
                inner: device.get_info().id.clone(),
            }
            .into_py(py);
            ids.append(py_id)?;
        }
        Ok(ids.into())
    }

    fn connect_device(&self, device_id: PyDeviceId) -> PyResult<bool> {
        to_pyresult(self.inner.connect_device(&device_id.inner))
    }

    fn disconnect_device(&self, device_id: PyDeviceId) -> PyResult<bool> {
        to_pyresult(self.inner.disconnect_device(&device_id.inner))
    }

    fn is_device_connected(&self, device_id: PyDeviceId) -> PyResult<bool> {
        to_pyresult(self.inner.is_device_connected(&device_id.inner))
    }

    fn get_device_property(
        &self,
        device_id: PyDeviceId,
        property_name: &str,
        py: Python,
    ) -> PyResult<Option<PyObject>> {
        let result = to_pyresult(self.inner.get_device_property(&device_id.inner, property_name))?;
        match result {
            Some(value) => Ok(Some(value_to_pyobject(py, &value)?)),
            None => Ok(None),
        }
    }

    fn set_device_property(
        &self,
        device_id: PyDeviceId,
        property_name: &str,
        value: &PyAny,
    ) -> PyResult<bool> {
        let rust_value = pyobject_to_value(value)?;
        to_pyresult(
            self.inner
                .set_device_property(&device_id.inner, property_name, rust_value),
        )
    }

    fn execute_device_command(
        &self,
        device_id: PyDeviceId,
        command: &str,
        args: Option<&PyDict>,
        py: Python,
    ) -> PyResult<PyObject> {
        let parsed_args = if let Some(args_dict) = args {
            let mut map = std::collections::HashMap::new();
            for (k, v) in args_dict.iter() {
                let key = k.extract::<String>()?;
                let value = pyobject_to_value(v)?;
                map.insert(key, value);
            }
            Some(map)
        } else {
            None
        };

        let result = to_pyresult(
            self.inner
                .execute_device_command(&device_id.inner, command, parsed_args),
        )?;
        value_to_pyobject(py, &result)
    }

    // In a real implementation, you would also need to provide a way to subscribe to events
    // through a callback mechanism. This is more complex and requires consideration of
    // Python's threading model and PyO3's handling of callbacks.
}

pub fn register_device_module(py: Python<'_>, m: &PyModule) -> PyResult<()> {
    let device_module = PyModule::new(py, "device")?;

    device_module.add_class::<PyDeviceId>()?;
    device_module.add_class::<PyDeviceInfo>()?;
    device_module.add_class::<PyDeviceProperty>()?;
    device_module.add_class::<PyDeviceEvent>()?;
    device_module.add_class::<PyDeviceRegistry>()?;

    m.add_submodule(device_module)?;
    Ok(())
}
