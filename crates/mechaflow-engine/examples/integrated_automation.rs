use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use mechaflow_core::types::{Id, Value};
use mechaflow_devices::device::{BaseDevice, Device, DeviceCapability, DeviceState};
use mechaflow_devices::registry::{DeviceRegistry, SharedDeviceRegistry};
use mechaflow_devices::types::DeviceType;

use mechaflow_engine::action::{Action, ActionId, DelayAction, DevicePropertyAction, LogAction, LogLevel};
use mechaflow_engine::condition::{
    AndCondition, Condition, ConditionId, DevicePropertyCondition, ComparisonOperator,
};
use mechaflow_engine::context::BasicWorkflowContext;
use mechaflow_engine::rules::{Rule, RuleBuilder, RuleEngine, RuleId};
use mechaflow_engine::state_machine::{
    EventId, State, StateMachine, StateMachineBuilder, StateId, Transition,
};
use mechaflow_engine::trigger::{DevicePropertyTrigger, Trigger, TriggerId};
use mechaflow_engine::workflow::{Workflow, WorkflowBuilder, WorkflowId, WorkflowScheduler};

// Mock thermostat device implementation
struct MockThermostat {
    base: BaseDevice,
}

impl MockThermostat {
    fn new(id: &str) -> Self {
        let mut props = HashMap::new();
        props.insert("temperature".to_string(), Value::Float(21.0));
        props.insert("target_temperature".to_string(), Value::Float(20.0));
        props.insert("mode".to_string(), Value::String("auto".to_string()));
        props.insert("power".to_string(), Value::Bool(true));
        props.insert("humidity".to_string(), Value::Float(45.0));
        props.insert("error_code".to_string(), Value::Int(0));

        let mut base = BaseDevice::new(Id::new(id));
        base.set_name("Living Room Thermostat");
        base.set_description("Mock thermostat device for testing");
        base.set_device_type(DeviceType::Thermostat);
        base.add_capability(DeviceCapability::Temperature);
        base.add_capability(DeviceCapability::Humidity);
        base.add_capability(DeviceCapability::Switch);
        base.set_properties(props);
        base.set_state(DeviceState::Ready);

        Self { base }
    }
}

#[async_trait::async_trait]
impl Device for MockThermostat {
    fn get_id(&self) -> &Id {
        self.base.get_id()
    }

    fn get_name(&self) -> &str {
        self.base.get_name()
    }

    fn get_description(&self) -> Option<&str> {
        self.base.get_description()
    }

    fn get_device_type(&self) -> DeviceType {
        self.base.get_device_type()
    }

    fn get_capabilities(&self) -> &[DeviceCapability] {
        self.base.get_capabilities()
    }

    fn has_capability(&self, capability: DeviceCapability) -> bool {
        self.base.has_capability(capability)
    }

    async fn get_state(&self) -> DeviceState {
        self.base.get_state().await
    }

    async fn initialize(&self) -> mechaflow_core::error::Result<()> {
        self.base.initialize().await
    }

    async fn connect(&self) -> mechaflow_core::error::Result<()> {
        self.base.connect().await
    }

    async fn disconnect(&self) -> mechaflow_core::error::Result<()> {
        self.base.disconnect().await
    }

    async fn shutdown(&self) -> mechaflow_core::error::Result<()> {
        self.base.shutdown().await
    }

    async fn get_property(&self, name: &str) -> Option<Value> {
        self.base.get_property(name).await
    }

    async fn get_properties(&self) -> HashMap<String, Value> {
        self.base.get_properties().await
    }

    async fn set_property(&self, name: &str, value: Value) -> mechaflow_core::error::Result<()> {
        self.base.set_property(name, value).await
    }

    async fn execute_command(
        &self,
        command: &str,
        args: HashMap<String, Value>,
    ) -> mechaflow_core::error::Result<Value> {
        self.base.execute_command(command, args).await
    }

