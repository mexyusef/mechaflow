/*!
 * State machines for workflow state transitions.
 *
 * This module provides tools for defining and executing state machines that
 * drive complex workflow state transitions.
 */
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use mechaflow_core::types::Value;

use crate::action::{Action, ActionContext, ActionResult};
use crate::context::WorkflowContext;
use crate::error::{Error, Result};

/// State machine state identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StateId(String);

impl StateId {
    /// Create a new state ID
    pub fn new<S: Into<String>>(id: S) -> Self {
        Self(id.into())
    }

    /// Get the inner string
    pub fn inner(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for StateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for StateId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for StateId {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

/// State machine event
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(String);

impl EventId {
    /// Create a new event ID
    pub fn new<S: Into<String>>(id: S) -> Self {
        Self(id.into())
    }

    /// Get the inner string
    pub fn inner(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for EventId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for EventId {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

/// State machine state
#[derive(Debug, Clone)]
pub struct State {
    /// State ID
    id: StateId,
    /// State name
    name: String,
    /// State description
    description: Option<String>,
    /// Entry actions
    entry_actions: Vec<Arc<dyn Action>>,
    /// Exit actions
    exit_actions: Vec<Arc<dyn Action>>,
    /// State data
    data: HashMap<String, Value>,
    /// Whether this is a final state
    is_final: bool,
}

impl State {
    /// Create a new state
    pub fn new<S1: Into<StateId>, S2: Into<String>>(id: S1, name: S2) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            entry_actions: Vec::new(),
            exit_actions: Vec::new(),
            data: HashMap::new(),
            is_final: false,
        }
    }

    /// Get the state ID
    pub fn id(&self) -> &StateId {
        &self.id
    }

    /// Get the state name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the state description
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Set the state description
    pub fn set_description<S: Into<String>>(&mut self, description: S) {
        self.description = Some(description.into());
    }

    /// Add an entry action
    pub fn add_entry_action<A: Action + 'static>(&mut self, action: A) {
        self.entry_actions.push(Arc::new(action));
    }

    /// Get all entry actions
    pub fn entry_actions(&self) -> &[Arc<dyn Action>] {
        &self.entry_actions
    }

    /// Add an exit action
    pub fn add_exit_action<A: Action + 'static>(&mut self, action: A) {
        self.exit_actions.push(Arc::new(action));
    }

    /// Get all exit actions
    pub fn exit_actions(&self) -> &[Arc<dyn Action>] {
        &self.exit_actions
    }

    /// Set state data
    pub fn set_data<K: Into<String>, V: Into<Value>>(&mut self, key: K, value: V) {
        self.data.insert(key.into(), value.into());
    }

    /// Get state data
    pub fn data(&self) -> &HashMap<String, Value> {
        &self.data
    }

    /// Get a state data value
    pub fn get_data(&self, key: &str) -> Option<&Value> {
        self.data.get(key)
    }

    /// Set whether this is a final state
    pub fn set_final(&mut self, is_final: bool) {
        self.is_final = is_final;
    }

    /// Check if this is a final state
    pub fn is_final(&self) -> bool {
        self.is_final
    }
}

/// State transition
#[derive(Debug, Clone)]
pub struct Transition {
    /// Source state ID
    source: StateId,
    /// Target state ID
    target: StateId,
    /// Event ID that triggers this transition
    event: EventId,
    /// Guard condition
    guard: Option<Arc<dyn TransitionGuard>>,
    /// Transition actions
    actions: Vec<Arc<dyn Action>>,
}

impl Transition {
    /// Create a new transition
    pub fn new<S1: Into<StateId>, S2: Into<StateId>, E: Into<EventId>>(
        source: S1,
        target: S2,
        event: E,
    ) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            event: event.into(),
            guard: None,
            actions: Vec::new(),
        }
    }

    /// Get the source state ID
    pub fn source(&self) -> &StateId {
        &self.source
    }

    /// Get the target state ID
    pub fn target(&self) -> &StateId {
        &self.target
    }

