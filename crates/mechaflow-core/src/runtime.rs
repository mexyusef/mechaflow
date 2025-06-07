/*!
 * Runtime engine for MechaFlow.
 *
 * This module provides the core runtime engine for executing workflows,
 * managing devices, and handling events.
 */
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use tokio::runtime::Runtime as TokioRuntime;
use tokio::runtime::Builder as TokioRuntimeBuilder;
use tokio::sync::Mutex;
use tracing::{debug, error, info, info_span, trace, warn};

use crate::config::{Config, SharedConfig};
use crate::error::{Error, Result};
use crate::event::{EventBus, SharedEventBus};
use crate::types::Id;

/// Runtime engine for MechaFlow
pub struct Runtime {
    /// Tokio runtime for async execution
    tokio_runtime: TokioRuntime,
    /// Configuration
    config: SharedConfig,
    /// Event bus
    event_bus: SharedEventBus,
    /// Device registry
    devices: Arc<RwLock<HashMap<Id, Box<dyn Device>>>>,
    /// Workflow registry
    workflows: Arc<RwLock<HashMap<Id, Box<dyn Workflow>>>>,
    /// Running workflows
    running_workflows: Arc<Mutex<HashMap<Id, tokio::task::JoinHandle<Result<()>>>>>,
}

/// Device trait for hardware abstraction
pub trait Device: Send + Sync {
    /// Get the device ID
    fn id(&self) -> &Id;

    /// Get the device type
    fn device_type(&self) -> &str;

    /// Initialize the device
    fn initialize(&mut self) -> Result<()>;

    /// Shutdown the device
    fn shutdown(&mut self) -> Result<()>;
}

/// Workflow trait for automation workflows
pub trait Workflow: Send + Sync {
    /// Get the workflow ID
    fn id(&self) -> &Id;

    /// Get the workflow name
    fn name(&self) -> &str;

    /// Execute the workflow
    fn execute(&self, ctx: &dyn WorkflowContext) -> Result<()>;
}

/// Workflow context for workflow execution
pub trait WorkflowContext: Send + Sync {
    /// Get a reference to the event bus
    fn event_bus(&self) -> &SharedEventBus;

    /// Get a device by ID
    fn get_device(&self, id: &Id) -> Result<Box<dyn Device>>;

    /// Read a value from a device
    fn read_device(&self, device_id: &Id, property: &str) -> Result<crate::types::Value>;

    /// Write a value to a device
    fn write_device(&self, device_id: &Id, property: &str, value: crate::types::Value) -> Result<()>;
}

/// Context implementation for workflow execution
struct RuntimeContext {
    runtime: Arc<Runtime>,
}

impl WorkflowContext for RuntimeContext {
    fn event_bus(&self) -> &SharedEventBus {
        &self.runtime.event_bus
    }

    fn get_device(&self, id: &Id) -> Result<Box<dyn Device>> {
        // This is a placeholder - in a real implementation, we would
        // clone a device reference or create a device proxy
        Err(Error::not_implemented("get_device not implemented yet"))
    }

    fn read_device(&self, device_id: &Id, property: &str) -> Result<crate::types::Value> {
        // This is a placeholder - in a real implementation, we would
        // read a value from the device
        Err(Error::not_implemented("read_device not implemented yet"))
    }

    fn write_device(&self, device_id: &Id, property: &str, value: crate::types::Value) -> Result<()> {
        // This is a placeholder - in a real implementation, we would
        // write a value to the device
        Err(Error::not_implemented("write_device not implemented yet"))
    }
}

impl Runtime {
    /// Create a new runtime with default configuration
    pub fn new() -> Result<Self> {
        Self::with_config(Config::default())
    }

