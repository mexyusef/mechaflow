/*!
 * Workflow scheduling and execution.
 *
 * This module handles scheduling and executing workflows in response
 * to triggers and time-based events.
 */
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use tokio::sync::{broadcast, mpsc, Mutex, RwLock};
use tokio::task::JoinHandle;
use tokio::time;
use tracing::{debug, error, info, warn};

use mechaflow_core::types::{Id, Value};
use mechaflow_devices::device::DeviceEvent;
use mechaflow_devices::registry::{DeviceRegistry, RegistryEvent};

use crate::context::{BasicWorkflowContext, WorkflowContext};
use crate::error::{Error, Result};
use crate::trigger::{TimerEvent, TriggerEvent};
use crate::workflow::{Workflow, WorkflowId};

/// Workflow execution status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowExecutionStatus {
    /// Workflow is queued for execution
    Queued,
    /// Workflow is running
    Running,
    /// Workflow completed successfully
    Completed,
    /// Workflow failed
    Failed(String),
    /// Workflow execution timed out
    TimedOut,
    /// Workflow was cancelled
    Cancelled,
}

/// Workflow execution instance
#[derive(Debug)]
pub struct WorkflowExecution {
    /// Execution ID
    id: String,
    /// Workflow ID
    workflow_id: WorkflowId,
    /// Trigger event that started the execution
    trigger_event: Option<TriggerEvent>,
    /// Execution status
    status: WorkflowExecutionStatus,
    /// Start time
    start_time: Option<DateTime<Utc>>,
    /// End time
    end_time: Option<DateTime<Utc>>,
    /// Context variables
    variables: HashMap<String, Value>,
    /// Task handle
    task: Option<JoinHandle<Result<()>>>,
}

impl WorkflowExecution {
    /// Create a new workflow execution
    fn new(workflow_id: WorkflowId, trigger_event: Option<TriggerEvent>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            workflow_id,
            trigger_event,
            status: WorkflowExecutionStatus::Queued,
            start_time: None,
            end_time: None,
            variables: HashMap::new(),
            task: None,
        }
    }

    /// Get the execution ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the workflow ID
    pub fn workflow_id(&self) -> &WorkflowId {
        &self.workflow_id
    }

    /// Get the trigger event
    pub fn trigger_event(&self) -> Option<&TriggerEvent> {
        self.trigger_event.as_ref()
    }

    /// Get the execution status
    pub fn status(&self) -> &WorkflowExecutionStatus {
        &self.status
    }

    /// Get the start time
    pub fn start_time(&self) -> Option<DateTime<Utc>> {
        self.start_time
    }

    /// Get the end time
    pub fn end_time(&self) -> Option<DateTime<Utc>> {
        self.end_time
    }

    /// Get the context variables
    pub fn variables(&self) -> &HashMap<String, Value> {
        &self.variables
    }

    /// Check if the execution is active
    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            WorkflowExecutionStatus::Queued | WorkflowExecutionStatus::Running
        )
    }

    /// Cancel the execution
    pub fn cancel(&mut self) {
        if self.is_active() {
            if let Some(task) = self.task.take() {
                task.abort();
            }
            self.status = WorkflowExecutionStatus::Cancelled;
            self.end_time = Some(Utc::now());
        }
    }
}

