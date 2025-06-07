# MechaFlow

A hybrid Rust-Python system for automation, robotics, and logistics.
(Incoomplete status - Work in progress)

## Overview

MechaFlow is an automation framework designed for high-reliability, real-time control systems.

It combines the performance and safety of Rust with the flexibility and ease of use of Python, creating a powerful platform for industrial automation, smart home systems, IoT applications, robotics, and more.

## Features

- **Core Rust Engine**: High-performance, reliable runtime built on async Rust and Tokio
- **Device Abstraction**: Unified interface for diverse hardware and protocols
- **Multi-Protocol Support**: MQTT, Modbus, OPC-UA, and more
- **Flexible Automation**: Combine workflows, state machines, and rules for complex logic
- **Python Integration**: Full Python API for maximum developer productivity
- **Safety First**: Strong error handling, validation, and fault recovery

## Architecture

MechaFlow consists of several modular components:

- **mechaflow-core**: Foundational types, utilities, and abstractions
- **mechaflow-devices**: Device management, protocol implementations, and device registry
- **mechaflow-engine**: Workflow engine, state machines, and rule-based automation
- **mechaflow-cli**: Command-line tools for project management
- **pymechaflow**: Python bindings for the MechaFlow ecosystem

## Quick Start

### Installation

```bash
# Coming soon
```

### Basic Usage

```rust
// Rust example
use mechaflow_core::types::Value;
use mechaflow_devices::registry::SharedDeviceRegistry;
use mechaflow_engine::workflow::WorkflowBuilder;
use mechaflow_engine::action::LogAction;
use mechaflow_engine::trigger::DevicePropertyTrigger;

// Create device registry
let registry = SharedDeviceRegistry::new();

// Build a workflow
let workflow = WorkflowBuilder::new()
    .with_id("my-workflow")
    .with_name("My First Workflow")
    .with_trigger(Box::new(DevicePropertyTrigger::new(
        "temp-trigger",
        device_id,
        "temperature"
    )))
    .with_action(Box::new(LogAction::new(
        "log-temp",
        LogLevel::Info,
        "Temperature changed!"
    )))
    .build()
    .expect("Failed to build workflow");
```

```python
# Python example
import asyncio
from pymechaflow.device import DeviceRegistry
from pymechaflow.engine import Workflow, RuleEngine, Rule

async def main():
    # Create device registry
    registry = DeviceRegistry()

    # Create a rule engine
    rule_engine = RuleEngine(registry)

    # Create a simple rule
    rule = Rule(
        name="Temperature Control",
        description="Adjust cooling when temperature is too high"
    )
    rule.enabled = True

    # Create a workflow
    workflow = Workflow(
        name="Temperature Monitor",
        description="Monitor and log temperature changes",
        enabled=True
    )

    # Print workflow details
    print(f"Workflow ID: {workflow.id}")
    print(f"Rule ID: {rule.id}")

    # Register workflow (when fully implemented)
    # scheduler = WorkflowScheduler(registry)
    # scheduler.register_workflow(workflow)
    # scheduler.start()

if __name__ == "__main__":
    asyncio.run(main())
```

## Example Applications

- Home automation system with temperature control
- Factory production line monitoring and control
- Agricultural sensor network with automated irrigation
- Robotics control systems with state-based behavior
- IoT device fleet management and monitoring

## Supported Platforms

- Linux (Debian, Ubuntu, Raspberry Pi OS)
- Windows 10/11
- macOS
- Embedded Linux platforms

## Development Status

MechaFlow is currently under active development. The core Rust implementation including the engine, device abstractions, and workflow system are functional. Python bindings have been implemented for the core functionality, providing access to device registry, workflow engine, and rule system.

## License

MechaFlow is licensed under the MIT License.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
