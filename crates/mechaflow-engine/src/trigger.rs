/*!
 * Triggers that initiate workflows.
 *
 * This module defines the triggers that can start workflow executions
 * in response to various events in the MechaFlow system.
 */
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use mechaflow_core::types::{Id, Value};
use mechaflow_devices::device::{Device, DeviceEvent, DeviceState};

use crate::error::Result;

/// A trigger event that can start a workflow
#[derive(Debug, Clone)]
pub enum TriggerEvent {
    /// Event from a device
    Device(DeviceEvent),
    /// Timer event
    Timer(TimerEvent),
    /// Webhook event
    Webhook(WebhookEvent),
    /// Manual trigger
    Manual(ManualTriggerEvent),
    /// State change event
    StateChange(StateChangeEvent),
    /// Custom event
    Custom(CustomEvent),
}

impl TriggerEvent {
    /// Get the event type as a string
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::Device(_) => "device",
            Self::Timer(_) => "timer",
            Self::Webhook(_) => "webhook",
            Self::Manual(_) => "manual",
            Self::StateChange(_) => "state_change",
            Self::Custom(_) => "custom",
        }
    }

    /// Get the event timestamp
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            Self::Device(e) => e.timestamp(),
            Self::Timer(e) => e.timestamp,
            Self::Webhook(e) => e.timestamp,
            Self::Manual(e) => e.timestamp,
            Self::StateChange(e) => e.timestamp,
            Self::Custom(e) => e.timestamp,
        }
    }
}

/// A timer event
#[derive(Debug, Clone)]
pub struct TimerEvent {
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Timer ID
    pub timer_id: String,
    /// Timer name
    pub timer_name: Option<String>,
    /// Additional data
    pub data: HashMap<String, Value>,
}

/// A webhook event
#[derive(Debug, Clone)]
pub struct WebhookEvent {
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Webhook ID
    pub webhook_id: String,
    /// Webhook path
    pub path: String,
    /// HTTP method
    pub method: String,
    /// Headers
    pub headers: HashMap<String, String>,
    /// Query parameters
    pub query_params: HashMap<String, String>,
    /// Request body
    pub body: Option<Value>,
}

/// A manual trigger event
#[derive(Debug, Clone)]
pub struct ManualTriggerEvent {
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// User ID that triggered the event
    pub user_id: Option<String>,
    /// Additional data
    pub data: HashMap<String, Value>,
}

/// A state change event
#[derive(Debug, Clone)]
pub struct StateChangeEvent {
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Entity type
    pub entity_type: String,
    /// Entity ID
    pub entity_id: Id,
    /// Previous state
    pub previous_state: Value,
    /// New state
    pub new_state: Value,
}

/// A custom event
#[derive(Debug, Clone)]
pub struct CustomEvent {
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Event type
    pub event_type: String,
    /// Source of the event
    pub source: String,
    /// Event data
    pub data: Value,
}

/// A trigger that can start a workflow
#[async_trait]
pub trait Trigger: fmt::Debug + Send + Sync {
    /// Get the trigger type
    fn trigger_type(&self) -> &'static str;

    /// Check if the trigger matches the given event
    async fn matches_event(&self, event: &TriggerEvent) -> Result<bool>;

    /// Get the trigger ID
    fn id(&self) -> &str;

    /// Get the trigger description
    fn description(&self) -> Option<&str>;
}

/// A device trigger that starts a workflow when a device event occurs
#[derive(Debug, Clone)]
pub struct DeviceTrigger {
    /// Trigger ID
    id: String,
    /// Trigger description
    description: Option<String>,
    /// Device ID to monitor
    device_id: Id,
    /// Device event type to monitor
    event_type: Option<String>,
    /// Device state to monitor
    state: Option<DeviceState>,
    /// Property to monitor
    property: Option<String>,
    /// Property value to match
    property_value: Option<Value>,
}

impl DeviceTrigger {
    /// Create a new device trigger
    pub fn new<S: Into<String>>(id: S, device_id: Id) -> Self {
        Self {
            id: id.into(),
            description: None,
            device_id,
            event_type: None,
            state: None,
            property: None,
            property_value: None,
        }
    }