    /// Get the event ID
    pub fn event(&self) -> &EventId {
        &self.event
    }

    /// Set a guard condition
    pub fn set_guard<G: TransitionGuard + 'static>(&mut self, guard: G) {
        self.guard = Some(Arc::new(guard));
    }

    /// Get the guard condition
    pub fn guard(&self) -> Option<&Arc<dyn TransitionGuard>> {
        self.guard.as_ref()
    }

    /// Add a transition action
    pub fn add_action<A: Action + 'static>(&mut self, action: A) {
        self.actions.push(Arc::new(action));
    }

    /// Get all transition actions
    pub fn actions(&self) -> &[Arc<dyn Action>] {
        &self.actions
    }
}

/// Guard condition for transitions
#[async_trait]
pub trait TransitionGuard: fmt::Debug + Send + Sync {
    /// Evaluate the guard condition
    async fn evaluate(&self, context: &dyn WorkflowContext) -> Result<bool>;
}

/// A simple guard condition based on a predicate function
#[derive(Clone)]
pub struct PredicateGuard<F>
where
    F: Fn(&dyn WorkflowContext) -> Result<bool> + Send + Sync + 'static,
{
    /// Predicate function
    predicate: F,
}

impl<F> PredicateGuard<F>
where
    F: Fn(&dyn WorkflowContext) -> Result<bool> + Send + Sync + 'static,
{
    /// Create a new predicate guard
    pub fn new(predicate: F) -> Self {
        Self { predicate }
    }
}

impl<F> fmt::Debug for PredicateGuard<F>
where
    F: Fn(&dyn WorkflowContext) -> Result<bool> + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PredicateGuard")
            .field("predicate", &"<function>")
            .finish()
    }
}

#[async_trait]
impl<F> TransitionGuard for PredicateGuard<F>
where
    F: Fn(&dyn WorkflowContext) -> Result<bool> + Send + Sync + 'static,
{
    async fn evaluate(&self, context: &dyn WorkflowContext) -> Result<bool> {
        (self.predicate)(context)
    }
}

/// State machine for workflow state transitions
#[derive(Debug)]
pub struct StateMachine {
    /// State machine ID
    id: String,
    /// State machine name
    name: String,
    /// State machine description
    description: Option<String>,
    /// States
    states: HashMap<StateId, State>,
    /// Transitions
    transitions: Vec<Transition>,
    /// Initial state ID
    initial_state: Option<StateId>,
    /// Current state ID
    current_state: RwLock<Option<StateId>>,
    /// History of state transitions
    history: RwLock<Vec<(StateId, EventId, StateId)>>,
}