    async fn subscribe_to_events(&self) -> mechaflow_core::error::Result<tokio::sync::broadcast::Receiver<mechaflow_devices::device::DeviceEvent>> {
        self.base.subscribe_to_events().await
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("Starting integrated automation example...");

    // Create device registry and device
    let device_registry = SharedDeviceRegistry::new();
    let thermostat = Arc::new(MockThermostat::new("thermostat-001"));
    device_registry.register_device(thermostat.clone()).await?;

    // Create context for actions and automation components
    let context = Arc::new(BasicWorkflowContext::new(device_registry.clone()));

    println!("\n=============================================");
    println!("1. SETTING UP STATE MACHINE FOR THERMOSTAT MODE");
    println!("=============================================");

    // Create state machine for thermostat mode management
    let state_machine = create_thermostat_state_machine(thermostat.get_id()).await?;

    // Initialize the state machine
    println!("Initializing state machine to idle state...");
    state_machine.initialize(&*context).await?;

    println!("\n=============================================");
    println!("2. SETTING UP RULES ENGINE FOR EMERGENCY HANDLING");
    println!("=============================================");

    // Create rule engine for handling error conditions
    let mut rule_engine = RuleEngine::new(context.clone());

    // Create and add error handling rule
    let error_rule = create_error_rule(thermostat.get_id());
    rule_engine.add_rule(error_rule).await?;

    // Print registered rules
    println!("Registered rules:");
    for rule in rule_engine.get_rules().await {
        println!("  - {} ({}): {}", rule.get_name(), rule.get_id(), rule.get_description().unwrap_or("No description"));
    }

    println!("\n=============================================");
    println!("3. SETTING UP WORKFLOW FOR TEMPERATURE MANAGEMENT");
    println!("=============================================");

    // Create and start workflow scheduler
    let workflow_scheduler = Arc::new(WorkflowScheduler::new(context.clone()));

    // Create temperature management workflow
    let temp_workflow = create_temperature_workflow(thermostat.get_id());

    // Register and start the workflow
    let workflow_id = temp_workflow.get_id().clone();
    workflow_scheduler.register_workflow(temp_workflow).await?;
    workflow_scheduler.start_workflow(&workflow_id).await?;

    println!("Started workflow: {}", workflow_id);

    // Subscribe to scheduler events
    let mut scheduler_events = workflow_scheduler.subscribe().await;

    // Spawn a task to handle scheduler events
    tokio::spawn(async move {
        while let Ok(event) = scheduler_events.recv().await {
            println!("Workflow event: {:?}", event);
        }
    });

    println!("\n=============================================");
    println!("4. DEMONSTRATION SCENARIO");
    println!("=============================================");

    // Now run through a demonstration scenario

    // Start with normal temperature
    println!("\nInitial thermostat state:");
    print_device_properties(&*thermostat).await;

    // 1. Trigger high temperature to activate workflow
    println!("\nSimulating temperature increase to 26°C...");
    thermostat.set_property("temperature", Value::Float(26.0)).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // 2. Trigger state machine transition
    println!("\nTriggering state machine to 'cooling' mode...");
    let temp_increased = EventId::new("temp_increased");
    let next_state = state_machine.trigger_event(&temp_increased, &*context).await?;
    println!("  New state: {:?}", next_state);

    // Check current thermostat state
    print_device_properties(&*thermostat).await;

    // 3. Trigger error condition to activate rule
    println!("\nSimulating device error (error code 101)...");
    thermostat.set_property("error_code", Value::Int(101)).await?;

    // Evaluate rules
    rule_engine.evaluate_all_rules().await?;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Check current thermostat state
    print_device_properties(&*thermostat).await;

    // 4. Reset error and return to normal operation
    println!("\nResetting error condition...");
    thermostat.set_property("error_code", Value::Int(0)).await?;

    // Reset state machine
    println!("Resetting state machine to idle mode...");
    let reset = EventId::new("reset");
    let next_state = state_machine.trigger_event(&reset, &*context).await?;
    println!("  New state: {:?}", next_state);

    // Temperature back to normal
    println!("Returning temperature to normal (21°C)...");
    thermostat.set_property("temperature", Value::Float(21.0)).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Final state
    println!("\nFinal thermostat state:");
    print_device_properties(&*thermostat).await;

    // Print state machine transition history
    println!("\nState machine transition history:");
    let history = state_machine.get_history().await;
    for (i, (from, event, to)) in history.iter().enumerate() {
        println!("  {}. {} --[{}]--> {}", i + 1, from, event, to);
    }

    // Shutdown the workflow scheduler
    println!("\nStopping automation components...");
    workflow_scheduler.stop_workflow(&workflow_id).await?;

    println!("\nIntegrated automation example completed successfully!");
    Ok(())
}

// Helper function to create thermostat state machine
async fn create_thermostat_state_machine(
    device_id: &Id,
) -> Result<Arc<StateMachine>, Box<dyn std::error::Error>> {
    // Define state IDs
    let idle_state = StateId::new("idle");
    let heating_state = StateId::new("heating");
    let cooling_state = StateId::new("cooling");
    let error_state = StateId::new("error");

    // Define event IDs
    let temp_increased = EventId::new("temp_increased");
    let temp_decreased = EventId::new("temp_decreased");
    let target_reached = EventId::new("target_reached");
    let error_detected = EventId::new("error_detected");
    let reset = EventId::new("reset");

    // Create states
    let mut idle = State::new(&idle_state, "Idle");
    idle.set_description("Thermostat is idle, maintaining current temperature");
    idle.add_entry_action(LogAction::new(
        "idle-entry",
        LogLevel::Info,
        "Entering idle state",
    ));
    idle.add_entry_action(DevicePropertyAction::new(
        "set-mode-auto",
        device_id.clone(),
        "mode",
        Value::String("auto".to_string()),
    ));

    let mut heating = State::new(&heating_state, "Heating");
    heating.set_description("Thermostat is actively heating");
    heating.add_entry_action(LogAction::new(
        "heating-entry",
        LogLevel::Info,
        "Starting to heat - activating heating element",
    ));
    heating.add_entry_action(DevicePropertyAction::new(
        "set-mode-heating",
        device_id.clone(),
        "mode",
        Value::String("heating".to_string()),
    ));
    heating.add_exit_action(LogAction::new(
        "heating-exit",
        LogLevel::Info,
        "Stopping heating",
    ));

    let mut cooling = State::new(&cooling_state, "Cooling");
    cooling.set_description("Thermostat is actively cooling");
    cooling.add_entry_action(LogAction::new(
        "cooling-entry",
        LogLevel::Info,
        "Starting to cool - activating cooling system",
    ));
    cooling.add_entry_action(DevicePropertyAction::new(
        "set-mode-cooling",
        device_id.clone(),
        "mode",
        Value::String("cooling".to_string()),
    ));
    cooling.add_exit_action(LogAction::new(
        "cooling-exit",
        LogLevel::Info,
        "Stopping cooling",
    ));

    let mut error = State::new(&error_state, "Error");
    error.set_description("Thermostat is in error state");
    error.add_entry_action(LogAction::new(
        "error-entry",
        LogLevel::Error,
        "Thermostat error detected!",
    ));
    error.add_entry_action(DevicePropertyAction::new(
        "set-mode-off",
        device_id.clone(),
        "power",
        Value::Bool(false),
    ));

    // Create transitions
    let mut idle_to_heating = Transition::new(&idle_state, &heating_state, &temp_decreased);
    idle_to_heating.add_action(LogAction::new(
        "to-heating-action",
        LogLevel::Info,
        "Temperature below target, starting heating",
    ));

    let mut idle_to_cooling = Transition::new(&idle_state, &cooling_state, &temp_increased);
    idle_to_cooling.add_action(LogAction::new(
        "to-cooling-action",
        LogLevel::Info,
        "Temperature above target, starting cooling",
    ));

    let heating_to_idle = Transition::new(&heating_state, &idle_state, &target_reached);
    let cooling_to_idle = Transition::new(&cooling_state, &idle_state, &target_reached);

    let mut any_to_error = Transition::new(&idle_state, &error_state, &error_detected);
    any_to_error.add_action(LogAction::new(
        "to-error-action",
        LogLevel::Error,
        "Transitioning to error state due to detected issue",
    ));

    let error_to_idle = Transition::new(&error_state, &idle_state, &reset);

    // Build the state machine
    let state_machine = StateMachineBuilder::new()
        .with_id("thermostat-state-machine")
        .with_name("Thermostat Control")
        .with_description("Controls thermostat heating and cooling cycles")
        .with_state(idle)
        .with_state(heating)
        .with_state(cooling)
        .with_state(error)
        .with_transition(idle_to_heating)
        .with_transition(idle_to_cooling)
        .with_transition(heating_to_idle)
        .with_transition(cooling_to_idle)
        .with_transition(any_to_error)
        .with_transition(error_to_idle)
        .with_initial_state(&idle_state)
        .build()?;

    Ok(state_machine)
}

// Helper function to create error handling rule
fn create_error_rule(device_id: &Id) -> Rule {
    // Create condition for error condition
    let error_condition = DevicePropertyCondition::new(
        "error-detected",
        device_id.clone(),
        "error_code",
        ComparisonOperator::GreaterThan,
        Value::Int(0)
    );

    // Actions to take when error occurs
    let log_action = LogAction::new(
        "log-error",
        LogLevel::Error,
        "CRITICAL: Thermostat error detected! Emergency protocol activated."
    );

    let power_off_action = DevicePropertyAction::new(
        "power-off",
        device_id.clone(),
        "power",
        Value::Bool(false)
    );

    // Build the rule
    RuleBuilder::new()
        .with_id("emergency-shutdown")
        .with_name("Emergency Shutdown Rule")
        .with_description("Automatically shut down thermostat when an error is detected")
        .with_condition(Box::new(error_condition))
        .with_action(Box::new(log_action) as Box<dyn Action>)
        .with_action(Box::new(power_off_action) as Box<dyn Action>)
        .with_enabled(true)
        .build()
        .expect("Failed to build rule")
}

// Helper function to create temperature management workflow
fn create_temperature_workflow(device_id: &Id) -> Workflow {
    // Create trigger based on temperature change
    let temp_trigger = DevicePropertyTrigger::new(
        "temp-trigger",
        device_id.clone(),
        "temperature"
    );

    // Create condition to check if temperature is high
    let high_temp_condition = DevicePropertyCondition::new(
        "high-temp-condition",
        device_id.clone(),
        "temperature",
        ComparisonOperator::GreaterThan,
        Value::Float(25.0)
    );

    // Create condition to check if device is powered on
    let power_on_condition = DevicePropertyCondition::new(
        "power-on-condition",
        device_id.clone(),
        "power",
        ComparisonOperator::Equal,
        Value::Bool(true)
    );

    // Combined condition
    let and_condition = AndCondition::new(
        "high-temp-and-on",
        vec![
            Box::new(high_temp_condition) as Box<dyn Condition>,
            Box::new(power_on_condition) as Box<dyn Condition>,
        ]
    );

    // Actions to take when high temperature detected
    let log_action = LogAction::new(
        "log-high-temp",
        LogLevel::Info,
        "High temperature detected, adjusting target temperature"
    );

    let set_target_action = DevicePropertyAction::new(
        "lower-target-temp",
        device_id.clone(),
        "target_temperature",
        Value::Float(20.0)
    );

    let delay_action = DelayAction::new("delay", Duration::from_secs(1));

    // Build the workflow
    WorkflowBuilder::new()
        .with_id("temperature-management")
        .with_name("Temperature Management")
        .with_description("Manages temperature by adjusting thermostat settings")
        .with_trigger(Box::new(temp_trigger) as Box<dyn Trigger>)
        .with_condition(Box::new(and_condition))
        .with_action(Box::new(log_action) as Box<dyn Action>)
        .with_action(Box::new(set_target_action) as Box<dyn Action>)
        .with_action(Box::new(delay_action) as Box<dyn Action>)
        .build()
        .expect("Failed to build workflow")
}

// Helper function to print device properties
async fn print_device_properties(device: &dyn Device) {
    println!("Current device state:");
    let props = device.get_properties().await;
    for (key, value) in props.iter() {
        println!("  - {}: {:?}", key, value);
    }
}