/// Scheduler events
#[derive(Debug, Clone)]
pub enum SchedulerEvent {
    /// Workflow execution started
    WorkflowStarted {
        /// Execution ID
        execution_id: String,
        /// Workflow ID
        workflow_id: WorkflowId,
        /// Workflow name
        workflow_name: String,
    },
    /// Workflow execution completed
    WorkflowCompleted {
        /// Execution ID
        execution_id: String,
        /// Workflow ID
        workflow_id: WorkflowId,
        /// Workflow name
        workflow_name: String,
    },
    /// Workflow execution failed
    WorkflowFailed {
        /// Execution ID
        execution_id: String,
        /// Workflow ID
        workflow_id: WorkflowId,
        /// Workflow name
        workflow_name: String,
        /// Error message
        error: String,
    },
    /// Workflow execution timed out
    WorkflowTimedOut {
        /// Execution ID
        execution_id: String,
        /// Workflow ID
        workflow_id: WorkflowId,
        /// Workflow name
        workflow_name: String,
    },
    /// Workflow execution cancelled
    WorkflowCancelled {
        /// Execution ID
        execution_id: String,
        /// Workflow ID
        workflow_id: WorkflowId,
        /// Workflow name
        workflow_name: String,
    },
    /// Workflow registered
    WorkflowRegistered {
        /// Workflow ID
        workflow_id: WorkflowId,
        /// Workflow name
        workflow_name: String,
    },
    /// Workflow unregistered
    WorkflowUnregistered {
        /// Workflow ID
        workflow_id: WorkflowId,
        /// Workflow name
        workflow_name: String,
    },
    /// Scheduler started
    SchedulerStarted,
    /// Scheduler stopped
    SchedulerStopped,
}

/// Workflow scheduler
#[derive(Debug)]
pub struct WorkflowScheduler {
    /// Device registry
    device_registry: Arc<DeviceRegistry>,
    /// Registered workflows
    workflows: RwLock<HashMap<WorkflowId, Arc<Workflow>>>,
    /// Active executions
    executions: RwLock<HashMap<String, WorkflowExecution>>,
    /// Event sender
    event_tx: broadcast::Sender<SchedulerEvent>,
    /// Device event receiver
    device_event_rx: Option<broadcast::Receiver<DeviceEvent>>,
    /// Registry event receiver
    registry_event_rx: Option<broadcast::Receiver<RegistryEvent>>,
    /// Timer event channel
    timer_tx: mpsc::Sender<TimerEvent>,
    /// Timer event receiver
    timer_rx: Mutex<Option<mpsc::Receiver<TimerEvent>>>,
    /// Background task handles
    tasks: Mutex<Vec<JoinHandle<()>>>,
    /// Running flag
    running: RwLock<bool>,
    /// Maximum concurrent executions
    max_concurrent: usize,
    /// Default execution timeout
    default_timeout: Option<Duration>,
}

impl WorkflowScheduler {
    /// Create a new workflow scheduler
    pub fn new(device_registry: Arc<DeviceRegistry>) -> Self {
        let (event_tx, _) = broadcast::channel(100);
        let (timer_tx, timer_rx) = mpsc::channel(100);

        Self {
            device_registry,
            workflows: RwLock::new(HashMap::new()),
            executions: RwLock::new(HashMap::new()),
            event_tx,
            device_event_rx: None,
            registry_event_rx: None,
            timer_tx,
            timer_rx: Mutex::new(Some(timer_rx)),
            tasks: Mutex::new(Vec::new()),
            running: RwLock::new(false),
            max_concurrent: 10,
            default_timeout: Some(Duration::from_secs(60)),
        }
    }

    /// Set the maximum number of concurrent executions
    pub fn set_max_concurrent(&mut self, max_concurrent: usize) {
        self.max_concurrent = max_concurrent;
    }

    /// Set the default execution timeout
    pub fn set_default_timeout(&mut self, timeout: Option<Duration>) {
        self.default_timeout = timeout;
    }

    /// Subscribe to scheduler events
    pub fn subscribe(&self) -> broadcast::Receiver<SchedulerEvent> {
        self.event_tx.subscribe()
    }

