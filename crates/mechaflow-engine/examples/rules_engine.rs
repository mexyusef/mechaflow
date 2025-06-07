use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use mechaflow_core::types::{Id, Value};
use mechaflow_devices::device::{BaseDevice, Device, DeviceCapability, DeviceState};
use mechaflow_devices::registry::{DeviceRegistry, SharedDeviceRegistry};
use mechaflow_devices::types::DeviceType;

use mechaflow_engine::action::{Action, ActionId, DevicePropertyAction, LogAction, LogLevel};
use mechaflow_engine::condition::{
    AndCondition, Condition, ConditionId, DevicePropertyCondition, ComparisonOperator,
};
use mechaflow_engine::context::BasicWorkflowContext;
use mechaflow_engine::rules::{Rule, RuleBuilder, RuleEngine, RuleId};

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

    async fn set_temperature(&mut self, temp: f64) {
        self.base.set_property("temperature", Value::Float(temp)).await;
    }

    async fn set_humidity(&mut self, humidity: f64) {
        self.base.set_property("humidity", Value::Float(humidity)).await;
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

    println!("Starting rules engine example...");

    // Create device registry
    let device_registry = SharedDeviceRegistry::new();

    // Create mock thermostat device
    let thermostat = Arc::new(MockThermostat::new("thermostat-001"));
    device_registry.register_device(thermostat.clone()).await?;

    let mut thermostat_mut = MockThermostat::new("thermostat-001");

    // Create context for actions
    let context = Arc::new(BasicWorkflowContext::new(device_registry.clone()));

    // Create rule engine
    let mut rule_engine = RuleEngine::new(context.clone());

    // Subscribe to rule engine events
    let mut rule_events = rule_engine.subscribe().await;

    // Spawn a task to listen for rule events
    tokio::spawn(async move {
        while let Ok(event) = rule_events.recv().await {
            println!("Rule engine event: {:?}", event);
        }
    });

    // Create conditions
    let high_temp_condition = DevicePropertyCondition::new(
        "high-temp",
        thermostat.get_id().clone(),
        "temperature",
        ComparisonOperator::GreaterThan,
        Value::Float(25.0)
    );

    let high_humidity_condition = DevicePropertyCondition::new(
        "high-humidity",
        thermostat.get_id().clone(),
        "humidity",
        ComparisonOperator::GreaterThan,
        Value::Float(60.0)
    );

    let power_on_condition = DevicePropertyCondition::new(
        "power-on",
        thermostat.get_id().clone(),
        "power",
        ComparisonOperator::Equal,
        Value::Bool(true)
    );

    // Combined condition using AND
    let hot_and_humid = AndCondition::new(
        "hot-and-humid",
        vec![
            Box::new(high_temp_condition.clone()) as Box<dyn Condition>,
            Box::new(high_humidity_condition.clone()) as Box<dyn Condition>,
            Box::new(power_on_condition.clone()) as Box<dyn Condition>,
        ]
    );

    // Create actions
    let log_action = LogAction::new(
        "log-climate",
        LogLevel::Info,
        "High temperature and humidity detected! Adjusting thermostat..."
    );

    let cooling_action = DevicePropertyAction::new(
        "set-cooling",
        thermostat.get_id().clone(),
        "mode",
        Value::String("cooling".to_string())
    );

    let target_temp_action = DevicePropertyAction::new(
        "lower-target",
        thermostat.get_id().clone(),
        "target_temperature",
        Value::Float(20.0)
    );

    // Build rules using RuleBuilder
    let climate_control_rule = RuleBuilder::new()
        .with_id("climate-control")
        .with_name("Climate Control Rule")
        .with_description("Automatically adjust thermostat when temperature and humidity are high")
        .with_condition(Box::new(hot_and_humid))
        .with_action(Box::new(log_action) as Box<dyn Action>)
        .with_action(Box::new(cooling_action) as Box<dyn Action>)
        .with_action(Box::new(target_temp_action) as Box<dyn Action>)
        .with_enabled(true)
        .build()?;

    // Add rule to engine
    rule_engine.add_rule(climate_control_rule).await?;

    // Print registered rules
    println!("Registered rules:");
    for rule in rule_engine.get_rules().await {
        println!("  - {} ({}): {}", rule.get_name(), rule.get_id(), rule.get_description().unwrap_or("No description"));
    }

    // Simulate normal conditions
    println!("\nSimulating normal conditions (21°C, 45% humidity)");
    rule_engine.evaluate_all_rules().await?;

    // Simulate high temperature
    println!("\nSimulating high temperature (26°C)");
    thermostat_mut.set_temperature(26.0).await;
    thermostat.set_property("temperature", Value::Float(26.0)).await?;
    rule_engine.evaluate_all_rules().await?;

    // Simulate high temperature and humidity
    println!("\nSimulating high temperature and humidity (26°C, 65% humidity)");
    thermostat_mut.set_humidity(65.0).await;
    thermostat.set_property("humidity", Value::Float(65.0)).await?;
    rule_engine.evaluate_all_rules().await?;

    // Small delay to allow for async processing
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Check current thermostat state
    println!("\nCurrent thermostat state:");
    let props = thermostat.get_properties().await;
    for (key, value) in props.iter() {
        println!("  - {}: {:?}", key, value);
    }

    // Disable the rule
    println!("\nDisabling the climate control rule");
    rule_engine.disable_rule(&RuleId::new("climate-control")).await?;

    // Set conditions that would trigger the rule
    println!("\nSimulating extreme conditions with rule disabled (30°C, 80% humidity)");
    thermostat.set_property("temperature", Value::Float(30.0)).await?;
    thermostat.set_property("humidity", Value::Float(80.0)).await?;

    // Evaluate rules again - should not trigger because rule is disabled
    rule_engine.evaluate_all_rules().await?;

    // Check current thermostat state - should be unchanged
    println!("\nCurrent thermostat state after disabled rule evaluation:");
    let props = thermostat.get_properties().await;
    for (key, value) in props.iter() {
        println!("  - {}: {:?}", key, value);
    }

    println!("\nRules engine example completed successfully!");
    Ok(())
}
