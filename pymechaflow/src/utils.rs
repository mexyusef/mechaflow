use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::collections::HashMap;

use mechaflow_core::types::Value;

/// Convert a Rust Value to a Python object
pub fn value_to_pyobject(py: Python, value: &Value) -> PyResult<PyObject> {
    match value {
        Value::Null => Ok(py.None()),
        Value::Bool(b) => Ok(b.to_object(py)),
        Value::Number(n) => {
            if n.is_i64() {
                Ok(n.as_i64().unwrap().to_object(py))
            } else if n.is_u64() {
                Ok(n.as_u64().unwrap().to_object(py))
            } else {
                Ok(n.as_f64().unwrap().to_object(py))
            }
        }
        Value::String(s) => Ok(s.to_object(py)),
        Value::Array(a) => {
            let list = PyList::empty(py);
            for item in a {
                list.append(value_to_pyobject(py, item)?)?;
            }
            Ok(list.to_object(py))
        }
        Value::Object(o) => {
            let dict = PyDict::new(py);
            for (k, v) in o {
                dict.set_item(k, value_to_pyobject(py, v)?)?;
            }
            Ok(dict.to_object(py))
        }
    }
}

/// Convert a Python object to a Rust Value
pub fn pyobject_to_value(obj: &PyAny) -> PyResult<Value> {
    if obj.is_none() {
        return Ok(Value::Null);
    }

    if let Ok(b) = obj.extract::<bool>() {
        return Ok(Value::Bool(b));
    }

    if let Ok(i) = obj.extract::<i64>() {
        return Ok(Value::Number(serde_json::Number::from(i)));
    }

    if let Ok(f) = obj.extract::<f64>() {
        return Ok(Value::Number(
            serde_json::Number::from_f64(f).unwrap_or_else(|| serde_json::Number::from(0)),
        ));
    }

    if let Ok(s) = obj.extract::<String>() {
        return Ok(Value::String(s));
    }

    if let Ok(list) = obj.downcast::<PyList>() {
        let mut array = Vec::new();
        for item in list.iter() {
            array.push(pyobject_to_value(item)?);
        }
        return Ok(Value::Array(array));
    }

    if let Ok(dict) = obj.downcast::<PyDict>() {
        let mut map = HashMap::new();
        for (k, v) in dict.iter() {
            let key = k.extract::<String>()?;
            let value = pyobject_to_value(v)?;
            map.insert(key, value);
        }
        return Ok(Value::Object(map));
    }

    Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
        "Unsupported Python type: {}",
        obj.get_type().name()?
    )))
}

/// Convert a Rust HashMap to a Python Dict
pub fn hashmap_to_pydict<K, V>(py: Python, map: &HashMap<K, V>) -> PyResult<Py<PyDict>>
where
    K: ToPyObject,
    V: ToPyObject,
{
    let dict = PyDict::new(py);
    for (k, v) in map {
        dict.set_item(k, v)?;
    }
    Ok(dict.into())
}

/// Convert a Python Dict to a Rust HashMap
pub fn pydict_to_hashmap<K, V>(dict: &PyDict) -> PyResult<HashMap<K, V>>
where
    K: std::hash::Hash + Eq + std::fmt::Debug + Clone + FromPyObject<'_>,
    V: Clone + FromPyObject<'_>,
{
    let mut map = HashMap::new();
    for (k, v) in dict.iter() {
        let key = k.extract::<K>()?;
        let value = v.extract::<V>()?;
        map.insert(key, value);
    }
    Ok(map)
}