    /// Set the trigger description
    pub fn with_description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the event type to monitor
    pub fn with_event_type<S: Into<String>>(mut self, event_type: S) -> Self {
        self.event_type = Some(event_type.into());
        self
    }

    /// Set the state to monitor
    pub fn with_state(mut self, state: DeviceState) -> Self {
        self.state = Some(state);
        self
    }

    /// Set the property to monitor
    pub fn with_property<S: Into<String>>(mut self, property: S) -> Self {
        self.property = Some(property.into());
        self
    }

    /// Set the property value to match
    pub fn with_property_value<V: Into<Value>>(mut self, value: V) -> Self {
        self.property_value = Some(value.into());
        self
    }
}

#[async_trait]
impl Trigger for DeviceTrigger {
    fn trigger_type(&self) -> &'static str {
        "device"
    }

    async fn matches_event(&self, event: &TriggerEvent) -> Result<bool> {
        match event {
            TriggerEvent::Device(device_event) => {
                // Check if device ID matches
                if device_event.device_id() != &self.device_id {
                    return Ok(false);
                }

                // Check if event type matches
                if let Some(ref event_type) = self.event_type {
                    if device_event.event_type() != event_type {
                        return Ok(false);
                    }
                }

                // Check if state matches
                if let Some(ref state) = self.state {
                    if let Some(device_state) = device_event.state() {
                        if device_state != state {
                            return Ok(false);
                        }
                    } else {
                        return Ok(false);
                    }
                }

                // Check if property and value match
                if let Some(ref property) = self.property {
                    if let Some((prop, value)) = device_event.property() {
                        if prop != property {
                            return Ok(false);
                        }

                        if let Some(ref expected_value) = self.property_value {
                            if value != expected_value {
                                return Ok(false);
                            }
                        }
                    } else {
                        return Ok(false);
                    }
                }

                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

/// A timer trigger that starts a workflow at specific times
#[derive(Debug, Clone)]
pub struct TimerTrigger {
    /// Trigger ID
    id: String,
    /// Trigger description
    description: Option<String>,
    /// Cron expression for scheduling
    cron: Option<String>,
    /// Interval in seconds
    interval: Option<u64>,
    /// Specific times
    times: Vec<String>,
    /// Timer ID to match
    timer_id: Option<String>,
}

impl TimerTrigger {
    /// Create a new timer trigger
    pub fn new<S: Into<String>>(id: S) -> Self {
        Self {
            id: id.into(),
            description: None,
            cron: None,
            interval: None,
            times: Vec::new(),
            timer_id: None,
        }
    }

    /// Set the trigger description
    pub fn with_description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set a cron expression for scheduling
    pub fn with_cron<S: Into<String>>(mut self, cron: S) -> Self {
        self.cron = Some(cron.into());
        self
    }

    /// Set an interval in seconds
    pub fn with_interval(mut self, interval: u64) -> Self {
        self.interval = Some(interval);
        self
    }

    /// Add a specific time
    pub fn add_time<S: Into<String>>(mut self, time: S) -> Self {
        self.times.push(time.into());
        self
    }

    /// Set the timer ID to match
    pub fn with_timer_id<S: Into<String>>(mut self, timer_id: S) -> Self {
        self.timer_id = Some(timer_id.into());
        self
    }
}

#[async_trait]
impl Trigger for TimerTrigger {
    fn trigger_type(&self) -> &'static str {
        "timer"
    }

    async fn matches_event(&self, event: &TriggerEvent) -> Result<bool> {
        match event {
            TriggerEvent::Timer(timer_event) => {
                // Check if timer ID matches
                if let Some(ref timer_id) = self.timer_id {
                    if &timer_event.timer_id != timer_id {
                        return Ok(false);
                    }
                }

                // For cron, interval, and specific times, the scheduler should
                // only send events that match, so we don't need to check here

                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

/// A webhook trigger that starts a workflow when an HTTP request is received
#[derive(Debug, Clone)]
pub struct WebhookTrigger {
    /// Trigger ID
    id: String,
    /// Trigger description
    description: Option<String>,
    /// Webhook path
    path: String,
    /// HTTP method
    method: Option<String>,
    /// Required headers
    headers: HashMap<String, String>,
    /// Required query parameters
    query_params: HashMap<String, String>,
}

impl WebhookTrigger {
    /// Create a new webhook trigger
    pub fn new<S1: Into<String>, S2: Into<String>>(id: S1, path: S2) -> Self {
        Self {
            id: id.into(),
            description: None,
            path: path.into(),
            method: None,
            headers: HashMap::new(),
            query_params: HashMap::new(),
        }
    }

    /// Set the trigger description
    pub fn with_description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the HTTP method
    pub fn with_method<S: Into<String>>(mut self, method: S) -> Self {
        self.method = Some(method.into());
        self
    }

    /// Add a required header
    pub fn add_header<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Add a required query parameter
    pub fn add_query_param<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.query_params.insert(key.into(), value.into());
        self
    }
}

#[async_trait]
impl Trigger for WebhookTrigger {
    fn trigger_type(&self) -> &'static str {
        "webhook"
    }

    async fn matches_event(&self, event: &TriggerEvent) -> Result<bool> {
        match event {
            TriggerEvent::Webhook(webhook_event) => {
                // Check if path matches
                if webhook_event.path != self.path {
                    return Ok(false);
                }

                // Check if method matches
                if let Some(ref method) = self.method {
                    if &webhook_event.method != method {
                        return Ok(false);
                    }
                }

                // Check if required headers match
                for (key, value) in &self.headers {
                    match webhook_event.headers.get(key) {
                        Some(header_value) if header_value == value => {}
                        _ => return Ok(false),
                    }
                }

                // Check if required query parameters match
                for (key, value) in &self.query_params {
                    match webhook_event.query_params.get(key) {
                        Some(param_value) if param_value == value => {}
                        _ => return Ok(false),
                    }
                }

                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

/// A manual trigger that starts a workflow when manually triggered
#[derive(Debug, Clone)]
pub struct ManualTrigger {
    /// Trigger ID
    id: String,
    /// Trigger description
    description: Option<String>,
    /// Required data fields
    required_data: Vec<String>,
}

impl ManualTrigger {
    /// Create a new manual trigger
    pub fn new<S: Into<String>>(id: S) -> Self {
        Self {
            id: id.into(),
            description: None,
            required_data: Vec::new(),
        }
    }

    /// Set the trigger description
    pub fn with_description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add a required data field
    pub fn add_required_data<S: Into<String>>(mut self, field: S) -> Self {
        self.required_data.push(field.into());
        self
    }
}

#[async_trait]
impl Trigger for ManualTrigger {
    fn trigger_type(&self) -> &'static str {
        "manual"
    }

    async fn matches_event(&self, event: &TriggerEvent) -> Result<bool> {
        match event {
            TriggerEvent::Manual(manual_event) => {
                // Check if required data fields are present
                for field in &self.required_data {
                    if !manual_event.data.contains_key(field) {
                        return Ok(false);
                    }
                }

                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

/// A custom trigger with custom matching logic
#[derive(Debug, Clone)]
pub struct CustomTrigger<F>
where
    F: Fn(&TriggerEvent) -> Result<bool> + Send + Sync + 'static,
{
    /// Trigger ID
    id: String,
    /// Trigger description
    description: Option<String>,
    /// Custom match function
    matcher: F,
}

impl<F> CustomTrigger<F>
where
    F: Fn(&TriggerEvent) -> Result<bool> + Send + Sync + 'static,
{
    /// Create a new custom trigger
    pub fn new<S: Into<String>>(id: S, matcher: F) -> Self {
        Self {
            id: id.into(),
            description: None,
            matcher,
        }
    }

    /// Set the trigger description
    pub fn with_description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }
}

#[async_trait]
impl<F> Trigger for CustomTrigger<F>
where
    F: Fn(&TriggerEvent) -> Result<bool> + Send + Sync + 'static,
{
    fn trigger_type(&self) -> &'static str {
        "custom"
    }

    async fn matches_event(&self, event: &TriggerEvent) -> Result<bool> {
        (self.matcher)(event)
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}