    /// Start the scheduler
    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Ok(());
        }

        *running = true;

        // Set up device event receiver
        let device_rx = self.device_registry.subscribe_events();
        let registry_rx = self.device_registry.subscribe();

        // Store receivers
        self.device_event_rx.replace(device_rx);
        self.registry_event_rx.replace(registry_rx);

        // Take timer receiver
        let timer_rx = self.timer_rx.lock().await.take().unwrap();

        // Start background tasks
        let mut tasks = self.tasks.lock().await;

        // Start device event handler task
        tasks.push(self.start_device_event_handler().await?);

        // Start timer event handler task
        tasks.push(self.start_timer_event_handler(timer_rx).await?);

        // Emit scheduler started event
        if let Err(e) = self.event_tx.send(SchedulerEvent::SchedulerStarted) {
            warn!("Failed to send scheduler started event: {}", e);
        }

        info!("Workflow scheduler started");
        Ok(())
    }

    /// Stop the scheduler
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Ok(());
        }

        *running = false;

        // Cancel all active executions
        {
            let mut executions = self.executions.write().await;
            for execution in executions.values_mut() {
                execution.cancel();
            }
        }

        // Abort background tasks
        {
            let mut tasks = self.tasks.lock().await;
            for task in tasks.drain(..) {
                task.abort();
            }
        }

        // Emit scheduler stopped event
        if let Err(e) = self.event_tx.send(SchedulerEvent::SchedulerStopped) {
            warn!("Failed to send scheduler stopped event: {}", e);
        }

        info!("Workflow scheduler stopped");
        Ok(())
    }

    /// Register a workflow
    pub async fn register_workflow(&self, workflow: Arc<Workflow>) -> Result<()> {
        let mut workflows = self.workflows.write().await;

        if let Some(existing) = workflows.get(workflow.id()) {
            return Err(Error::already_exists(format!(
                "Workflow with ID {} already exists: {}",
                workflow.id(),
                existing.name()
            )));
        }

        workflows.insert(workflow.id().clone(), workflow.clone());

        // Emit workflow registered event
        if let Err(e) = self
            .event_tx
            .send(SchedulerEvent::WorkflowRegistered {
                workflow_id: workflow.id().clone(),
                workflow_name: workflow.name().to_string(),
            })
        {
            warn!("Failed to send workflow registered event: {}", e);
        }

        info!("Registered workflow: {}", workflow.name());
        Ok(())
    }

    /// Unregister a workflow
    pub async fn unregister_workflow(&self, workflow_id: &WorkflowId) -> Result<()> {
        let mut workflows = self.workflows.write().await;

        if let Some(workflow) = workflows.remove(workflow_id) {
            // Cancel any active executions for this workflow
            {
                let mut executions = self.executions.write().await;
                for execution in executions.values_mut() {
                    if execution.workflow_id() == workflow_id {
                        execution.cancel();
                    }
                }
            }

            // Emit workflow unregistered event
            if let Err(e) = self
                .event_tx
                .send(SchedulerEvent::WorkflowUnregistered {
                    workflow_id: workflow_id.clone(),
                    workflow_name: workflow.name().to_string(),
                })
            {
                warn!("Failed to send workflow unregistered event: {}", e);
            }

            info!("Unregistered workflow: {}", workflow.name());
            Ok(())
        } else {
            Err(Error::not_found(format!(
                "Workflow with ID {} not found",
                workflow_id
            )))
        }
    }

    /// Get a workflow by ID
    pub async fn get_workflow(&self, workflow_id: &WorkflowId) -> Result<Arc<Workflow>> {
        let workflows = self.workflows.read().await;

        if let Some(workflow) = workflows.get(workflow_id) {
            Ok(workflow.clone())
        } else {
            Err(Error::not_found(format!(
                "Workflow with ID {} not found",
                workflow_id
            )))
        }
    }

    /// Get all registered workflows
    pub async fn get_workflows(&self) -> Result<Vec<Arc<Workflow>>> {
        let workflows = self.workflows.read().await;
        Ok(workflows.values().cloned().collect())
    }

    /// Trigger a workflow manually
    pub async fn trigger_workflow(
        &self,
        workflow_id: &WorkflowId,
        variables: Option<HashMap<String, Value>>,
    ) -> Result<String> {
        // Prepare trigger event
        let mut data = HashMap::new();
        if let Some(vars) = variables {
            data = vars;
        }

        let trigger_event = TriggerEvent::Manual(crate::trigger::ManualTriggerEvent {
            timestamp: Utc::now(),
            user_id: None,
            data,
        });

        // Queue the workflow execution
        self.queue_workflow_execution(workflow_id.clone(), Some(trigger_event))
            .await
    }

    /// Get a workflow execution by ID
    pub async fn get_execution(&self, execution_id: &str) -> Result<WorkflowExecution> {
        let executions = self.executions.read().await;

        if let Some(execution) = executions.get(execution_id) {
            Ok(execution.clone())
        } else {
            Err(Error::not_found(format!(
                "Execution with ID {} not found",
                execution_id
            )))
        }
    }

    /// Get all active workflow executions
    pub async fn get_active_executions(&self) -> Result<Vec<WorkflowExecution>> {
        let executions = self.executions.read().await;
        Ok(executions
            .values()
            .filter(|e| e.is_active())
            .cloned()
            .collect())
    }

    /// Get all workflow executions
    pub async fn get_executions(&self) -> Result<Vec<WorkflowExecution>> {
        let executions = self.executions.read().await;
        Ok(executions.values().cloned().collect())
    }

    /// Cancel a workflow execution
    pub async fn cancel_execution(&self, execution_id: &str) -> Result<()> {
        let mut executions = self.executions.write().await;

        if let Some(execution) = executions.get_mut(execution_id) {
            execution.cancel();
            Ok(())
        } else {
            Err(Error::not_found(format!(
                "Execution with ID {} not found",
                execution_id
            )))
        }
    }

    /// Schedule a timer event
    pub async fn schedule_timer(
        &self,
        timer_id: String,
        name: Option<String>,
        delay: Duration,
        data: HashMap<String, Value>,
    ) -> Result<()> {
        let timer_tx = self.timer_tx.clone();

        tokio::spawn(async move {
            // Wait for the specified delay
            time::sleep(delay).await;

            // Create and send the timer event
            let timer_event = TimerEvent {
                timestamp: Utc::now(),
                timer_id,
                timer_name: name,
                data,
            };

            if let Err(e) = timer_tx.send(timer_event).await {
                error!("Failed to send timer event: {}", e);
            }
        });

        Ok(())
    }

    /// Queue a workflow execution
    async fn queue_workflow_execution(
        &self,
        workflow_id: WorkflowId,
        trigger_event: Option<TriggerEvent>,
    ) -> Result<String> {
        // Check if the workflow exists
        let workflow = {
            let workflows = self.workflows.read().await;
            match workflows.get(&workflow_id) {
                Some(workflow) => workflow.clone(),
                None => {
                    return Err(Error::not_found(format!(
                        "Workflow with ID {} not found",
                        workflow_id
                    )));
                }
            }
        };

        // Check if the workflow is enabled
        if !workflow.is_enabled() {
            return Err(Error::workflow(format!(
                "Workflow {} is disabled",
                workflow.name()
            )));
        }

        // Check if we have too many concurrent executions
        let active_count = {
            let executions = self.executions.read().await;
            executions.values().filter(|e| e.is_active()).count()
        };

        if active_count >= self.max_concurrent {
            return Err(Error::workflow(format!(
                "Too many concurrent workflow executions (max: {})",
                self.max_concurrent
            )));
        }

        // Check if the workflow has a max concurrent limit
        if let Some(max) = workflow.max_concurrent() {
            let workflow_active_count = {
                let executions = self.executions.read().await;
                executions
                    .values()
                    .filter(|e| e.is_active() && e.workflow_id() == &workflow_id)
                    .count()
            };

            if workflow_active_count >= max {
                return Err(Error::workflow(format!(
                    "Too many concurrent executions of workflow {} (max: {})",
                    workflow.name(),
                    max
                )));
            }
        }

        // Create the execution
        let mut execution = WorkflowExecution::new(workflow_id.clone(), trigger_event);
        let execution_id = execution.id().to_string();

        // Get timeout
        let timeout = workflow
            .timeout()
            .or(self.default_timeout)
            .map(|d| d.as_secs())
            .unwrap_or(60);

        // Create the workflow context
        let context = Arc::new(BasicWorkflowContext::new(self.device_registry.clone()));

        // Spawn the execution task
        let workflow_clone = workflow.clone();
        let context_clone = context.clone();
        let event_tx = self.event_tx.clone();
        let execution_id_clone = execution_id.clone();

        let task = tokio::spawn(async move {
            // Update status to running
            {
                let event = SchedulerEvent::WorkflowStarted {
                    execution_id: execution_id_clone.clone(),
                    workflow_id: workflow_clone.id().clone(),
                    workflow_name: workflow_clone.name().to_string(),
                };
                if let Err(e) = event_tx.send(event) {
                    warn!("Failed to send workflow started event: {}", e);
                }
            }

            let result = tokio::time::timeout(
                Duration::from_secs(timeout),
                workflow_clone.execute(&*context_clone),
            )
            .await;

            match result {
                Ok(Ok(_)) => {
                    // Workflow completed successfully
                    let event = SchedulerEvent::WorkflowCompleted {
                        execution_id: execution_id_clone.clone(),
                        workflow_id: workflow_clone.id().clone(),
                        workflow_name: workflow_clone.name().to_string(),
                    };
                    if let Err(e) = event_tx.send(event) {
                        warn!("Failed to send workflow completed event: {}", e);
                    }
                    Ok(())
                }
                Ok(Err(e)) => {
                    // Workflow failed
                    let error_msg = e.to_string();
                    let event = SchedulerEvent::WorkflowFailed {
                        execution_id: execution_id_clone.clone(),
                        workflow_id: workflow_clone.id().clone(),
                        workflow_name: workflow_clone.name().to_string(),
                        error: error_msg.clone(),
                    };
                    if let Err(e) = event_tx.send(event) {
                        warn!("Failed to send workflow failed event: {}", e);
                    }
                    Err(Error::workflow(error_msg))
                }
                Err(_) => {
                    // Workflow timed out
                    let event = SchedulerEvent::WorkflowTimedOut {
                        execution_id: execution_id_clone.clone(),
                        workflow_id: workflow_clone.id().clone(),
                        workflow_name: workflow_clone.name().to_string(),
                    };
                    if let Err(e) = event_tx.send(event) {
                        warn!("Failed to send workflow timed out event: {}", e);
                    }
                    Err(Error::timeout(format!(
                        "Workflow execution timed out after {} seconds",
                        timeout
                    )))
                }
            }
        });

        execution.task = Some(task);
        execution.start_time = Some(Utc::now());
        execution.status = WorkflowExecutionStatus::Running;

        // Store the execution
        {
            let mut executions = self.executions.write().await;
            executions.insert(execution_id.clone(), execution);
        }

        Ok(execution_id)
    }

    /// Start the device event handler task
    async fn start_device_event_handler(&self) -> Result<JoinHandle<()>> {
        let event_rx = match &self.device_event_rx {
            Some(rx) => rx.resubscribe(),
            None => {
                return Err(Error::workflow(
                    "Device event receiver not initialized".to_string(),
                ));
            }
        };

        let workflows_lock = self.workflows.clone();
        let device_registry = self.device_registry.clone();
        let this = Arc::new(self.clone());

        Ok(tokio::spawn(async move {
            let mut rx = event_rx;

            while let Ok(event) = rx.recv().await {
                debug!("Received device event: {:?}", event);

                // Convert to trigger event
                let trigger_event = TriggerEvent::Device(event);

                // Check which workflows should be triggered
                let workflows = {
                    let workflows = workflows_lock.read().await;
                    workflows.values().cloned().collect::<Vec<_>>()
                };

                for workflow in workflows {
                    let this_clone = this.clone();
                    let trigger_event_clone = trigger_event.clone();
                    let context = BasicWorkflowContext::new(device_registry.clone());

                    tokio::spawn(async move {
                        match workflow.process_event(&trigger_event_clone, &context).await {
                            Ok(true) => {
                                // Workflow was triggered, queue it for execution
                                match this_clone
                                    .queue_workflow_execution(
                                        workflow.id().clone(),
                                        Some(trigger_event_clone),
                                    )
                                    .await
                                {
                                    Ok(execution_id) => {
                                        debug!(
                                            "Queued workflow {} for execution (ID: {})",
                                            workflow.name(),
                                            execution_id
                                        );
                                    }
                                    Err(e) => {
                                        error!(
                                            "Failed to queue workflow {} for execution: {}",
                                            workflow.name(),
                                            e
                                        );
                                    }
                                }
                            }
                            Ok(false) => {
                                // Workflow was not triggered
                                debug!(
                                    "Workflow {} was not triggered by event",
                                    workflow.name()
                                );
                            }
                            Err(e) => {
                                error!(
                                    "Error processing event for workflow {}: {}",
                                    workflow.name(),
                                    e
                                );
                            }
                        }
                    });
                }
            }
        }))
    }

    /// Start the timer event handler task
    async fn start_timer_event_handler(
        &self,
        mut timer_rx: mpsc::Receiver<TimerEvent>,
    ) -> Result<JoinHandle<()>> {
        let workflows_lock = self.workflows.clone();
        let device_registry = self.device_registry.clone();
        let this = Arc::new(self.clone());

        Ok(tokio::spawn(async move {
            while let Some(event) = timer_rx.recv().await {
                debug!("Received timer event: {:?}", event);

                // Convert to trigger event
                let trigger_event = TriggerEvent::Timer(event);

                // Check which workflows should be triggered
                let workflows = {
                    let workflows = workflows_lock.read().await;
                    workflows.values().cloned().collect::<Vec<_>>()
                };

                for workflow in workflows {
                    let this_clone = this.clone();
                    let trigger_event_clone = trigger_event.clone();
                    let context = BasicWorkflowContext::new(device_registry.clone());

                    tokio::spawn(async move {
                        match workflow.process_event(&trigger_event_clone, &context).await {
                            Ok(true) => {
                                // Workflow was triggered, queue it for execution
                                match this_clone
                                    .queue_workflow_execution(
                                        workflow.id().clone(),
                                        Some(trigger_event_clone),
                                    )
                                    .await
                                {
                                    Ok(execution_id) => {
                                        debug!(
                                            "Queued workflow {} for execution (ID: {})",
                                            workflow.name(),
                                            execution_id
                                        );
                                    }
                                    Err(e) => {
                                        error!(
                                            "Failed to queue workflow {} for execution: {}",
                                            workflow.name(),
                                            e
                                        );
                                    }
                                }
                            }
                            Ok(false) => {
                                // Workflow was not triggered
                                debug!(
                                    "Workflow {} was not triggered by timer event",
                                    workflow.name()
                                );
                            }
                            Err(e) => {
                                error!(
                                    "Error processing timer event for workflow {}: {}",
                                    workflow.name(),
                                    e
                                );
                            }
                        }
                    });
                }
            }
        }))
    }
}

impl Clone for WorkflowScheduler {
    fn clone(&self) -> Self {
        Self {
            device_registry: self.device_registry.clone(),
            workflows: self.workflows.clone(),
            executions: self.executions.clone(),
            event_tx: self.event_tx.clone(),
            device_event_rx: None, // Don't clone the receivers
            registry_event_rx: None,
            timer_tx: self.timer_tx.clone(),
            timer_rx: Mutex::new(None), // Don't clone the receiver
            tasks: Mutex::new(Vec::new()),
            running: self.running.clone(),
            max_concurrent: self.max_concurrent,
            default_timeout: self.default_timeout,
        }
    }
}
