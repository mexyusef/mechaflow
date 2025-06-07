#!/usr/bin/env python3
"""
Simple example demonstrating basic usage of the MechaFlow Python bindings.
"""
import asyncio
import logging
import time
from pymechaflow.device import DeviceId, DeviceInfo, DeviceProperty, DeviceRegistry
from pymechaflow.engine import Workflow, RuleEngine, Rule

# Set up logging
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
    handlers=[logging.StreamHandler()]
)
logger = logging.getLogger(__name__)

async def main():
    logger.info("Starting MechaFlow Python example")

    # Create device registry
    registry = DeviceRegistry()
    logger.info("Created device registry")

    # Since we can't register Python-defined devices directly yet,
    # we'll use a mock approach with direct property access.
    # In a real implementation, Python callbacks would handle device methods.

    # Set up a rule engine
    rule_engine = RuleEngine(registry)
    logger.info("Created rule engine")

    # Create a simple rule
    rule = Rule(
        id=None,  # Auto-generate ID
        name="Example Rule",
        description="A simple example rule"
    )
    logger.info(f"Created rule: {rule}")

    # Set rule properties
    rule.enabled = True
    logger.info(f"Rule enabled: {rule.enabled}")
    logger.info(f"Rule ID: {rule.id}")

    # Create a workflow
    workflow = Workflow(
        id=None,  # Auto-generate ID
        name="Example Workflow",
        description="A simple example workflow",
        enabled=True
    )
    logger.info(f"Created workflow: {workflow}")

    # Log workflow properties
    logger.info(f"Workflow ID: {workflow.id}")
    logger.info(f"Workflow name: {workflow.name}")
    logger.info(f"Workflow description: {workflow.description}")
    logger.info(f"Workflow enabled: {workflow.enabled}")

    # Demonstrate changing properties
    workflow.name = "Updated Workflow Name"
    workflow.description = "This description has been updated"
    logger.info(f"Updated workflow name: {workflow.name}")
    logger.info(f"Updated workflow description: {workflow.description}")

    # Note on future implementation:
    logger.info("\nFuture implementation will include:")
    logger.info("1. Registering Python-defined devices")
    logger.info("2. Defining triggers, conditions and actions in Python")
    logger.info("3. Executing workflows with Python-defined components")
    logger.info("4. Subscribing to device and workflow events")

    logger.info("\nMechaFlow Python example completed")

if __name__ == "__main__":
    asyncio.run(main())
