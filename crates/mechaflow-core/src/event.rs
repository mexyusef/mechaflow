/*!
 * Event system for MechaFlow.
 *
 * This module provides a flexible, typed event system for publishing and subscribing
 * to events throughout the MechaFlow ecosystem.
 */
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

use tracing::{debug, trace, warn};

use crate::error::{Error, Result};
use crate::types::{Id, Value};

/// Maximum number of events that can be buffered in a channel
const DEFAULT_CHANNEL_CAPACITY: usize = 1024;

/// Event priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    /// Low priority events (non-critical)
    Low = 0,
    /// Normal priority events (default)
    Normal = 1,
    /// High priority events (important)
    High = 2,
    /// Critical priority events (system-critical)
    Critical = 3,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Normal
    }
}

/// Base trait for all events
pub trait Event: Any + Debug + Send + Sync {
    /// Get the event type name
    fn event_type(&self) -> &'static str;

    /// Get the event source ID
    fn source(&self) -> Option<&Id>;

    /// Get the event priority
    fn priority(&self) -> Priority {
        Priority::Normal
    }

    /// Get the event timestamp
    fn timestamp(&self) -> chrono::DateTime<chrono::Utc>;

    /// Convert the event to a value representation
    fn to_value(&self) -> Value;

    /// Clone the event as a box
    fn box_clone(&self) -> Box<dyn Event>;
}

impl Clone for Box<dyn Event> {
    fn clone(&self) -> Self {
        self.box_clone()
    }
}

/// Basic event implementation
#[derive(Debug, Clone)]
pub struct BasicEvent {
    /// Event type name
    pub event_type: &'static str,
    /// Source ID
    pub source: Option<Id>,
    /// Event priority
    pub priority: Priority,
    /// Event timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Event payload
    pub payload: Value,
}

impl BasicEvent {
    /// Create a new basic event
    pub fn new(event_type: &'static str, payload: Value) -> Self {
        Self {
            event_type,
            source: None,
            priority: Priority::Normal,
            timestamp: chrono::Utc::now(),
            payload,
        }
    }

    /// Set the source ID
    pub fn with_source(mut self, source: Id) -> Self {
        self.source = Some(source);
        self
    }

    /// Set the priority
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }
}

impl Event for BasicEvent {
    fn event_type(&self) -> &'static str {
        self.event_type
    }

    fn source(&self) -> Option<&Id> {
        self.source.as_ref()
    }

    fn priority(&self) -> Priority {
        self.priority
    }

    fn timestamp(&self) -> chrono::DateTime<chrono::Utc> {
        self.timestamp
    }

    fn to_value(&self) -> Value {
        self.payload.clone()
    }

    fn box_clone(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }
}

/// Type-specific event wrapper
#[derive(Debug, Clone)]
pub struct TypedEvent<T: Clone + Debug + Send + Sync + 'static> {
    /// Event type name
    pub event_type: &'static str,
    /// Source ID
    pub source: Option<Id>,
    /// Event priority
    pub priority: Priority,
    /// Event timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Strongly-typed payload
    pub payload: T,
}

impl<T: Clone + Debug + Send + Sync + 'static> TypedEvent<T> {
    /// Create a new typed event
    pub fn new(event_type: &'static str, payload: T) -> Self {
        Self {
            event_type,
            source: None,
            priority: Priority::Normal,
            timestamp: chrono::Utc::now(),
            payload,
        }
    }

    /// Set the source ID
    pub fn with_source(mut self, source: Id) -> Self {
        self.source = Some(source);
        self
    }

    /// Set the priority
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }
}

impl<T: Clone + Debug + Send + Sync + 'static> Event for TypedEvent<T> {
    fn event_type(&self) -> &'static str {
        self.event_type
    }

    fn source(&self) -> Option<&Id> {
        self.source.as_ref()
    }

    fn priority(&self) -> Priority {
        self.priority
    }

    fn timestamp(&self) -> chrono::DateTime<chrono::Utc> {
        self.timestamp
    }

    fn to_value(&self) -> Value {
        // Default implementation - should be overridden if T is convertible to Value
        Value::String(format!("{:?}", self.payload))
    }

    fn box_clone(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }
}

