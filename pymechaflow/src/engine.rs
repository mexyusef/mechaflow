use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use mechaflow_core::types::Value;
use mechaflow_engine::{
    action::Action,
    condition::Condition,
    context::WorkflowContext,
    rules::{Rule, RuleEngine, RuleId},
    trigger::Trigger,
    workflow::{Workflow, WorkflowId, WorkflowScheduler},
};

use crate::device::PyDeviceRegistry;
use crate::error::to_pyresult;
use crate::utils::{hashmap_to_pydict, pydict_to_hashmap, pyobject_to_value, value_to_pyobject};

// Workflow Module

#[pyclass(name = "WorkflowId")]
#[derive(Clone)]
pub struct PyWorkflowId {
    pub inner: WorkflowId,
}

#[pymethods]
impl PyWorkflowId {
    #[new]
    fn new(id: String) -> Self {
        PyWorkflowId {
            inner: WorkflowId::new(id),
        }
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("WorkflowId('{}')", self.inner)
    }
}

#[pyclass(name = "Workflow")]
pub struct PyWorkflow {
    pub inner: Arc<RwLock<Workflow>>,
}

#[pymethods]
impl PyWorkflow {
    #[new]
    fn new(
        id: Option<String>,
        name: String,
        description: Option<String>,
        enabled: Option<bool>,
    ) -> Self {
        let workflow_id = match id {
            Some(id_str) => WorkflowId::new(id_str),
            None => WorkflowId::generate(),
        };

        let workflow = Workflow::new(workflow_id, name)
            .with_description(description)
            .with_enabled(enabled.unwrap_or(true));

        PyWorkflow {
            inner: Arc::new(RwLock::new(workflow)),
        }
    }

    #[getter]
    fn id(&self) -> PyResult<PyWorkflowId> {
        let workflow = to_pyresult(self.inner.read().map_err(|e| {
            mechaflow_core::error::Error::Runtime(format!("Failed to acquire read lock: {}", e))
        }))?;

        Ok(PyWorkflowId {
            inner: workflow.id().clone(),
        })
    }

    #[getter]
    fn name(&self) -> PyResult<String> {
        let workflow = to_pyresult(self.inner.read().map_err(|e| {
            mechaflow_core::error::Error::Runtime(format!("Failed to acquire read lock: {}", e))
        }))?;

        Ok(workflow.name().to_owned())
    }

    #[setter]
    fn set_name(&self, name: String) -> PyResult<()> {
        let mut workflow = to_pyresult(self.inner.write().map_err(|e| {
            mechaflow_core::error::Error::Runtime(format!("Failed to acquire write lock: {}", e))
        }))?;

        workflow.set_name(name);
        Ok(())
    }

    #[getter]
    fn description(&self) -> PyResult<Option<String>> {
        let workflow = to_pyresult(self.inner.read().map_err(|e| {
            mechaflow_core::error::Error::Runtime(format!("Failed to acquire read lock: {}", e))
        }))?;

        Ok(workflow.description().cloned())
    }

    #[setter]
    fn set_description(&self, description: Option<String>) -> PyResult<()> {
        let mut workflow = to_pyresult(self.inner.write().map_err(|e| {
            mechaflow_core::error::Error::Runtime(format!("Failed to acquire write lock: {}", e))
        }))?;

        workflow.set_description(description);
        Ok(())
    }

    #[getter]
    fn enabled(&self) -> PyResult<bool> {
        let workflow = to_pyresult(self.inner.read().map_err(|e| {
            mechaflow_core::error::Error::Runtime(format!("Failed to acquire read lock: {}", e))
        }))?;

        Ok(workflow.enabled())
    }

    #[setter]
    fn set_enabled(&self, enabled: bool) -> PyResult<()> {
        let mut workflow = to_pyresult(self.inner.write().map_err(|e| {
            mechaflow_core::error::Error::Runtime(format!("Failed to acquire write lock: {}", e))
        }))?;

        workflow.set_enabled(enabled);
        Ok(())
    }

    // For now, we'll implement placeholders for triggers, conditions, and actions
    // In a complete implementation, these would be Python callback functions

    fn add_trigger(&self, py_trigger: PyObject) -> PyResult<()> {
        Err(PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(
            "Adding triggers from Python not yet implemented",
        ))
    }

    fn add_condition(&self, py_condition: PyObject) -> PyResult<()> {
        Err(PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(
            "Adding conditions from Python not yet implemented",
        ))
    }

    fn add_action(&self, py_action: PyObject) -> PyResult<()> {
        Err(PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(
            "Adding actions from Python not yet implemented",
        ))
    }

    fn __str__(&self) -> PyResult<String> {
        let workflow = to_pyresult(self.inner.read().map_err(|e| {
            mechaflow_core::error::Error::Runtime(format!("Failed to acquire read lock: {}", e))
        }))?;

        Ok(format!(
            "Workflow(id={}, name='{}', enabled={})",
            workflow.id(),
            workflow.name(),
            workflow.enabled()
        ))
    }
}

