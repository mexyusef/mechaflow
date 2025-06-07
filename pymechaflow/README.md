# PyMechaFlow

Python bindings for the MechaFlow automation framework.
(Incoomplete status - Work in progress)

## Overview

PyMechaFlow provides Python access to the core functionality of the MechaFlow automation framework, which is written in Rust. It allows you to:

- Manage and interact with IoT devices
- Create automation workflows
- Define rules for device control
- Build state machines for complex automation sequences

## Installation

### Prerequisites

- Python 3.7 or higher
- Rust 1.55 or higher

### Install from PyPI (coming soon)

```bash
pip install pymechaflow
```

### Install from source

```bash
git clone https://github.com/mexyusef/mechaflow.git
cd mechaflow/pymechaflow
pip install -e .
```

## Quick Start

Here's a simple example that creates a device registry and a workflow:

```python
import asyncio
from pymechaflow.device import DeviceRegistry
from pymechaflow.engine import Workflow, RuleEngine

async def main():
    # Create device registry
    registry = DeviceRegistry()

    # Create a workflow
    workflow = Workflow(
        name="My Workflow",
        description="A simple example workflow",
        enabled=True
    )

    # Print workflow details
    print(f"Workflow ID: {workflow.id}")
    print(f"Workflow name: {workflow.name}")

    # Create a rule engine
    rule_engine = RuleEngine(registry)

    # More functionality coming soon...

if __name__ == "__main__":
    asyncio.run(main())
```

## Current Limitations

The bindings are currently in early development. Some features are not yet fully implemented:

- Device registration from Python (coming soon)
- Python-defined triggers, conditions, and actions
- Event callbacks from Rust to Python
- Complete workflow execution from Python

## API Documentation

API documentation is currently under development.

## License

This project is licensed under the MIT License - see the LICENSE file for details.