type EventSender<T> = broadcast::Sender<T>;
type EventReceiver<T> = broadcast::Receiver<T>;

/// Event bus for publishing and subscribing to events
#[derive(Debug)]
pub struct EventBus {
    channels: Mutex<HashMap<TypeId, Box<dyn Any + Send + Sync>>>,
    channel_capacity: usize,
}

impl EventBus {
    /// Create a new event bus
    pub fn new() -> Self {
        Self {
            channels: Mutex::new(HashMap::new()),
            channel_capacity: DEFAULT_CHANNEL_CAPACITY,
        }
    }

    /// Create a new event bus with a specific channel capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            channels: Mutex::new(HashMap::new()),
            channel_capacity: capacity,
        }
    }

    /// Publish an event
    pub fn publish<T: Clone + Debug + Send + Sync + 'static>(&self, event: T) -> Result<usize> {
        let type_id = TypeId::of::<T>();
        let mut channels = self.channels.lock().map_err(|_| Error::event("Failed to lock channels"))?;

        // Get or create channel for this event type
        let sender = if let Some(sender) = channels.get(&type_id) {
            sender.downcast_ref::<EventSender<T>>()
                .ok_or_else(|| Error::event("Failed to downcast sender"))?
                .clone()
        } else {
            let (sender, _) = broadcast::channel(self.channel_capacity);
            channels.insert(type_id, Box::new(sender.clone()));
            sender
        };

        // Send the event
        let receivers = sender.receiver_count();
        if receivers > 0 {
            match sender.send(event) {
                Ok(n) => {
                    trace!("Published event to {} receivers", n);
                    Ok(n)
                },
                Err(e) => {
                    warn!("Failed to publish event: {}", e);
                    Err(Error::event(format!("Failed to publish event: {}", e)))
                }
            }
        } else {
            debug!("No receivers for event");
            Ok(0)
        }
    }

    /// Subscribe to events of a specific type
    pub fn subscribe<T: Clone + Debug + Send + Sync + 'static>(&self) -> Result<EventReceiver<T>> {
        let type_id = TypeId::of::<T>();
        let mut channels = self.channels.lock().map_err(|_| Error::event("Failed to lock channels"))?;

        // Get or create channel for this event type
        let sender = if let Some(sender) = channels.get(&type_id) {
            sender.downcast_ref::<EventSender<T>>()
                .ok_or_else(|| Error::event("Failed to downcast sender"))?
                .clone()
        } else {
            let (sender, _) = broadcast::channel(self.channel_capacity);
            channels.insert(type_id, Box::new(sender.clone()));
            sender
        };

        Ok(sender.subscribe())
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// A shared event bus that can be cloned
#[derive(Debug, Clone)]
pub struct SharedEventBus(Arc<EventBus>);

impl SharedEventBus {
    /// Create a new shared event bus
    pub fn new() -> Self {
        Self(Arc::new(EventBus::new()))
    }

    /// Create a new shared event bus with a specific channel capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Arc::new(EventBus::with_capacity(capacity)))
    }

    /// Publish an event
    pub fn publish<T: Clone + Debug + Send + Sync + 'static>(&self, event: T) -> Result<usize> {
        self.0.publish(event)
    }

    /// Subscribe to events of a specific type
    pub fn subscribe<T: Clone + Debug + Send + Sync + 'static>(&self) -> Result<EventReceiver<T>> {
        self.0.subscribe()
    }
}

