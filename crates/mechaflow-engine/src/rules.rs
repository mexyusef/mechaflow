/*!
 * Simple rules engine for MechaFlow.
 *
 * This module provides a lightweight rules engine for defining and
 * executing automation rules within the MechaFlow system.
 */
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use mechaflow_core::types::Value;

use crate::action::{Action, ActionContext, ActionResult};
use crate::condition::Condition;
use crate::context::WorkflowContext;
use crate::error::{Error, Result};

/// Rule identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RuleId(String);

impl RuleId {
    /// Create a new rule ID
    pub fn new<S: Into<String>>(id: S) -> Self {
        Self(id.into())
    }

    /// Get the inner string
    pub fn inner(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for RuleId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for RuleId {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

/// A rule that defines an automation rule
#[derive(Debug, Clone)]
pub struct Rule {
    /// Rule ID
    id: RuleId,
    /// Rule name
    name: String,
    /// Rule description
    description: Option<String>,
    /// Rule condition
    condition: Option<Arc<dyn Condition>>,
    /// Rule actions
    actions: Vec<Arc<dyn Action>>,
    /// Whether the rule is enabled
    enabled: bool,
    /// Rule metadata
    metadata: HashMap<String, Value>,
}

impl Rule {
    /// Create a new rule
    pub fn new<S1: Into<RuleId>, S2: Into<String>>(id: S1, name: S2) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            condition: None,
            actions: Vec::new(),
            enabled: true,
            metadata: HashMap::new(),
        }
    }

    /// Get the rule ID
    pub fn id(&self) -> &RuleId {
        &self.id
    }

    /// Get the rule name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the rule description
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Set the rule description
    pub fn set_description<S: Into<String>>(&mut self, description: S) {
        self.description = Some(description.into());
    }

    /// Set the rule condition
    pub fn set_condition<C: Condition + 'static>(&mut self, condition: C) {
        self.condition = Some(Arc::new(condition));
    }

    /// Get the rule condition
    pub fn condition(&self) -> Option<&Arc<dyn Condition>> {
        self.condition.as_ref()
    }

    /// Add a rule action
    pub fn add_action<A: Action + 'static>(&mut self, action: A) {
        self.actions.push(Arc::new(action));
    }

    /// Get all rule actions
    pub fn actions(&self) -> &[Arc<dyn Action>] {
        &self.actions
    }

    /// Check if the rule is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Enable the rule
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable the rule
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Set rule metadata
    pub fn set_metadata<K: Into<String>, V: Into<Value>>(&mut self, key: K, value: V) {
        self.metadata.insert(key.into(), value.into());
    }

    /// Get rule metadata
    pub fn metadata(&self) -> &HashMap<String, Value> {
        &self.metadata
    }

    /// Get a metadata value
    pub fn get_metadata(&self, key: &str) -> Option<&Value> {
        self.metadata.get(key)
    }

    /// Check if the rule's condition is met
    pub async fn check_condition(&self, context: &dyn WorkflowContext) -> Result<bool> {
        if !self.enabled {
            return Ok(false);
        }

        match &self.condition {
            Some(condition) => condition.evaluate(context).await,
            None => Ok(true), // No condition means the rule always applies
        }
    }

    /// Execute the rule's actions
    pub async fn execute_actions(&self, context: &dyn ActionContext) -> Result<Vec<ActionResult>> {
        if !self.enabled {
            return Err(Error::rule(format!("Rule {} is disabled", self.name)));
        }

        let mut results = Vec::new();

        for action in &self.actions {
            match action.execute(context).await {
                Ok(result) => {
                    results.push(result);
                }
                Err(e) => {
                    error!("Failed to execute action in rule {}: {}", self.name, e);
                    return Err(e);
                }
            }
        }

        Ok(results)
    }
}

/// Result of evaluating a rule
#[derive(Debug, Clone)]
pub struct RuleEvaluationResult {
    /// Rule ID
    pub rule_id: RuleId,
    /// Rule name
    pub rule_name: String,
    /// Whether the condition was met
    pub condition_met: bool,
    /// Action results, if the condition was met
    pub action_results: Vec<ActionResult>,
    /// Error, if any
    pub error: Option<String>,
}

/// Rule engine events
#[derive(Debug, Clone)]
pub enum RuleEngineEvent {
    /// Rule was triggered
    RuleTriggered {
        /// Rule ID
        rule_id: RuleId,
        /// Rule name
        rule_name: String,
    },
    /// Rule was executed
    RuleExecuted {
        /// Rule ID
        rule_id: RuleId,
        /// Rule name
        rule_name: String,
        /// Action results
        results: Vec<ActionResult>,
    },
    /// Rule execution failed
    RuleFailed {
        /// Rule ID
        rule_id: RuleId,
        /// Rule name
        rule_name: String,
        /// Error message
        error: String,
    },
    /// Rule was added
    RuleAdded {
        /// Rule ID
        rule_id: RuleId,
        /// Rule name
        rule_name: String,
    },
    /// Rule was removed
    RuleRemoved {
        /// Rule ID
        rule_id: RuleId,
        /// Rule name
        rule_name: String,
    },
    /// Rule was enabled
    RuleEnabled {
        /// Rule ID
        rule_id: RuleId,
        /// Rule name
        rule_name: String,
    },
    /// Rule was disabled
    RuleDisabled {
        /// Rule ID
        rule_id: RuleId,
        /// Rule name
        rule_name: String,
    },
}

