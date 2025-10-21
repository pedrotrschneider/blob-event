# blob-event

A lightweight, thread-safe event system for Rust that enables flexible publish-subscribe patterns with minimal overhead.

## Features

- **Thread-safe** - Uses `Arc<Mutex<>>` for safe concurrent access
- **Type-safe** - Generic over event argument types
- **Flexible subscriptions** - Supports closures with captured state
- **Manual lifetime control** - Subscriptions persist until explicitly removed
- **Zero dependencies** - Built using only Rust standard library
- **Cloneable events** - Share event instances across threads and modules

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
blob-event = { path = "." }
```

## Quick Start

### Basic Usage

```rust
use blob_event::Event;
use std::sync::{Arc, Mutex};

fn main() {
    // Create an event that takes an i32 parameter
    let event = Event::<i32>::new();
    
    // Subscribe to the event
    let subscription = event.subscribe(|value| {
        println!("Event triggered with value: {}", value);
    });
    
    // Trigger the event
    event.invoke(42);  // Prints: Event triggered with value: 42
    event.invoke(100); // Prints: Event triggered with value: 100
    
    // Unsubscribe when done
    event.unsubscribe(subscription);
}
```

### Events with No Parameters

```rust
use blob_event::Event;

let button_click = Event::<()>::new();

let subscription = button_click.subscribe(|()| {
    println!("Button was clicked!");
});

button_click.invoke(());
button_click.unsubscribe(subscription);
```

### Events with Multiple Parameters

Use tuples for multiple parameters:

```rust
use blob_event::Event;

let user_login = Event::<(String, u32)>::new();

let subscription = user_login.subscribe(|(username, user_id)| {
    println!("User {} (ID: {}) logged in", username, user_id);
});

user_login.invoke(("alice".to_string(), 123));
user_login.unsubscribe(subscription);
```

## Core Concepts

### Subscriptions

When you subscribe to an event, you receive a `Subscription` token. This token is required to unsubscribe later:

```rust
let event = Event::<String>::new();

// Subscribe and store the token
let sub = event.subscribe(|msg| {
    println!("Message: {}", msg);
});

// Later, use the token to unsubscribe
event.unsubscribe(sub);
```

**Important:** Subscriptions remain active until explicitly unsubscribed. They don't automatically clean up when they go out of scope.

### Multiple Subscribers

Multiple callbacks can subscribe to the same event:

```rust
use blob_event::Event;

let event = Event::<i32>::new();

let sub1 = event.subscribe(|x| println!("Handler 1: {}", x));
let sub2 = event.subscribe(|x| println!("Handler 2: {}", x * 2));
let sub3 = event.subscribe(|x| println!("Handler 3: {}", x * 3));

event.invoke(10);
// Prints:
// Handler 1: 10
// Handler 2: 20
// Handler 3: 30

// Clean up
event.unsubscribe(sub1);
event.unsubscribe(sub2);
event.unsubscribe(sub3);
```

### Stateful Callbacks

Closures can capture and modify state:

```rust
use blob_event::Event;
use std::sync::{Arc, Mutex};

let event = Event::<i32>::new();

// Create shared state
let counter = Arc::new(Mutex::new(0));
let counter_clone = Arc::clone(&counter);

// Subscribe with stateful closure
let sub = event.subscribe(move |value| {
    let mut count = counter_clone.lock().unwrap();
    *count += value;
});

event.invoke(5);
event.invoke(10);
event.invoke(3);

assert_eq!(*counter.lock().unwrap(), 18);

event.unsubscribe(sub);
```

## Advanced Usage

### Cloning Events

Events can be cloned to share them across different parts of your application:

```rust
use blob_event::Event;

let event1 = Event::<String>::new();
let event2 = event1.clone();

let sub = event1.subscribe(|msg| {
    println!("Received: {}", msg);
});

// Both references trigger the same handlers
event1.invoke("Hello".to_string());
event2.invoke("World".to_string());

event1.unsubscribe(sub);
```

### Thread Safety

Events are thread-safe and can be shared across threads:

```rust
use blob_event::Event;
use std::thread;
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};

let event = Event::<i32>::new();
let counter = Arc::new(AtomicUsize::new(0));
let counter_clone = Arc::clone(&counter);

let sub = event.subscribe(move |_| {
    counter_clone.fetch_add(1, Ordering::SeqCst);
});

// Spawn multiple threads that trigger the event
let mut handles = vec![];
for _ in 0..10 {
    let event_clone = event.clone();
    let handle = thread::spawn(move || {
        for _ in 0..100 {
            event_clone.invoke(1);
        }
    });
    handles.push(handle);
}

for handle in handles {
    handle.join().unwrap();
}

assert_eq!(counter.load(Ordering::SeqCst), 1000);
event.unsubscribe(sub);
```

### Managing Subscriptions

```rust
use blob_event::Event;