impl Default for SharedEventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Barrier;

    #[derive(Debug, Clone)]
    struct TestEvent {
        id: u32,
        message: String,
    }

    #[tokio::test]
    async fn test_publish_subscribe() -> Result<()> {
        let event_bus = EventBus::new();
        let mut rx = event_bus.subscribe::<TestEvent>()?;

        let event = TestEvent {
            id: 1,
            message: "Hello, world!".to_string(),
        };

        let receivers = event_bus.publish(event.clone())?;
        assert_eq!(receivers, 1);

        let received = rx.recv().await.map_err(|e| Error::event(e.to_string()))?;
        assert_eq!(received.id, event.id);
        assert_eq!(received.message, event.message);

        Ok(())
    }

    #[tokio::test]
    async fn test_multiple_subscribers() -> Result<()> {
        let event_bus = SharedEventBus::new();
        let mut rx1 = event_bus.subscribe::<TestEvent>()?;
        let mut rx2 = event_bus.subscribe::<TestEvent>()?;

        let event = TestEvent {
            id: 2,
            message: "Test message".to_string(),
        };

        let receivers = event_bus.publish(event.clone())?;
        assert_eq!(receivers, 2);

        let received1 = rx1.recv().await.map_err(|e| Error::event(e.to_string()))?;
        let received2 = rx2.recv().await.map_err(|e| Error::event(e.to_string()))?;

        assert_eq!(received1.id, event.id);
        assert_eq!(received1.message, event.message);
        assert_eq!(received2.id, event.id);
        assert_eq!(received2.message, event.message);

        Ok(())
    }

    #[tokio::test]
    async fn test_multiple_event_types() -> Result<()> {
        #[derive(Debug, Clone)]
        struct OtherEvent {
            value: String,
        }

        let event_bus = EventBus::new();
        let mut rx1 = event_bus.subscribe::<TestEvent>()?;
        let mut rx2 = event_bus.subscribe::<OtherEvent>()?;

        let test_event = TestEvent {
            id: 3,
            message: "Test event".to_string(),
        };

        let other_event = OtherEvent {
            value: "Other event".to_string(),
        };

        // Publish both event types
        event_bus.publish(test_event.clone())?;
        event_bus.publish(other_event.clone())?;

        // Each subscriber should receive only its event type
        let received1 = rx1.recv().await.map_err(|e| Error::event(e.to_string()))?;
        let received2 = rx2.recv().await.map_err(|e| Error::event(e.to_string()))?;

        assert_eq!(received1.id, test_event.id);
        assert_eq!(received1.message, test_event.message);
        assert_eq!(received2.value, other_event.value);

        Ok(())
    }

    #[tokio::test]
    async fn test_event_implementations() {
        // Test BasicEvent
        let payload = Value::String("test payload".to_string());
        let basic_event = BasicEvent::new("test", payload.clone())
            .with_source("source1".into())
            .with_priority(Priority::High);

        assert_eq!(basic_event.event_type(), "test");
        assert_eq!(basic_event.source().unwrap().as_str(), "source1");
        assert_eq!(basic_event.priority(), Priority::High);
        assert_eq!(basic_event.to_value(), payload);

        // Test TypedEvent
        let typed_event = TypedEvent::new("typed_test", "typed payload")
            .with_source("source2".into())
            .with_priority(Priority::Critical);

        assert_eq!(typed_event.event_type(), "typed_test");
        assert_eq!(typed_event.source().unwrap().as_str(), "source2");
        assert_eq!(typed_event.priority(), Priority::Critical);
        assert!(matches!(typed_event.to_value(), Value::String(_)));
    }

    #[tokio::test]
    async fn test_concurrent_publish() -> Result<()> {
        const NUM_PUBLISHERS: usize = 10;
        const EVENTS_PER_PUBLISHER: usize = 10;

        let event_bus = SharedEventBus::new();
        let mut rx = event_bus.subscribe::<TestEvent>()?;

        let barrier = Arc::new(Barrier::new(NUM_PUBLISHERS));

        let mut handles = Vec::with_capacity(NUM_PUBLISHERS);

        for publisher_id in 0..NUM_PUBLISHERS {
            let event_bus = event_bus.clone();
            let barrier = barrier.clone();

            let handle = tokio::spawn(async move {
                // Wait for all publishers to be ready
                barrier.wait().await;

                for i in 0..EVENTS_PER_PUBLISHER {
                    let event = TestEvent {
                        id: (publisher_id * EVENTS_PER_PUBLISHER + i) as u32,
                        message: format!("Event from publisher {}, index {}", publisher_id, i),
                    };

                    event_bus.publish(event).unwrap();
                }
            });

            handles.push(handle);
        }

        // Wait for all publishers to complete
        for handle in handles {
            handle.await.unwrap();
        }

        // Count received events
        let mut received_count = 0;
        while let Ok(_) = rx.try_recv() {
            received_count += 1;
        }

        assert_eq!(received_count, NUM_PUBLISHERS * EVENTS_PER_PUBLISHER);

        Ok(())
    }
}