impl StateMachine {
    /// Create a new state machine
    pub fn new<S1: Into<String>, S2: Into<String>>(id: S1, name: S2) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            states: HashMap::new(),
            transitions: Vec::new(),
            initial_state: None,
            current_state: RwLock::new(None),
            history: RwLock::new(Vec::new()),
        }
    }

    /// Get the state machine ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the state machine name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the state machine description
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Set the state machine description
    pub fn set_description<S: Into<String>>(&mut self, description: S) {
        self.description = Some(description.into());
    }

    /// Add a state
    pub fn add_state(&mut self, state: State) -> Result<()> {
        if self.states.contains_key(state.id()) {
            return Err(Error::already_exists(format!(
                "State with ID {} already exists",
                state.id()
            )));
        }

        self.states.insert(state.id().clone(), state);
        Ok(())
    }

    /// Get a state by ID
    pub fn get_state(&self, state_id: &StateId) -> Option<&State> {
        self.states.get(state_id)
    }

    /// Get all states
    pub fn get_states(&self) -> Vec<&State> {
        self.states.values().collect()
    }

    /// Add a transition
    pub fn add_transition(&mut self, transition: Transition) -> Result<()> {
        // Check that source and target states exist
        if !self.states.contains_key(transition.source()) {
            return Err(Error::not_found(format!(
                "Source state with ID {} not found",
                transition.source()
            )));
        }

        if !self.states.contains_key(transition.target()) {
            return Err(Error::not_found(format!(
                "Target state with ID {} not found",
                transition.target()
            )));
        }

        self.transitions.push(transition);
        Ok(())
    }

    /// Get all transitions
    pub fn get_transitions(&self) -> &[Transition] {
        &self.transitions
    }

    /// Set the initial state
    pub fn set_initial_state(&mut self, state_id: StateId) -> Result<()> {
        if !self.states.contains_key(&state_id) {
            return Err(Error::not_found(format!(
                "State with ID {} not found",
                state_id
            )));
        }

        self.initial_state = Some(state_id);
        Ok(())
    }

    /// Get the initial state ID
    pub fn initial_state(&self) -> Option<&StateId> {
        self.initial_state.as_ref()
    }

    /// Initialize the state machine
    pub async fn initialize(&self, context: &dyn ActionContext) -> Result<()> {
        let mut current_state = self.current_state.write().await;

        if current_state.is_some() {
            return Err(Error::state_machine(
                "State machine is already initialized".to_string(),
            ));
        }

        let initial_state_id = self.initial_state.as_ref().ok_or_else(|| {
            Error::state_machine("No initial state defined for state machine".to_string())
        })?;

        let initial_state = self.states.get(initial_state_id).ok_or_else(|| {
            Error::state_machine(format!(
                "Initial state with ID {} not found",
                initial_state_id
            ))
        })?;

        // Execute entry actions
        for action in &initial_state.entry_actions {
            if let Err(e) = action.execute(context).await {
                error!(
                    "Failed to execute entry action for state {}: {}",
                    initial_state.name(),
                    e
                );
            }
        }

        *current_state = Some(initial_state_id.clone());

        info!(
            "State machine {} initialized with state {}",
            self.name, initial_state.name()
        );

        Ok(())
    }

    /// Get the current state ID
    pub async fn current_state(&self) -> Option<StateId> {
        self.current_state.read().await.clone()
    }

    /// Get the current state
    pub async fn get_current_state(&self) -> Option<State> {
        let current_state_id = self.current_state.read().await.clone();
        current_state_id.and_then(|id| self.states.get(&id).cloned())
    }

    /// Trigger an event
    pub async fn trigger_event(
        &self,
        event_id: &EventId,
        context: &dyn ActionContext,
    ) -> Result<Option<StateId>> {
        let current_state_id = {
            let current = self.current_state.read().await;
            match &*current {
                Some(state_id) => state_id.clone(),
                None => {
                    return Err(Error::state_machine(
                        "State machine is not initialized".to_string(),
                    ));
                }
            }
        };

        // Find applicable transitions
        let mut applicable_transitions = Vec::new();
        for transition in &self.transitions {
            if transition.source() == &current_state_id && transition.event() == event_id {
                applicable_transitions.push(transition);
            }
        }

        if applicable_transitions.is_empty() {
            debug!(
                "No transition found for event {} from state {}",
                event_id, current_state_id
            );
            return Ok(None);
        }

        // Evaluate guards and find first matching transition
        let mut matching_transition = None;
        for transition in applicable_transitions {
            let guard_matched = match &transition.guard {
                Some(guard) => guard.evaluate(context).await?,
                None => true,
            };

            if guard_matched {
                matching_transition = Some(transition);
                break;
            }
        }

        let transition = match matching_transition {
            Some(t) => t,
            None => {
                debug!(
                    "No transition guard matched for event {} from state {}",
                    event_id, current_state_id
                );
                return Ok(None);
            }
        };

        // Get source and target states
        let source_state = self.states.get(&current_state_id).ok_or_else(|| {
            Error::state_machine(format!("Source state {} not found", current_state_id))
        })?;

        let target_state_id = transition.target().clone();
        let target_state = self.states.get(&target_state_id).ok_or_else(|| {
            Error::state_machine(format!("Target state {} not found", target_state_id))
        })?;

        // Execute exit actions for source state
        for action in &source_state.exit_actions {
            if let Err(e) = action.execute(context).await {
                error!(
                    "Failed to execute exit action for state {}: {}",
                    source_state.name(),
                    e
                );
            }
        }

        // Execute transition actions
        for action in transition.actions() {
            if let Err(e) = action.execute(context).await {
                error!("Failed to execute transition action: {}", e);
            }
        }

        // Execute entry actions for target state
        for action in &target_state.entry_actions {
            if let Err(e) = action.execute(context).await {
                error!(
                    "Failed to execute entry action for state {}: {}",
                    target_state.name(),
                    e
                );
            }
        }

        // Update current state and history
        {
            let mut current = self.current_state.write().await;
            *current = Some(target_state_id.clone());

            let mut history = self.history.write().await;
            history.push((
                current_state_id.clone(),
                event_id.clone(),
                target_state_id.clone(),
            ));
        }

        info!(
            "State machine {} transitioned from {} to {} on event {}",
            self.name,
            source_state.name(),
            target_state.name(),
            event_id
        );

        Ok(Some(target_state_id))
    }

    /// Get the transition history
    pub async fn get_history(&self) -> Vec<(StateId, EventId, StateId)> {
        self.history.read().await.clone()
    }

    /// Reset the state machine
    pub async fn reset(&self, context: &dyn ActionContext) -> Result<()> {
        // Clear current state and history
        {
            let mut current = self.current_state.write().await;
            *current = None;

            let mut history = self.history.write().await;
            history.clear();
        }

        // Initialize again
        self.initialize(context).await
    }

    /// Check if the state machine is in a final state
    pub async fn is_complete(&self) -> bool {
        let current_state_id = match &*self.current_state.read().await {
            Some(state_id) => state_id,
            None => return false,
        };

        match self.states.get(current_state_id) {
            Some(state) => state.is_final(),
            None => false,
        }
    }

    /// Get all possible events from the current state
    pub async fn get_possible_events(&self) -> HashSet<EventId> {
        let current_state_id = match &*self.current_state.read().await {
            Some(state_id) => state_id,
            None => return HashSet::new(),
        };

        let mut events = HashSet::new();
        for transition in &self.transitions {
            if transition.source() == current_state_id {
                events.insert(transition.event().clone());
            }
        }

        events
    }
}