    /// Create a new runtime with a specific configuration
    pub fn with_config(config: Config) -> Result<Self> {
        let shared_config = SharedConfig::new(config.clone());

        // Build the Tokio runtime
        let worker_threads = if config.runtime.worker_threads == 0 {
            None // Let Tokio decide based on available cores
        } else {
            Some(config.runtime.worker_threads)
        };

        let tokio_runtime = TokioRuntimeBuilder::new_multi_thread()
            .worker_threads(worker_threads.unwrap_or_else(num_cpus::get))
            .enable_all()
            .thread_name("mechaflow-worker")
            .build()
            .map_err(|e| Error::runtime(format!("Failed to create Tokio runtime: {}", e)))?;

        // Create the event bus
        let event_bus = SharedEventBus::new();

        info!("Created MechaFlow runtime");

        Ok(Self {
            tokio_runtime,
            config: shared_config,
            event_bus,
            devices: Arc::new(RwLock::new(HashMap::new())),
            workflows: Arc::new(RwLock::new(HashMap::new())),
            running_workflows: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Get a reference to the configuration
    pub fn config(&self) -> &SharedConfig {
        &self.config
    }

    /// Get a reference to the event bus
    pub fn event_bus(&self) -> &SharedEventBus {
        &self.event_bus
    }

    /// Register a device with the runtime
    pub fn register_device<D: Device + 'static>(&self, device: D) -> Result<()> {
        let id = device.id().clone();
        let mut devices = self.devices.write()
            .map_err(|_| Error::runtime("Failed to acquire write lock on devices"))?;

        if devices.contains_key(&id) {
            return Err(Error::device(format!("Device with ID {} already registered", id)));
        }

        devices.insert(id.clone(), Box::new(device));
        debug!("Registered device with ID {}", id);

        Ok(())
    }

    /// Unregister a device from the runtime
    pub fn unregister_device(&self, id: &Id) -> Result<()> {
        let mut devices = self.devices.write()
            .map_err(|_| Error::runtime("Failed to acquire write lock on devices"))?;

        if devices.remove(id).is_none() {
            return Err(Error::device(format!("Device with ID {} not found", id)));
        }

        debug!("Unregistered device with ID {}", id);

        Ok(())
    }

    /// Register a workflow with the runtime
    pub fn register_workflow<W: Workflow + 'static>(&self, workflow: W) -> Result<()> {
        let id = workflow.id().clone();
        let mut workflows = self.workflows.write()
            .map_err(|_| Error::runtime("Failed to acquire write lock on workflows"))?;

        if workflows.contains_key(&id) {
            return Err(Error::workflow(format!("Workflow with ID {} already registered", id)));
        }

        workflows.insert(id.clone(), Box::new(workflow));
        debug!("Registered workflow with ID {}", id);

        Ok(())
    }

    /// Unregister a workflow from the runtime
    pub fn unregister_workflow(&self, id: &Id) -> Result<()> {
        let mut workflows = self.workflows.write()
            .map_err(|_| Error::runtime("Failed to acquire write lock on workflows"))?;

        if workflows.remove(id).is_none() {
            return Err(Error::workflow(format!("Workflow with ID {} not found", id)));
        }

        debug!("Unregistered workflow with ID {}", id);

        Ok(())
    }

    /// Run a workflow
    pub fn run_workflow(&self, id: &Id) -> Result<()> {
        let workflows = self.workflows.read()
            .map_err(|_| Error::runtime("Failed to acquire read lock on workflows"))?;

        let workflow = workflows.get(id)
            .ok_or_else(|| Error::workflow(format!("Workflow with ID {} not found", id)))?;

        let workflow_clone = Arc::new(workflow.clone());
        let runtime_clone = Arc::new(self.clone());
        let id_clone = id.clone();

        let span = info_span!("workflow", id = %id, name = %workflow.name());

        let handle = self.tokio_runtime.spawn(async move {
            let _guard = span.enter();
            info!("Starting workflow execution");

            let ctx = RuntimeContext {
                runtime: runtime_clone,
            };

            let result = workflow_clone.execute(&ctx);

            match &result {
                Ok(_) => info!("Workflow execution completed successfully"),
                Err(e) => error!("Workflow execution failed: {}", e),
            }

            // Remove from running workflows
            let mut running = runtime_clone.running_workflows.lock().await;
            running.remove(&id_clone);

            result
        });

        // Register the running workflow
        let mut running_workflows = self.running_workflows.blocking_lock();
        running_workflows.insert(id.clone(), handle);

        Ok(())
    }

    /// Stop a running workflow
    pub async fn stop_workflow(&self, id: &Id) -> Result<()> {
        let mut running_workflows = self.running_workflows.lock().await;

        if let Some(handle) = running_workflows.remove(id) {
            debug!("Stopping workflow with ID {}", id);
            handle.abort();
            return Ok(());
        }

        Err(Error::workflow(format!("Workflow with ID {} is not running", id)))
    }

    /// Initialize all registered devices
    pub fn initialize_devices(&self) -> Result<()> {
        let mut devices = self.devices.write()
            .map_err(|_| Error::runtime("Failed to acquire write lock on devices"))?;

        for (id, device) in devices.iter_mut() {
            debug!("Initializing device with ID {}", id);
            if let Err(e) = device.initialize() {
                error!("Failed to initialize device with ID {}: {}", id, e);
                return Err(e);
            }
        }

        info!("Initialized {} devices", devices.len());

        Ok(())
    }

    /// Shutdown all registered devices
    pub fn shutdown_devices(&self) -> Result<()> {
        let mut devices = self.devices.write()
            .map_err(|_| Error::runtime("Failed to acquire write lock on devices"))?;

        for (id, device) in devices.iter_mut() {
            debug!("Shutting down device with ID {}", id);
            if let Err(e) = device.shutdown() {
                warn!("Failed to shut down device with ID {}: {}", id, e);
                // Continue with other devices even if one fails
            }
        }

        info!("Shut down {} devices", devices.len());

        Ok(())
    }

    /// Stop all running workflows
    pub async fn stop_all_workflows(&self) -> Result<()> {
        let mut running_workflows = self.running_workflows.lock().await;

        for (id, handle) in running_workflows.drain() {
            debug!("Stopping workflow with ID {}", id);
            handle.abort();
        }

        Ok(())
    }

    /// Shutdown the runtime
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down MechaFlow runtime");

        // Stop all running workflows
        self.stop_all_workflows().await?;

        // Shutdown all devices
        self.shutdown_devices()?;

        info!("MechaFlow runtime shutdown complete");

        Ok(())
    }

