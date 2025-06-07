/*!
 * Conditions for workflow execution.
 *
 * This module defines the conditions that can be used in workflows to
 * determine when a workflow should run or actions should be executed.
 */
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use chrono::{DateTime, Utc, Weekday};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use mechaflow_core::types::{Id, Value};

use crate::context::WorkflowContext;
use crate::error::Result;

/// A condition that can be evaluated
#[async_trait]
pub trait Condition: fmt::Debug + Send + Sync {
    /// Get the condition type
    fn condition_type(&self) -> &'static str;

    /// Evaluate the condition
    async fn evaluate(&self, context: &dyn WorkflowContext) -> Result<bool>;

    /// Get the condition ID
    fn id(&self) -> &str;

    /// Get the condition description
    fn description(&self) -> Option<&str>;
}

/// A device condition that checks device properties
#[derive(Debug, Clone)]
pub struct DeviceCondition {
    /// Condition ID
    id: String,
    /// Condition description
    description: Option<String>,
    /// Device ID
    device_id: Id,
    /// Property to check
    property: String,
    /// Operator for comparison
    operator: ComparisonOperator,
    /// Value to compare against
    value: Value,
}

/// Comparison operators for conditions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComparisonOperator {
    /// Equal to
    Eq,
    /// Not equal to
    Ne,
    /// Greater than
    Gt,
    /// Greater than or equal to
    Ge,
    /// Less than
    Lt,
    /// Less than or equal to
    Le,
    /// Contains (for strings, arrays)
    Contains,
    /// Starts with (for strings)
    StartsWith,
    /// Ends with (for strings)
    EndsWith,
    /// Matches regex (for strings)
    Matches,
}

impl DeviceCondition {
    /// Create a new device condition
    pub fn new<S: Into<String>>(id: S, device_id: Id, property: S, operator: ComparisonOperator, value: Value) -> Self {
        Self {
            id: id.into(),
            description: None,
            device_id,
            property: property.into(),
            operator,
            value,
        }
    }

    /// Set the condition description
    pub fn with_description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }
}