/// Rules engine for executing rules
#[derive(Debug)]
pub struct RuleEngine {
    /// Rules
    rules: RwLock<HashMap<RuleId, Rule>>,
    /// Event sender
    event_tx: tokio::sync::broadcast::Sender<RuleEngineEvent>,
}

impl RuleEngine {
    /// Create a new rule engine
    pub fn new() -> Self {
        let (event_tx, _) = tokio::sync::broadcast::channel(100);

        Self {
            rules: RwLock::new(HashMap::new()),
            event_tx,
        }
    }

    /// Subscribe to rule engine events
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<RuleEngineEvent> {
        self.event_tx.subscribe()
    }

    /// Add a rule
    pub async fn add_rule(&self, rule: Rule) -> Result<()> {
        let mut rules = self.rules.write().await;

        if rules.contains_key(rule.id()) {
            return Err(Error::already_exists(format!(
                "Rule with ID {} already exists",
                rule.id()
            )));
        }

        let rule_id = rule.id().clone();
        let rule_name = rule.name().to_string();

        rules.insert(rule_id.clone(), rule);

        // Emit rule added event
        if let Err(e) = self.event_tx.send(RuleEngineEvent::RuleAdded {
            rule_id,
            rule_name,
        }) {
            warn!("Failed to send rule added event: {}", e);
        }

        Ok(())
    }

    /// Remove a rule
    pub async fn remove_rule(&self, rule_id: &RuleId) -> Result<()> {
        let mut rules = self.rules.write().await;

        if let Some(rule) = rules.remove(rule_id) {
            // Emit rule removed event
            if let Err(e) = self.event_tx.send(RuleEngineEvent::RuleRemoved {
                rule_id: rule_id.clone(),
                rule_name: rule.name().to_string(),
            }) {
                warn!("Failed to send rule removed event: {}", e);
            }

            Ok(())
        } else {
            Err(Error::not_found(format!("Rule with ID {} not found", rule_id)))
        }
    }

    /// Get a rule by ID
    pub async fn get_rule(&self, rule_id: &RuleId) -> Result<Rule> {
        let rules = self.rules.read().await;

        if let Some(rule) = rules.get(rule_id) {
            Ok(rule.clone())
        } else {
            Err(Error::not_found(format!("Rule with ID {} not found", rule_id)))
        }
    }

    /// Get all rules
    pub async fn get_rules(&self) -> Vec<Rule> {
        let rules = self.rules.read().await;
        rules.values().cloned().collect()
    }

    /// Enable a rule
    pub async fn enable_rule(&self, rule_id: &RuleId) -> Result<()> {
        let mut rules = self.rules.write().await;

        if let Some(rule) = rules.get_mut(rule_id) {
            rule.enable();

            // Emit rule enabled event
            if let Err(e) = self.event_tx.send(RuleEngineEvent::RuleEnabled {
                rule_id: rule_id.clone(),
                rule_name: rule.name().to_string(),
            }) {
                warn!("Failed to send rule enabled event: {}", e);
            }

            Ok(())
        } else {
            Err(Error::not_found(format!("Rule with ID {} not found", rule_id)))
        }
    }

    /// Disable a rule
    pub async fn disable_rule(&self, rule_id: &RuleId) -> Result<()> {
        let mut rules = self.rules.write().await;

        if let Some(rule) = rules.get_mut(rule_id) {
            rule.disable();

            // Emit rule disabled event
            if let Err(e) = self.event_tx.send(RuleEngineEvent::RuleDisabled {
                rule_id: rule_id.clone(),
                rule_name: rule.name().to_string(),
            }) {
                warn!("Failed to send rule disabled event: {}", e);
            }

            Ok(())
        } else {
            Err(Error::not_found(format!("Rule with ID {} not found", rule_id)))
        }
    }

    /// Evaluate all rules
    pub async fn evaluate_all(
        &self,
        workflow_context: &dyn WorkflowContext,
        action_context: &dyn ActionContext,
    ) -> Vec<RuleEvaluationResult> {
        let rules = self.rules.read().await;
        let mut results = Vec::new();

        for rule in rules.values() {
            match self.evaluate_rule(rule, workflow_context, action_context).await {
                Ok(result) => {
                    results.push(result);
                }
                Err(e) => {
                    error!("Failed to evaluate rule {}: {}", rule.name(), e);
                    results.push(RuleEvaluationResult {
                        rule_id: rule.id().clone(),
                        rule_name: rule.name().to_string(),
                        condition_met: false,
                        action_results: Vec::new(),
                        error: Some(e.to_string()),
                    });
                }
            }
        }

        results
    }