    /// Block the current thread until a ctrl-c signal is received
    pub fn block_until_ctrl_c(&self) -> Result<()> {
        self.tokio_runtime.block_on(async {
            match tokio::signal::ctrl_c().await {
                Ok(()) => {
                    info!("Received ctrl-c signal, shutting down");
                    self.shutdown().await?;
                    Ok(())
                }
                Err(err) => {
                    error!("Failed to listen for ctrl-c: {}", err);
                    Err(Error::runtime(format!("Failed to listen for ctrl-c: {}", err)))
                }
            }
        })
    }
}

impl Clone for Runtime {
    fn clone(&self) -> Self {
        Self {
            tokio_runtime: self.tokio_runtime.clone(),
            config: self.config.clone(),
            event_bus: self.event_bus.clone(),
            devices: self.devices.clone(),
            workflows: self.workflows.clone(),
            running_workflows: self.running_workflows.clone(),
        }
    }
}

/// A shared runtime that can be cloned
#[derive(Clone)]
pub struct SharedRuntime(Arc<Runtime>);

impl SharedRuntime {
    /// Create a new shared runtime with default configuration
    pub fn new() -> Result<Self> {
        Ok(Self(Arc::new(Runtime::new()?)))
    }

    /// Create a new shared runtime with a specific configuration
    pub fn with_config(config: Config) -> Result<Self> {
        Ok(Self(Arc::new(Runtime::with_config(config)?)))
    }

    /// Get a reference to the runtime
    pub fn runtime(&self) -> &Runtime {
        &self.0
    }
}

impl AsRef<Runtime> for SharedRuntime {
    fn as_ref(&self) -> &Runtime {
        self.runtime()
    }
}

/// Add a `not_implemented` method to the Error type
trait ErrorExt {
    fn not_implemented(msg: &str) -> Self;
}