#[async_trait]
impl Condition for DeviceCondition {
    fn condition_type(&self) -> &'static str {
        "device"
    }

    async fn evaluate(&self, context: &dyn WorkflowContext) -> Result<bool> {
        let current_value = context.read_property(&self.device_id, &self.property)?;

        match self.operator {
            ComparisonOperator::Eq => Ok(current_value == self.value),
            ComparisonOperator::Ne => Ok(current_value != self.value),
            ComparisonOperator::Gt => compare_values(&current_value, &self.value, |a, b| a > b),
            ComparisonOperator::Ge => compare_values(&current_value, &self.value, |a, b| a >= b),
            ComparisonOperator::Lt => compare_values(&current_value, &self.value, |a, b| a < b),
            ComparisonOperator::Le => compare_values(&current_value, &self.value, |a, b| a <= b),
            ComparisonOperator::Contains => {
                match &current_value {
                    Value::String(s) => {
                        if let Value::String(v) = &self.value {
                            Ok(s.contains(v))
                        } else {
                            Ok(false)
                        }
                    }
                    Value::Array(arr) => {
                        Ok(arr.contains(&self.value))
                    }
                    _ => {
                        warn!("Contains operator not supported for value type: {:?}", current_value);
                        Ok(false)
                    }
                }
            }
            ComparisonOperator::StartsWith => {
                match (&current_value, &self.value) {
                    (Value::String(s), Value::String(v)) => Ok(s.starts_with(v)),
                    _ => {
                        warn!("StartsWith operator requires string values, got: {:?} and {:?}", current_value, self.value);
                        Ok(false)
                    }
                }
            }
            ComparisonOperator::EndsWith => {
                match (&current_value, &self.value) {
                    (Value::String(s), Value::String(v)) => Ok(s.ends_with(v)),
                    _ => {
                        warn!("EndsWith operator requires string values, got: {:?} and {:?}", current_value, self.value);
                        Ok(false)
                    }
                }
            }
            ComparisonOperator::Matches => {
                match (&current_value, &self.value) {
                    (Value::String(s), Value::String(pattern)) => {
                        match regex::Regex::new(pattern) {
                            Ok(re) => Ok(re.is_match(s)),
                            Err(e) => {
                                warn!("Invalid regex pattern '{}': {}", pattern, e);
                                Ok(false)
                            }
                        }
                    }
                    _ => {
                        warn!("Matches operator requires string values, got: {:?} and {:?}", current_value, self.value);
                        Ok(false)
                    }
                }
            }
        }
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

/// Helper function to compare values using a comparison function
fn compare_values<F>(a: &Value, b: &Value, cmp: F) -> Result<bool>
where
    F: Fn(f64, f64) -> bool,
{
    match (a, b) {
        (Value::Number(a_num), Value::Number(b_num)) => {
            let a_f64 = a_num.as_f64().unwrap_or_default();
            let b_f64 = b_num.as_f64().unwrap_or_default();
            Ok(cmp(a_f64, b_f64))
        }
        _ => {
            warn!("Numeric comparison not possible for values: {:?} and {:?}", a, b);
            Ok(false)
        }
    }
}

/// A time condition that checks if the current time meets certain criteria
#[derive(Debug, Clone)]
pub struct TimeCondition {
    /// Condition ID
    id: String,
    /// Condition description
    description: Option<String>,
    /// Check if time is after this time
    after: Option<String>,
    /// Check if time is before this time
    before: Option<String>,
    /// Check if time is on these days of the week
    days_of_week: Option<Vec<Weekday>>,
}

impl TimeCondition {
    /// Create a new time condition
    pub fn new<S: Into<String>>(id: S) -> Self {
        Self {
            id: id.into(),
            description: None,
            after: None,
            before: None,
            days_of_week: None,
        }
    }

    /// Set the condition description
    pub fn with_description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the after time (format: "HH:MM")
    pub fn after<S: Into<String>>(mut self, time: S) -> Self {
        self.after = Some(time.into());
        self
    }

    /// Set the before time (format: "HH:MM")
    pub fn before<S: Into<String>>(mut self, time: S) -> Self {
        self.before = Some(time.into());
        self
    }

    /// Set the days of week
    pub fn on_days(mut self, days: Vec<Weekday>) -> Self {
        self.days_of_week = Some(days);
        self
    }
}

#[async_trait]
impl Condition for TimeCondition {
    fn condition_type(&self) -> &'static str {
        "time"
    }

    async fn evaluate(&self, _context: &dyn WorkflowContext) -> Result<bool> {
        let now = Utc::now();
        let now_time = format!("{:02}:{:02}", now.hour(), now.minute());

        // Check if time is after the specified time
        if let Some(ref after) = self.after {
            if now_time < *after {
                return Ok(false);
            }
        }

        // Check if time is before the specified time
        if let Some(ref before) = self.before {
            if now_time > *before {
                return Ok(false);
            }
        }

        // Check if today is one of the specified days
        if let Some(ref days) = self.days_of_week {
            if !days.contains(&now.weekday()) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

/// A variable condition that checks workflow variables
#[derive(Debug, Clone)]
pub struct VariableCondition {
    /// Condition ID
    id: String,
    /// Condition description
    description: Option<String>,
    /// Variable name
    variable: String,
    /// Operator for comparison
    operator: ComparisonOperator,
    /// Value to compare against
    value: Value,
}

impl VariableCondition {
    /// Create a new variable condition
    pub fn new<S1: Into<String>, S2: Into<String>>(
        id: S1,
        variable: S2,
        operator: ComparisonOperator,
        value: Value,
    ) -> Self {
        Self {
            id: id.into(),
            description: None,
            variable: variable.into(),
            operator,
            value,
        }
    }

    /// Set the condition description
    pub fn with_description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }
}

#[async_trait]
impl Condition for VariableCondition {
    fn condition_type(&self) -> &'static str {
        "variable"
    }

    async fn evaluate(&self, context: &dyn WorkflowContext) -> Result<bool> {
        let current_value = context.get_variable(&self.variable)?;

        match self.operator {
            ComparisonOperator::Eq => Ok(current_value == self.value),
            ComparisonOperator::Ne => Ok(current_value != self.value),
            ComparisonOperator::Gt => compare_values(&current_value, &self.value, |a, b| a > b),
            ComparisonOperator::Ge => compare_values(&current_value, &self.value, |a, b| a >= b),
            ComparisonOperator::Lt => compare_values(&current_value, &self.value, |a, b| a < b),
            ComparisonOperator::Le => compare_values(&current_value, &self.value, |a, b| a <= b),
            ComparisonOperator::Contains => {
                match &current_value {
                    Value::String(s) => {
                        if let Value::String(v) = &self.value {
                            Ok(s.contains(v))
                        } else {
                            Ok(false)
                        }
                    }
                    Value::Array(arr) => {
                        Ok(arr.contains(&self.value))
                    }
                    _ => {
                        warn!("Contains operator not supported for value type: {:?}", current_value);
                        Ok(false)
                    }
                }
            }
            ComparisonOperator::StartsWith => {
                match (&current_value, &self.value) {
                    (Value::String(s), Value::String(v)) => Ok(s.starts_with(v)),
                    _ => {
                        warn!("StartsWith operator requires string values, got: {:?} and {:?}", current_value, self.value);
                        Ok(false)
                    }
                }
            }
            ComparisonOperator::EndsWith => {
                match (&current_value, &self.value) {
                    (Value::String(s), Value::String(v)) => Ok(s.ends_with(v)),
                    _ => {
                        warn!("EndsWith operator requires string values, got: {:?} and {:?}", current_value, self.value);
                        Ok(false)
                    }
                }
            }
            ComparisonOperator::Matches => {
                match (&current_value, &self.value) {
                    (Value::String(s), Value::String(pattern)) => {
                        match regex::Regex::new(pattern) {
                            Ok(re) => Ok(re.is_match(s)),
                            Err(e) => {
                                warn!("Invalid regex pattern '{}': {}", pattern, e);
                                Ok(false)
                            }
                        }
                    }
                    _ => {
                        warn!("Matches operator requires string values, got: {:?} and {:?}", current_value, self.value);
                        Ok(false)
                    }
                }
            }
        }
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

/// A composite condition that combines multiple conditions
#[derive(Debug, Clone)]
pub struct AndCondition {
    /// Condition ID
    id: String,
    /// Condition description
    description: Option<String>,
    /// Conditions to combine
    conditions: Vec<Arc<dyn Condition>>,
}

impl AndCondition {
    /// Create a new AND condition
    pub fn new<S: Into<String>>(id: S) -> Self {
        Self {
            id: id.into(),
            description: None,
            conditions: Vec::new(),
        }
    }

    /// Set the condition description
    pub fn with_description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add a condition
    pub fn add_condition<C: Condition + 'static>(mut self, condition: C) -> Self {
        self.conditions.push(Arc::new(condition));
        self
    }
}

#[async_trait]
impl Condition for AndCondition {
    fn condition_type(&self) -> &'static str {
        "and"
    }

    async fn evaluate(&self, context: &dyn WorkflowContext) -> Result<bool> {
        for condition in &self.conditions {
            if !condition.evaluate(context).await? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

/// A composite condition that requires any of its conditions to be true
#[derive(Debug, Clone)]
pub struct OrCondition {
    /// Condition ID
    id: String,
    /// Condition description
    description: Option<String>,
    /// Conditions to combine
    conditions: Vec<Arc<dyn Condition>>,
}

impl OrCondition {
    /// Create a new OR condition
    pub fn new<S: Into<String>>(id: S) -> Self {
        Self {
            id: id.into(),
            description: None,
            conditions: Vec::new(),
        }
    }

    /// Set the condition description
    pub fn with_description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add a condition
    pub fn add_condition<C: Condition + 'static>(mut self, condition: C) -> Self {
        self.conditions.push(Arc::new(condition));
        self
    }
}

#[async_trait]
impl Condition for OrCondition {
    fn condition_type(&self) -> &'static str {
        "or"
    }

    async fn evaluate(&self, context: &dyn WorkflowContext) -> Result<bool> {
        if self.conditions.is_empty() {
            return Ok(true);
        }

        for condition in &self.conditions {
            if condition.evaluate(context).await? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

/// A condition that negates another condition
#[derive(Debug, Clone)]
pub struct NotCondition {
    /// Condition ID
    id: String,
    /// Condition description
    description: Option<String>,
    /// Condition to negate
    condition: Arc<dyn Condition>,
}

impl NotCondition {
    /// Create a new NOT condition
    pub fn new<S: Into<String>, C: Condition + 'static>(id: S, condition: C) -> Self {
        Self {
            id: id.into(),
            description: None,
            condition: Arc::new(condition),
        }
    }

    /// Set the condition description
    pub fn with_description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }
}

#[async_trait]
impl Condition for NotCondition {
    fn condition_type(&self) -> &'static str {
        "not"
    }

    async fn evaluate(&self, context: &dyn WorkflowContext) -> Result<bool> {
        Ok(!self.condition.evaluate(context).await?)
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

/// A custom condition with custom evaluation logic
#[derive(Debug, Clone)]
pub struct CustomCondition<F>
where
    F: Fn(&dyn WorkflowContext) -> Result<bool> + Send + Sync + 'static,
{
    /// Condition ID
    id: String,
    /// Condition description
    description: Option<String>,
    /// Custom evaluation function
    evaluate_fn: F,
}

impl<F> CustomCondition<F>
where
    F: Fn(&dyn WorkflowContext) -> Result<bool> + Send + Sync + 'static,
{
    /// Create a new custom condition
    pub fn new<S: Into<String>>(id: S, evaluate_fn: F) -> Self {
        Self {
            id: id.into(),
            description: None,
            evaluate_fn,
        }
    }

    /// Set the condition description
    pub fn with_description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }
}

#[async_trait]
impl<F> Condition for CustomCondition<F>
where
    F: Fn(&dyn WorkflowContext) -> Result<bool> + Send + Sync + 'static,
{
    fn condition_type(&self) -> &'static str {
        "custom"
    }

    async fn evaluate(&self, context: &dyn WorkflowContext) -> Result<bool> {
        (self.evaluate_fn)(context)
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}