let event = Event::<i32>::new();

// Store subscription IDs for later management
let mut subscriptions = Vec::new();

subscriptions.push(event.subscribe(|x| println!("Handler 1: {}", x)));
subscriptions.push(event.subscribe(|x| println!("Handler 2: {}", x)));
subscriptions.push(event.subscribe(|x| println!("Handler 3: {}", x)));

// Check subscriber count
assert_eq!(event.subscriber_count(), 3);

// Unsubscribe all
for sub in subscriptions {
    event.unsubscribe(sub);
}

assert_eq!(event.subscriber_count(), 0);
```

### Clearing All Subscriptions

```rust
use blob_event::Event;

let event = Event::<()>::new();

event.subscribe(|| println!("Handler 1"));
event.subscribe(|| println!("Handler 2"));
event.subscribe(|| println!("Handler 3"));

assert_eq!(event.subscriber_count(), 3);

// Remove all subscriptions at once
event.unsubscribe_all();

assert_eq!(event.subscriber_count(), 0);
```

## Real-World Examples

### Game Event System

```rust
use blob_event::Event;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct Enemy {
    id: u32,
    health: i32,
}

struct Game {
    on_enemy_killed: Event<Enemy>,
    on_player_scored: Event<u32>,
    score: Arc<Mutex<u32>>,
}

impl Game {
    fn new() -> Self {
        let game = Game {
            on_enemy_killed: Event::new(),
            on_player_scored: Event::new(),
            score: Arc::new(Mutex::new(0)),
        };
        
        // Setup event handlers
        let score = Arc::clone(&game.score);
        game.on_enemy_killed.subscribe(move |enemy| {
            println!("Enemy {} defeated!", enemy.id);
            let mut s = score.lock().unwrap();
            *s += 100;
        });
        
        game.on_player_scored.subscribe(|points| {
            println!("Player scored {} points!", points);
        });
        
        game
    }
    
    fn kill_enemy(&self, enemy: Enemy) {
        self.on_enemy_killed.invoke(enemy);
    }
}

fn main() {
    let game = Game::new();
    
    game.kill_enemy(Enemy { id: 1, health: 0 });
    game.kill_enemy(Enemy { id: 2, health: 0 });
    
    println!("Final score: {}", game.score.lock().unwrap());
}
```

### UI Component System

```rust
use blob_event::Event;

struct Button {
    label: String,
    on_click: Event<()>,
}

impl Button {
    fn new(label: &str) -> Self {
        Button {
            label: label.to_string(),
            on_click: Event::new(),
        }
    }
    
    fn click(&self) {
        println!("Button '{}' clicked", self.label);
        self.on_click.invoke(());
    }
}

fn main() {
    let button = Button::new("Submit");
    
    // Subscribe to button clicks
    let sub1 = button.on_click.subscribe(|| {
        println!("Validating form...");
    });
    
    let sub2 = button.on_click.subscribe(|| {
        println!("Sending data to server...");
    });
    
    button.click();
    // Output:
    // Button 'Submit' clicked
    // Validating form...
    // Sending data to server...
    
    // Clean up
    button.on_click.unsubscribe(sub1);
    button.on_click.unsubscribe(sub2);
}
```

### Message Bus Pattern

```rust
use blob_event::Event;
use std::collections::HashMap;

struct MessageBus {
    channels: HashMap<String, Event<String>>,
}

impl MessageBus {
    fn new() -> Self {
        MessageBus {
            channels: HashMap::new(),
        }
    }
    
    fn create_channel(&mut self, name: &str) {
        self.channels.insert(name.to_string(), Event::new());
    }
    
    fn subscribe(&self, channel: &str, handler: impl FnMut(String) + Send + 'static) -> Option<blob_event::Subscription> {
        self.channels.get(channel).map(|event| event.subscribe(handler))
    }
    
    fn publish(&self, channel: &str, message: String) {
        if let Some(event) = self.channels.get(channel) {
            event.invoke(message);
        }
    }
}

fn main() {
    let mut bus = MessageBus::new();
    
    bus.create_channel("notifications");
    bus.create_channel("errors");
    
    let sub1 = bus.subscribe("notifications", |msg| {
        println!("[NOTIFICATION] {}", msg);
    }).unwrap();
    
    let sub2 = bus.subscribe("errors", |msg| {
        eprintln!("[ERROR] {}", msg);
    }).unwrap();
    
    bus.publish("notifications", "System started".to_string());
    bus.publish("errors", "Connection failed".to_string());
    
    // Clean up (in real code, store these properly)
    if let Some(event) = bus.channels.get("notifications") {
        event.unsubscribe(sub1);
    }
    if let Some(event) = bus.channels.get("errors") {
        event.unsubscribe(sub2);
    }
}
```

### Observer Pattern for State Changes

```rust
use blob_event::Event;
use std::sync::{Arc, Mutex};

