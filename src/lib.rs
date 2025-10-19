use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// Core trait that defines what an event can do
trait EventHandler<Args>: Send {
    fn call(&mut self, args: Args);
}

// Implement for closures
impl<F, Args> EventHandler<Args> for F
where
    F: FnMut(Args) + Send,
{
    fn call(&mut self, args: Args) {
        self(args);
    }
}

/// A unique identifier for a subscription.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Subscription(usize);

impl Subscription {
    pub fn new(id: usize) -> Subscription {
        return Subscription(id);
    }
}

/// A thread-safe event system that allows multiple subscribers to register callbacks.
///
/// Subscribers remain active until explicitly unsubscribed using the `SubscriptionId`.
pub struct Event<Args> {
    handlers: Arc<Mutex<EventHandlers<Args>>>,
}

struct EventHandlers<Args> {
    handlers: HashMap<Subscription, Box<dyn EventHandler<Args>>>,
    next_id: usize,
}

impl<Args> Event<Args> {
    /// Creates a new event with no subscribers.
    pub fn new() -> Self {
        Event {
            handlers: Arc::new(Mutex::new(EventHandlers {
                handlers: HashMap::new(),
                next_id: 0,
            })),
        }
    }

    /// Subscribes a callback to this event.
    ///
    /// Returns a `SubscriptionId` that must be used to unsubscribe later.
    /// The subscription will remain active until explicitly unsubscribed.
    pub fn subscribe<F>(&self, handler: F) -> Subscription
    where
        F: FnMut(Args) + Send + 'static,
    {
        let mut handlers = self.handlers.lock().unwrap();
        let id = Subscription(handlers.next_id);
        handlers.next_id += 1;
        handlers.handlers.insert(id, Box::new(handler));
        id
    }

    /// Unsubscribes a callback from this event.
    ///
    /// Returns `true` if the subscription was found and removed, `false` otherwise.
    pub fn unsubscribe(&self, id: Subscription) -> bool {
        let mut handlers = self.handlers.lock().unwrap();
        return handlers.handlers.remove(&id).is_some();
    }

    /// Removes all subscribers from this event.
    pub fn unsubscribe_all(&self) {
        let mut handlers = self.handlers.lock().unwrap();
        handlers.handlers.clear();
    }

    /// Triggers the event, calling all subscribed handlers with the provided arguments.
    pub fn invoke(&self, args: Args)
    where
        Args: Clone,
    {
        let ids: Vec<Subscription> = {
            let handlers = self.handlers.lock().unwrap();
            handlers.handlers.keys().copied().collect()
        };

        for id in ids {
            let mut handlers = self.handlers.lock().unwrap();
            if let Some(handler) = handlers.handlers.get_mut(&id) {
                handler.call(args.clone());
            }
        }
    }

    /// Returns the current number of active subscribers.
    pub fn subscriber_count(&self) -> usize {
        let handlers = self.handlers.lock().unwrap();
        return handlers.handlers.len();
    }
}

impl<Args> Clone for Event<Args> {
    fn clone(&self) -> Self {
        Event {
            handlers: Arc::clone(&self.handlers),
        }
    }
}

impl<Args> Default for Event<Args> {
    fn default() -> Self {
        Self::new()
    }
}