/// Builder for state machines
#[derive(Debug, Default)]
pub struct StateMachineBuilder {
    /// State machine ID
    id: Option<String>,
    /// State machine name
    name: Option<String>,
    /// State machine description
    description: Option<String>,
    /// States
    states: Vec<State>,
    /// Transitions
    transitions: Vec<Transition>,
    /// Initial state ID
    initial_state: Option<StateId>,
}

impl StateMachineBuilder {
    /// Create a new state machine builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the state machine ID
    pub fn with_id<S: Into<String>>(mut self, id: S) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the state machine name
    pub fn with_name<S: Into<String>>(mut self, name: S) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the state machine description
    pub fn with_description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add a state
    pub fn with_state(mut self, state: State) -> Self {
        self.states.push(state);
        self
    }

    /// Add a transition
    pub fn with_transition(mut self, transition: Transition) -> Self {
        self.transitions.push(transition);
        self
    }

    /// Set the initial state
    pub fn with_initial_state<S: Into<StateId>>(mut self, state_id: S) -> Self {
        self.initial_state = Some(state_id.into());
        self
    }

    /// Build the state machine
    pub fn build(self) -> Result<StateMachine> {
        let id = self.id.ok_or_else(|| Error::validation("State machine ID is required"))?;
        let name = self.name.ok_or_else(|| Error::validation("State machine name is required"))?;

        let mut state_machine = StateMachine::new(id, name);

        if let Some(description) = self.description {
            state_machine.set_description(description);
        }

        // Add states
        for state in self.states {
            state_machine.add_state(state)?;
        }

        // Set initial state
        if let Some(initial_state) = self.initial_state {
            state_machine.set_initial_state(initial_state)?;
        }

        // Add transitions
        for transition in self.transitions {
            state_machine.add_transition(transition)?;
        }

        Ok(state_machine)
    }
}