struct Counter {
    value: Arc<Mutex<i32>>,
    on_changed: Event<(i32, i32)>, // (old_value, new_value)
}

impl Counter {
    fn new(initial: i32) -> Self {
        Counter {
            value: Arc::new(Mutex::new(initial)),
            on_changed: Event::new(),
        }
    }
    
    fn increment(&self) {
        let mut val = self.value.lock().unwrap();
        let old = *val;
        *val += 1;
        let new = *val;
        drop(val);
        
        self.on_changed.invoke((old, new));
    }
    
    fn get(&self) -> i32 {
        *self.value.lock().unwrap()
    }
}

fn main() {
    let counter = Counter::new(0);
    
    let sub = counter.on_changed.subscribe(|(old, new)| {
        println!("Counter changed: {} -> {}", old, new);
    });
    
    counter.increment(); // Counter changed: 0 -> 1
    counter.increment(); // Counter changed: 1 -> 2
    counter.increment(); // Counter changed: 2 -> 3
    
    counter.on_changed.unsubscribe(sub);
}
```

## API Reference

### `Event<Args>`

The main event type, generic over the argument type.

#### Methods

- **`new() -> Self`**  
  Creates a new event with no subscribers.

- **`subscribe<F>(&self, handler: F) -> Subscription`**  
  Subscribes a callback to the event. Returns a subscription token.
  - `F: FnMut(Args) + Send + 'static`

- **`unsubscribe(&self, id: Subscription) -> bool`**  
  Removes a subscription. Returns `true` if the subscription was found and removed.

- **`unsubscribe_all(&self)`**  
  Removes all subscriptions from the event.

- **`invoke(&self, args: Args)`**  
  Triggers the event, calling all subscribed handlers with the provided arguments.
  - Requires `Args: Clone`

- **`subscriber_count(&self) -> usize`**  
  Returns the current number of active subscribers.

- **`clone(&self) -> Self`**  
  Creates a clone that shares the same subscription list.

- **`default() -> Self`**  
  Creates a new event (same as `new()`).

### `Subscription`

An opaque token representing an active subscription.

- Implements: `Copy`, `Clone`, `Debug`, `PartialEq`, `Eq`, `Hash`
- Can be stored in collections and compared

## Design Decisions

### Why Manual Unsubscription?

Unlike some event systems that use weak references or automatic cleanup, blob-event requires explicit unsubscription. This design:

- **Provides clarity** - It's always clear when a subscription is active
- **Prevents surprises** - No unexpected cleanup based on reference counts
- **Gives control** - You decide exactly when to remove subscriptions
- **Avoids overhead** - No need for weak reference tracking

### Thread Safety

The event system uses `Arc<Mutex<>>` internally, which means:

- Events can be safely shared across threads
- Multiple threads can subscribe, unsubscribe, and invoke simultaneously
- Each handler invocation is protected by the mutex
- For high-performance scenarios, consider using one event per thread

### Handler Order

Handlers are called in an unspecified order. Do not rely on the order of subscription for correctness.

## Performance Considerations

- Each `invoke()` locks the mutex to collect handler IDs, then locks again for each handler
- For performance-critical code, consider:
  - Minimizing the number of subscribers
  - Keeping handler execution time short
  - Using event batching if triggering many events rapidly

## Common Patterns

### Cleanup Helper

```rust
struct EventManager {
    subscriptions: Vec<blob_event::Subscription>,
}

impl EventManager {
    fn new() -> Self {
        EventManager {
            subscriptions: Vec::new(),
        }
    }
    
    fn subscribe<Args>(&mut self, event: &Event<Args>, handler: impl FnMut(Args) + Send + 'static) {
        let sub = event.subscribe(handler);
        self.subscriptions.push(sub);
    }
    
    fn clear(&mut self, event: &Event<impl Clone>) {
        for sub in self.subscriptions.drain(..) {
            event.unsubscribe(sub);
        }
    }
}
```

### Event Chain

```rust
let event_a = Event::<i32>::new();
let event_b = Event::<i32>::new();

let event_b_clone = event_b.clone();
event_a.subscribe(move |x| {
    event_b_clone.invoke(x * 2);
});

event_b.subscribe(|x| {
    println!("Final value: {}", x);
});

event_a.invoke(5); // Prints: Final value: 10
```

## License

MIT License - Copyright (c) 2025 Pedro Schneider

## Contributing

Contributions are welcome! Please feel free to submit issues or pull requests.

## Future Enhancements

Potential features for future versions:

- Async/await support for async handlers
- Priority-based handler ordering
- One-time subscriptions (auto-unsubscribe after first trigger)
- Handler execution policies (parallel, sequential, etc.)
- Event filtering/transformation middleware