#[pyclass(name = "WorkflowScheduler")]
pub struct PyWorkflowScheduler {
    pub inner: Arc<RwLock<WorkflowScheduler>>,
    pub device_registry: PyDeviceRegistry,
}

#[pymethods]
impl PyWorkflowScheduler {
    #[new]
    fn new(device_registry: PyDeviceRegistry) -> Self {
        PyWorkflowScheduler {
            inner: Arc::new(RwLock::new(WorkflowScheduler::new(device_registry.inner.clone()))),
            device_registry,
        }
    }

    fn register_workflow(&self, workflow: PyWorkflow) -> PyResult<()> {
        let mut scheduler = to_pyresult(self.inner.write().map_err(|e| {
            mechaflow_core::error::Error::Runtime(format!("Failed to acquire write lock: {}", e))
        }))?;

        to_pyresult(scheduler.register_workflow(workflow.inner.clone()))
    }

    fn unregister_workflow(&self, workflow_id: PyWorkflowId) -> PyResult<()> {
        let mut scheduler = to_pyresult(self.inner.write().map_err(|e| {
            mechaflow_core::error::Error::Runtime(format!("Failed to acquire write lock: {}", e))
        }))?;

        to_pyresult(scheduler.unregister_workflow(&workflow_id.inner))
    }

    fn get_workflow(&self, workflow_id: PyWorkflowId) -> PyResult<Option<PyWorkflow>> {
        let scheduler = to_pyresult(self.inner.read().map_err(|e| {
            mechaflow_core::error::Error::Runtime(format!("Failed to acquire read lock: {}", e))
        }))?;

        let workflow_arc = match scheduler.get_workflow(&workflow_id.inner) {
            Some(w) => w,
            None => return Ok(None),
        };

        Ok(Some(PyWorkflow {
            inner: workflow_arc,
        }))
    }

    fn get_all_workflows(&self) -> PyResult<Vec<PyWorkflow>> {
        let scheduler = to_pyresult(self.inner.read().map_err(|e| {
            mechaflow_core::error::Error::Runtime(format!("Failed to acquire read lock: {}", e))
        }))?;

        let mut py_workflows = Vec::new();
        for workflow in scheduler.get_all_workflows() {
            py_workflows.push(PyWorkflow {
                inner: workflow.clone(),
            });
        }

        Ok(py_workflows)
    }

    fn start(&self) -> PyResult<()> {
        let mut scheduler = to_pyresult(self.inner.write().map_err(|e| {
            mechaflow_core::error::Error::Runtime(format!("Failed to acquire write lock: {}", e))
        }))?;

        to_pyresult(scheduler.start())
    }

    fn stop(&self) -> PyResult<()> {
        let mut scheduler = to_pyresult(self.inner.write().map_err(|e| {
            mechaflow_core::error::Error::Runtime(format!("Failed to acquire write lock: {}", e))
        }))?;

        to_pyresult(scheduler.stop())
    }

    fn is_running(&self) -> PyResult<bool> {
        let scheduler = to_pyresult(self.inner.read().map_err(|e| {
            mechaflow_core::error::Error::Runtime(format!("Failed to acquire read lock: {}", e))
        }))?;

        Ok(scheduler.is_running())
    }
}

// Rule Engine Module

#[pyclass(name = "RuleId")]
#[derive(Clone)]
pub struct PyRuleId {
    pub inner: RuleId,
}

#[pymethods]
impl PyRuleId {
    #[new]
    fn new(id: String) -> Self {
        PyRuleId {
            inner: RuleId::new(id),
        }
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("RuleId('{}')", self.inner)
    }
}

#[pyclass(name = "Rule")]
pub struct PyRule {
    pub inner: Rule,
}

#[pymethods]
impl PyRule {
    #[new]
    fn new(
        id: Option<String>,
        name: String,
        description: Option<String>,
        py: Python,
    ) -> Self {
        let rule_id = match id {
            Some(id_str) => RuleId::new(id_str),
            None => RuleId::generate(),
        };

        let mut rule = Rule::new(rule_id, name);

        if let Some(desc) = description {
            rule = rule.with_description(desc);
        }

        PyRule { inner: rule }
    }

    #[getter]
    fn id(&self) -> PyRuleId {
        PyRuleId {
            inner: self.inner.id().clone(),
        }
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.name().to_owned()
    }

    #[setter]
    fn set_name(&mut self, name: String) {
        self.inner.set_name(name);
    }

    #[getter]
    fn description(&self) -> Option<String> {
        self.inner.description().cloned()
    }

    #[setter]
    fn set_description(&mut self, description: Option<String>) {
        self.inner.set_description(description);
    }

    #[getter]
    fn enabled(&self) -> bool {
        self.inner.enabled()
    }

    #[setter]
    fn set_enabled(&mut self, enabled: bool) {
        self.inner.set_enabled(enabled);
    }