    /// Evaluate a single rule
    pub async fn evaluate_rule(
        &self,
        rule: &Rule,
        workflow_context: &dyn WorkflowContext,
        action_context: &dyn ActionContext,
    ) -> Result<RuleEvaluationResult> {
        if !rule.is_enabled() {
            return Ok(RuleEvaluationResult {
                rule_id: rule.id().clone(),
                rule_name: rule.name().to_string(),
                condition_met: false,
                action_results: Vec::new(),
                error: None,
            });
        }

        // Check if condition is met
        let condition_met = match rule.check_condition(workflow_context).await {
            Ok(met) => met,
            Err(e) => {
                return Ok(RuleEvaluationResult {
                    rule_id: rule.id().clone(),
                    rule_name: rule.name().to_string(),
                    condition_met: false,
                    action_results: Vec::new(),
                    error: Some(format!("Failed to evaluate condition: {}", e)),
                });
            }
        };

        if !condition_met {
            return Ok(RuleEvaluationResult {
                rule_id: rule.id().clone(),
                rule_name: rule.name().to_string(),
                condition_met: false,
                action_results: Vec::new(),
                error: None,
            });
        }

        // Emit rule triggered event
        if let Err(e) = self.event_tx.send(RuleEngineEvent::RuleTriggered {
            rule_id: rule.id().clone(),
            rule_name: rule.name().to_string(),
        }) {
            warn!("Failed to send rule triggered event: {}", e);
        }

        // Execute actions
        match rule.execute_actions(action_context).await {
            Ok(results) => {
                // Emit rule executed event
                if let Err(e) = self.event_tx.send(RuleEngineEvent::RuleExecuted {
                    rule_id: rule.id().clone(),
                    rule_name: rule.name().to_string(),
                    results: results.clone(),
                }) {
                    warn!("Failed to send rule executed event: {}", e);
                }

                Ok(RuleEvaluationResult {
                    rule_id: rule.id().clone(),
                    rule_name: rule.name().to_string(),
                    condition_met: true,
                    action_results: results,
                    error: None,
                })
            }
            Err(e) => {
                // Emit rule failed event
                if let Err(e2) = self.event_tx.send(RuleEngineEvent::RuleFailed {
                    rule_id: rule.id().clone(),
                    rule_name: rule.name().to_string(),
                    error: e.to_string(),
                }) {
                    warn!("Failed to send rule failed event: {}", e2);
                }

                Ok(RuleEvaluationResult {
                    rule_id: rule.id().clone(),
                    rule_name: rule.name().to_string(),
                    condition_met: true,
                    action_results: Vec::new(),
                    error: Some(e.to_string()),
                })
            }
        }
    }
}

impl Default for RuleEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for rules
#[derive(Debug, Default)]
pub struct RuleBuilder {
    /// Rule ID
    id: Option<RuleId>,
    /// Rule name
    name: Option<String>,
    /// Rule description
    description: Option<String>,
    /// Rule condition
    condition: Option<Arc<dyn Condition>>,
    /// Rule actions
    actions: Vec<Arc<dyn Action>>,
    /// Whether the rule is enabled
    enabled: bool,
    /// Rule metadata
    metadata: HashMap<String, Value>,
}

impl RuleBuilder {
    /// Create a new rule builder
    pub fn new() -> Self {
        Self {
            enabled: true,
            ..Default::default()
        }
    }

    /// Set the rule ID
    pub fn with_id<S: Into<RuleId>>(mut self, id: S) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the rule name
    pub fn with_name<S: Into<String>>(mut self, name: S) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the rule description
    pub fn with_description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the rule condition
    pub fn with_condition<C: Condition + 'static>(mut self, condition: C) -> Self {
        self.condition = Some(Arc::new(condition));
        self
    }

    /// Add a rule action
    pub fn with_action<A: Action + 'static>(mut self, action: A) -> Self {
        self.actions.push(Arc::new(action));
        self
    }

    /// Set whether the rule is enabled
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set rule metadata
    pub fn with_metadata<K: Into<String>, V: Into<Value>>(mut self, key: K, value: V) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Build the rule
    pub fn build(self) -> Result<Rule> {
        let id = self.id.ok_or_else(|| Error::validation("Rule ID is required"))?;
        let name = self.name.ok_or_else(|| Error::validation("Rule name is required"))?;

        let mut rule = Rule::new(id, name);

        if let Some(description) = self.description {
            rule.set_description(description);
        }

        if let Some(condition) = self.condition {
            rule.condition = Some(condition);
        }

        rule.actions = self.actions;
        rule.enabled = self.enabled;
        rule.metadata = self.metadata;

        Ok(rule)
    }
}