impl ErrorExt for Error {
    fn not_implemented(msg: &str) -> Self {
        Error::other(format!("Not implemented: {}", msg))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    // Mock device implementation for testing
    struct MockDevice {
        id: Id,
        initialized: AtomicBool,
        shutdown_called: AtomicBool,
    }

    impl MockDevice {
        fn new(id: &str) -> Self {
            Self {
                id: Id::from_string(id),
                initialized: AtomicBool::new(false),
                shutdown_called: AtomicBool::new(false),
            }
        }
    }

    impl Device for MockDevice {
        fn id(&self) -> &Id {
            &self.id
        }

        fn device_type(&self) -> &str {
            "mock"
        }

        fn initialize(&mut self) -> Result<()> {
            self.initialized.store(true, Ordering::SeqCst);
            Ok(())
        }

        fn shutdown(&mut self) -> Result<()> {
            self.shutdown_called.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    // Mock workflow implementation for testing
    struct MockWorkflow {
        id: Id,
        name: String,
        executed: Arc<AtomicBool>,
    }

    impl MockWorkflow {
        fn new(id: &str, name: &str) -> Self {
            Self {
                id: Id::from_string(id),
                name: name.to_string(),
                executed: Arc::new(AtomicBool::new(false)),
            }
        }

        fn was_executed(&self) -> bool {
            self.executed.load(Ordering::SeqCst)
        }
    }

    impl Workflow for MockWorkflow {
        fn id(&self) -> &Id {
            &self.id
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn execute(&self, _ctx: &dyn WorkflowContext) -> Result<()> {
            self.executed.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    impl Clone for MockWorkflow {
        fn clone(&self) -> Self {
            Self {
                id: self.id.clone(),
                name: self.name.clone(),
                executed: self.executed.clone(),
            }
        }
    }

    #[test]
    fn test_runtime_creation() {
        let runtime = Runtime::new().unwrap();
        assert!(runtime.config.get().general.app_name == "mechaflow");
    }

    #[test]
    fn test_device_registration() {
        let runtime = Runtime::new().unwrap();
        let device = MockDevice::new("device1");

        // Register device
        runtime.register_device(device).unwrap();

        // Try to register with same ID (should fail)
        let device2 = MockDevice::new("device1");
        assert!(runtime.register_device(device2).is_err());

        // Unregister device
        runtime.unregister_device(&Id::from_string("device1")).unwrap();

        // Try to unregister again (should fail)
        assert!(runtime.unregister_device(&Id::from_string("device1")).is_err());
    }

    #[test]
    fn test_workflow_registration() {
        let runtime = Runtime::new().unwrap();
        let workflow = MockWorkflow::new("workflow1", "test workflow");

        // Register workflow
        runtime.register_workflow(workflow).unwrap();

        // Try to register with same ID (should fail)
        let workflow2 = MockWorkflow::new("workflow1", "test workflow 2");
        assert!(runtime.register_workflow(workflow2).is_err());

        // Unregister workflow
        runtime.unregister_workflow(&Id::from_string("workflow1")).unwrap();

        // Try to unregister again (should fail)
        assert!(runtime.unregister_workflow(&Id::from_string("workflow1")).is_err());
    }

    #[test]
    fn test_device_initialization() {
        let runtime = Runtime::new().unwrap();
        let device = MockDevice::new("device1");
        runtime.register_device(device).unwrap();

        // Initialize devices
        runtime.initialize_devices().unwrap();

        // Check that the device was initialized
        let devices = runtime.devices.read().unwrap();
        let device = devices.get(&Id::from_string("device1")).unwrap();
        let device = device.as_any().downcast_ref::<MockDevice>().unwrap();
        assert!(device.initialized.load(Ordering::SeqCst));
    }

    #[test]
    fn test_device_shutdown() {
        let runtime = Runtime::new().unwrap();
        let device = MockDevice::new("device1");
        runtime.register_device(device).unwrap();

        // Shutdown devices
        runtime.shutdown_devices().unwrap();

        // Check that the device was shut down
        let devices = runtime.devices.read().unwrap();
        let device = devices.get(&Id::from_string("device1")).unwrap();
        let device = device.as_any().downcast_ref::<MockDevice>().unwrap();
        assert!(device.shutdown_called.load(Ordering::SeqCst));
    }

    // We'd need more tests for other functionality, but this is a start
}

/// Extension trait for downcasting Any objects to their concrete types
trait DeviceExt: Device {
    fn as_any(&self) -> &dyn std::any::Any;
}