    // For now, we'll implement placeholders for conditions and actions
    // In a complete implementation, these would be Python callback functions

    fn set_condition(&mut self, py_condition: PyObject) -> PyResult<()> {
        Err(PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(
            "Setting conditions from Python not yet implemented",
        ))
    }

    fn add_action(&mut self, py_action: PyObject) -> PyResult<()> {
        Err(PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(
            "Adding actions from Python not yet implemented",
        ))
    }

    fn __str__(&self) -> String {
        format!(
            "Rule(id={}, name='{}', enabled={})",
            self.inner.id(),
            self.inner.name(),
            self.inner.enabled()
        )
    }
}

#[pyclass(name = "RuleEngine")]
pub struct PyRuleEngine {
    pub inner: Arc<RwLock<RuleEngine>>,
    pub device_registry: PyDeviceRegistry,
}

#[pymethods]
impl PyRuleEngine {
    #[new]
    fn new(device_registry: PyDeviceRegistry) -> Self {
        PyRuleEngine {
            inner: Arc::new(RwLock::new(RuleEngine::new(device_registry.inner.clone()))),
            device_registry,
        }
    }

    fn add_rule(&self, rule: PyRule) -> PyResult<()> {
        let mut engine = to_pyresult(self.inner.write().map_err(|e| {
            mechaflow_core::error::Error::Runtime(format!("Failed to acquire write lock: {}", e))
        }))?;

        to_pyresult(engine.add_rule(rule.inner.clone()))
    }

    fn remove_rule(&self, rule_id: PyRuleId) -> PyResult<()> {
        let mut engine = to_pyresult(self.inner.write().map_err(|e| {
            mechaflow_core::error::Error::Runtime(format!("Failed to acquire write lock: {}", e))
        }))?;

        to_pyresult(engine.remove_rule(&rule_id.inner))
    }

    fn get_rule(&self, rule_id: PyRuleId) -> PyResult<Option<PyRule>> {
        let engine = to_pyresult(self.inner.read().map_err(|e| {
            mechaflow_core::error::Error::Runtime(format!("Failed to acquire read lock: {}", e))
        }))?;

        match engine.get_rule(&rule_id.inner) {
            Some(rule) => Ok(Some(PyRule { inner: rule.clone() })),
            None => Ok(None),
        }
    }

    fn get_all_rules(&self) -> PyResult<Vec<PyRule>> {
        let engine = to_pyresult(self.inner.read().map_err(|e| {
            mechaflow_core::error::Error::Runtime(format!("Failed to acquire read lock: {}", e))
        }))?;

        let mut py_rules = Vec::new();
        for rule in engine.get_all_rules() {
            py_rules.push(PyRule { inner: rule.clone() });
        }

        Ok(py_rules)
    }

    fn enable_rule(&self, rule_id: PyRuleId) -> PyResult<()> {
        let mut engine = to_pyresult(self.inner.write().map_err(|e| {
            mechaflow_core::error::Error::Runtime(format!("Failed to acquire write lock: {}", e))
        }))?;

        to_pyresult(engine.enable_rule(&rule_id.inner))
    }

    fn disable_rule(&self, rule_id: PyRuleId) -> PyResult<()> {
        let mut engine = to_pyresult(self.inner.write().map_err(|e| {
            mechaflow_core::error::Error::Runtime(format!("Failed to acquire write lock: {}", e))
        }))?;

        to_pyresult(engine.disable_rule(&rule_id.inner))
    }

    fn start(&self) -> PyResult<()> {
        let mut engine = to_pyresult(self.inner.write().map_err(|e| {
            mechaflow_core::error::Error::Runtime(format!("Failed to acquire write lock: {}", e))
        }))?;

        to_pyresult(engine.start())
    }

    fn stop(&self) -> PyResult<()> {
        let mut engine = to_pyresult(self.inner.write().map_err(|e| {
            mechaflow_core::error::Error::Runtime(format!("Failed to acquire write lock: {}", e))
        }))?;

        to_pyresult(engine.stop())
    }

    fn is_running(&self) -> PyResult<bool> {
        let engine = to_pyresult(self.inner.read().map_err(|e| {
            mechaflow_core::error::Error::Runtime(format!("Failed to acquire read lock: {}", e))
        }))?;

        Ok(engine.is_running())
    }
}

pub fn register_engine_module(py: Python<'_>, m: &PyModule) -> PyResult<()> {
    let engine_module = PyModule::new(py, "engine")?;

    // Workflow classes
    engine_module.add_class::<PyWorkflowId>()?;
    engine_module.add_class::<PyWorkflow>()?;
    engine_module.add_class::<PyWorkflowScheduler>()?;

    // Rule engine classes
    engine_module.add_class::<PyRuleId>()?;
    engine_module.add_class::<PyRule>()?;
    engine_module.add_class::<PyRuleEngine>()?;

    m.add_submodule(engine_module)?;
    Ok(())
}
