use std::sync::{Arc, RwLock};
use std::time::Duration;

use mechaflow_core::types::Value;
use mechaflow_devices::{
    DeviceCapability, DeviceEvent, DeviceRegistry, DeviceState, Protocol, ProtocolOptions,
    ProtocolRegistry, SharedDeviceRegistry,
};
use mechaflow_devices::devices::ThermostatDevice;
use mechaflow_devices::protocols::MqttProtocolProvider;

use tokio::time::sleep;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the logger
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Create protocol registry
    let protocol_registry = Arc::new(RwLock::new(ProtocolRegistry::new()));

    // Register MQTT protocol provider
    {
        let mut registry = protocol_registry.write().unwrap();
        registry.register_provider(MqttProtocolProvider::new());
    }

    // Create device registry
    let device_registry = SharedDeviceRegistry::new();

    // Create a thermostat device
    let thermostat = ThermostatDevice::new(
        None,
        "Living Room Thermostat".to_string(),
        protocol_registry.clone(),
        "mqtt".to_string(),
        "mqtt://host=localhost;port=1883".to_string(),
    );

    // Register the device
    device_registry.register_device(thermostat)?;

    // Get the registered device
    let device_id = device_registry
        .get_device_ids()
        .into_iter()
        .next()
        .expect("Failed to get device ID");

    let device = device_registry
        .get_device(&device_id)
        .expect("Failed to get device");

    // Subscribe to device events
    let mut event_rx = device.subscribe_events().await?;

    // Spawn a task to handle device events
    tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            match event {
                DeviceEvent::Connected => {
                    info!("Device connected event received");
                }
                DeviceEvent::Disconnected => {
                    info!("Device disconnected event received");
                }
                DeviceEvent::StateChanged { state } => {
                    info!("Device state changed to {:?}", state);
                }
                DeviceEvent::PropertyChanged { property, value } => {
                    info!("Property changed: {} = {:?}", property, value);
                }
                _ => {
                    info!("Other event received: {:?}", event);
                }
            }
        }
    });

    // Connect the device
    info!("Connecting device...");
    device.connect().await?;

    // Wait for the device to connect
    sleep(Duration::from_secs(2)).await;

    // Print device info
    info!("Device Info: {:?}", device.info());
    info!("Device State: {:?}", device.state());
    info!("Device Capabilities: {:?}", device.capabilities());

    // Read properties
    let temp = device.read_property("temperature").await?;
    let target = device.read_property("targetTemperature").await?;
    let mode = device.read_property("mode").await?;
    let fan = device.read_property("fanOn").await?;
    let humidity = device.read_property("humidity").await?;

    info!("Temperature: {:?}", temp);
    info!("Target Temperature: {:?}", target);
    info!("Mode: {:?}", mode);
    info!("Fan: {:?}", fan);
    info!("Humidity: {:?}", humidity);

    // Set to heating mode
    info!("Setting to heating mode...");
    device
        .write_property("mode", Value::String("heat".to_string()))
        .await?;

    // Set target temperature
    info!("Setting target temperature to 23°C...");
    device
        .write_property("targetTemperature", Value::Float(23.0))
        .await?;

    // Let the device run for a while (temperature should increase)
    info!("Running for 30 seconds...");
    for i in 0..6 {
        sleep(Duration::from_secs(5)).await;

        let temp = device.read_property("temperature").await?;
        let state = device.state();

        info!(
            "After {} seconds: Temperature: {:?}, State: {:?}",
            (i + 1) * 5,
            temp,
            state
        );
    }

    // Execute a command
    info!("Executing getStatus command...");
    let status = device
        .execute_command("getStatus", std::collections::HashMap::new())
        .await?;

    info!("Status: {:?}", status);

    // Set to cooling mode
    info!("Setting to cooling mode...");
    device
        .write_property("mode", Value::String("cool".to_string()))
        .await?;

    // Let the device run for a while (temperature should decrease)
    info!("Running for 15 seconds...");
    for i in 0..3 {
        sleep(Duration::from_secs(5)).await;

        let temp = device.read_property("temperature").await?;
        let state = device.state();

        info!(
            "After {} seconds: Temperature: {:?}, State: {:?}",
            (i + 1) * 5,
            temp,
            state
        );
    }

    // Turn off the device
    info!("Turning off the device...");
    device
        .write_property("mode", Value::String("off".to_string()))
        .await?;

    // Let the device run for a while (temperature should stabilize)
    info!("Running for 10 seconds...");
    sleep(Duration::from_secs(10)).await;

    // Read final state
    let temp = device.read_property("temperature").await?;
    let target = device.read_property("targetTemperature").await?;
    let mode = device.read_property("mode").await?;
    let fan = device.read_property("fanOn").await?;
    let state = device.state();

    info!("Final state:");
    info!("Temperature: {:?}", temp);
    info!("Target Temperature: {:?}", target);
    info!("Mode: {:?}", mode);
    info!("Fan: {:?}", fan);
    info!("State: {:?}", state);

    // Disconnect the device
    info!("Disconnecting device...");
    device.disconnect().await?;

    // Wait for disconnect to complete
    sleep(Duration::from_secs(1)).await;

    info!("Example completed!");

    Ok(())
}
