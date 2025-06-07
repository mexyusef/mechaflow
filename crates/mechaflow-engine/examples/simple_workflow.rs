use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use mechaflow_core::types::{Id, Value};
use mechaflow_devices::device::{DeviceEvent, DeviceState};
use mechaflow_devices::registry::DeviceRegistry;

use mechaflow_engine::action::{ActionResult, DeviceAction, DelayAction, LogAction, LogLevel};
use mechaflow_engine::condition::{AndCondition, DeviceCondition, ComparisonOperator, TimeCondition};
use mechaflow_engine::context::BasicWorkflowContext;
use mechaflow_engine::trigger::{DeviceTrigger, ManualTriggerEvent, TriggerEvent};
use mechaflow_engine::workflow::{Workflow, WorkflowBuilder};
use mechaflow_engine::scheduler::WorkflowScheduler;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create a device registry
    let device_registry = Arc::new(DeviceRegistry::new());

    // Create a thermostat device (mocked for this example)
    let thermostat_id = Id::from("thermostat-001");
    let mut thermostat_props = HashMap::new();
    thermostat_props.insert("temperature".to_string(), Value::from(22.5));
    thermostat_props.insert("target_temperature".to_string(), Value::from(21.0));
    thermostat_props.insert("mode".to_string(), Value::from("cooling"));
    thermostat_props.insert("power".to_string(), Value::from("on"));

    // Create a mock device registry for the example
    // In a real application, you would register actual devices
    println!("Setting up mock devices for the example...");

    // For this example, we'll just simulate device events rather than
    // actually registering devices with implementations

    // Create a workflow to adjust thermostat when temperature is too high
    println!("Creating a temperature control workflow...");

    // Create the workflow using the builder pattern
    let workflow = WorkflowBuilder::new()
        .with_name("Temperature Control")
        .with_description("Adjust thermostat when temperature is too high")
        .with_trigger(
            DeviceTrigger::new("temp-trigger", thermostat_id.clone())
                .with_property("temperature")
        )
        .with_condition(
            AndCondition::new("temp-condition")
                .add_condition(
                    DeviceCondition::new(
                        "high-temp",
                        thermostat_id.clone(),
                        "temperature",
                        ComparisonOperator::Gt,
                        Value::from(24.0)
                    )
                )
                .add_condition(
                    DeviceCondition::new(
                        "power-on",
                        thermostat_id.clone(),
                        "power",
                        ComparisonOperator::Eq,
                        Value::from("on")
                    )
                )
        )
        .with_action(
            LogAction::new(
                "log-high-temp",
                LogLevel::Info,
                "Temperature is too high, adjusting thermostat"
            )
            .add_value("device_id", Value::from(thermostat_id.inner()))
        )
        .with_action(
            DeviceAction::new("set-temp", thermostat_id.clone())
                .set_property("target_temperature", Value::from(20.0))
        )
        .with_action(
            DelayAction::new("wait", Duration::from_secs(2))
        )
        .with_action(
            DeviceAction::new("set-mode", thermostat_id.clone())
                .set_property("mode", Value::from("cooling"))
        )
        .build()?;

    // Create a workflow scheduler
    println!("Creating workflow scheduler...");
    let scheduler = WorkflowScheduler::new(device_registry.clone());

    // Register the workflow
    println!("Registering workflow...");
    scheduler.register_workflow(Arc::new(workflow)).await?;

    // Start the scheduler
    println!("Starting workflow scheduler...");
    scheduler.start().await?;

    // Subscribe to scheduler events
    let mut event_rx = scheduler.subscribe();

    // Spawn a task to handle scheduler events
    tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            println!("Scheduler event: {:?}", event);
        }
    });

    // Trigger the workflow manually with a simulated device event
    println!("Simulating a temperature change event...");

    // Create a simulated device event
    let device_event = DeviceEvent::property_changed(
        thermostat_id.clone(),
        "temperature".to_string(),
        Value::from(25.5), // High temperature to trigger workflow
    );

    // Create a trigger event from the device event
    let trigger_event = TriggerEvent::Device(device_event);

    // Create a workflow context
    let context = BasicWorkflowContext::new(device_registry.clone());

    // Manually set the device properties in the context for testing
    // In a real application, these would come from actual devices
    let mut vars = HashMap::new();
    vars.insert("thermostat_id".to_string(), Value::from(thermostat_id.inner()));
    context.init_variables(vars).await;

    // Trigger the workflow manually
    println!("Manually triggering the workflow...");
    let exec_id = scheduler.trigger_workflow(
        scheduler.get_workflows().await?[0].id(),
        None
    ).await?;

    println!("Workflow triggered with execution ID: {}", exec_id);

    // Wait for workflow execution to complete
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Get execution result
    let execution = scheduler.get_execution(&exec_id).await?;
    println!("Workflow execution status: {:?}", execution.status());

    // Stop the scheduler
    println!("Stopping workflow scheduler...");
    scheduler.stop().await?;

    println!("Example completed successfully!");
    Ok(())
}
