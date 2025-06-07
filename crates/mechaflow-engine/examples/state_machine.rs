use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use mechaflow_core::types::{Id, Value};
use mechaflow_devices::registry::DeviceRegistry;

use mechaflow_engine::action::{LogAction, LogLevel};
use mechaflow_engine::context::BasicWorkflowContext;
use mechaflow_engine::state_machine::{
    EventId, PredicateGuard, State, StateMachine, StateMachineBuilder, StateId, Transition,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("Creating a thermostat state machine example...");

    // Create a device registry
    let device_registry = Arc::new(DeviceRegistry::new());

    // Create a workflow context for our state machine actions
    let context = Arc::new(BasicWorkflowContext::new(device_registry.clone()));

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

    let mut heating = State::new(&heating_state, "Heating");
    heating.set_description("Thermostat is actively heating");
    heating.add_entry_action(LogAction::new(
        "heating-entry",
        LogLevel::Info,
        "Starting to heat - activating heating element",
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

    // Initialize the state machine
    println!("Initializing state machine to idle state...");
    state_machine.initialize(&*context).await?;

    // Simulate a temperature decrease
    println!("\nSimulating temperature decrease event...");
    let next_state = state_machine
        .trigger_event(&temp_decreased, &*context)
        .await?;
    println!("  New state: {:?}", next_state);

    // Let's see what events are possible now
    let possible_events = state_machine.get_possible_events().await;
    println!("  Possible events from current state: {:?}", possible_events);

    // Simulate reaching target temperature
    tokio::time::sleep(Duration::from_secs(1)).await;
    println!("\nSimulating target reached event...");
    let next_state = state_machine
        .trigger_event(&target_reached, &*context)
        .await?;
    println!("  New state: {:?}", next_state);

    // Simulate a temperature increase
    tokio::time::sleep(Duration::from_secs(1)).await;
    println!("\nSimulating temperature increase event...");
    let next_state = state_machine
        .trigger_event(&temp_increased, &*context)
        .await?;
    println!("  New state: {:?}", next_state);

    // Go back to idle
    tokio::time::sleep(Duration::from_secs(1)).await;
    println!("\nSimulating target reached event again...");
    let next_state = state_machine
        .trigger_event(&target_reached, &*context)
        .await?;
    println!("  New state: {:?}", next_state);

    // Simulate an error
    tokio::time::sleep(Duration::from_secs(1)).await;
    println!("\nSimulating error detected event...");
    let next_state = state_machine
        .trigger_event(&error_detected, &*context)
        .await?;
    println!("  New state: {:?}", next_state);

    // Reset the error
    tokio::time::sleep(Duration::from_secs(1)).await;
    println!("\nSimulating reset event...");
    let next_state = state_machine.trigger_event(&reset, &*context).await?;
    println!("  New state: {:?}", next_state);

    // Print transition history
    println!("\nState machine transition history:");
    let history = state_machine.get_history().await;
    for (i, (from, event, to)) in history.iter().enumerate() {
        println!("{}. {} --[{}]--> {}", i + 1, from, event, to);
    }

    println!("\nState machine example completed successfully!");
    Ok(())
}
