use blob_event::{Event, Subscription};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[test]
fn test_event_with_no_parameters() {
    let event = Event::<()>::new();
    let called = Arc::new(AtomicUsize::new(0));
    let called_clone = Arc::clone(&called);

    let sub = event.subscribe(move |()| {
        called_clone.fetch_add(1, Ordering::SeqCst);
    });

    event.invoke(());
    assert_eq!(called.load(Ordering::SeqCst), 1);

    event.invoke(());
    assert_eq!(called.load(Ordering::SeqCst), 2);

    event.unsubscribe(sub);
}

#[test]
fn test_event_with_single_parameter() {
    let event = Event::<i32>::new();
    let value = Arc::new(Mutex::new(0));
    let value_clone = Arc::clone(&value);

    let sub = event.subscribe(move |x| {
        *value_clone.lock().unwrap() = x;
    });

    event.invoke(42);
    assert_eq!(*value.lock().unwrap(), 42);

    event.invoke(100);
    assert_eq!(*value.lock().unwrap(), 100);

    event.unsubscribe(sub);
}

#[test]
fn test_event_with_multiple_parameters() {
    let event = Event::<(i32, String)>::new();
    let results = Arc::new(Mutex::new(Vec::new()));
    let results_clone = Arc::clone(&results);

    let sub = event.subscribe(move |(num, text)| {
        results_clone.lock().unwrap().push((num, text));
    });

    event.invoke((1, "hello".to_string()));
    event.invoke((2, "world".to_string()));

    let results = results.lock().unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0], (1, "hello".to_string()));
    assert_eq!(results[1], (2, "world".to_string()));

    drop(results);
    event.unsubscribe(sub);
}

#[test]
fn test_multiple_subscribers() {
    let event = Event::<i32>::new();
    let count1 = Arc::new(AtomicUsize::new(0));
    let count2 = Arc::new(AtomicUsize::new(0));
    let count3 = Arc::new(AtomicUsize::new(0));

    let count1_clone = Arc::clone(&count1);
    let count2_clone = Arc::clone(&count2);
    let count3_clone = Arc::clone(&count3);

    let sub1 = event.subscribe(move |_| {
        count1_clone.fetch_add(1, Ordering::SeqCst);
    });
    let sub2 = event.subscribe(move |_| {
        count2_clone.fetch_add(1, Ordering::SeqCst);
    });
    let sub3 = event.subscribe(move |_| {
        count3_clone.fetch_add(1, Ordering::SeqCst);
    });

    event.invoke(0);

    assert_eq!(count1.load(Ordering::SeqCst), 1);
    assert_eq!(count2.load(Ordering::SeqCst), 1);
    assert_eq!(count3.load(Ordering::SeqCst), 1);

    event.unsubscribe(sub1);
    event.unsubscribe(sub2);
    event.unsubscribe(sub3);
}

#[test]
fn test_unsubscribe() {
    let event = Event::<i32>::new();
    let count = Arc::new(AtomicUsize::new(0));
    let count_clone = Arc::clone(&count);

    let sub = event.subscribe(move |_| {
        count_clone.fetch_add(1, Ordering::SeqCst);
    });

    event.invoke(0);
    assert_eq!(count.load(Ordering::SeqCst), 1);

    let removed = event.unsubscribe(sub);
    assert!(removed);

    event.invoke(0);
    assert_eq!(count.load(Ordering::SeqCst), 1); // Still 1, not called again
}

#[test]
fn test_unsubscribe_returns_false_for_invalid_id() {
    let event = Event::<i32>::new();
    let sub = event.subscribe(|_| {});

    event.unsubscribe(sub);

    // Trying to unsubscribe again should return false
    let removed = event.unsubscribe(sub);
    assert!(!removed);

    // Unsubscribing a made-up ID should return false
    let fake_id = Subscription::new(9999);
    let removed = event.unsubscribe(fake_id);
    assert!(!removed);
}

#[test]
fn test_subscription_persists_across_scopes() {
    let event = Event::<i32>::new();
    let count = Arc::new(AtomicUsize::new(0));
    let count_clone = Arc::clone(&count);

    let sub = {
        // Subscribe in inner scope
        let count_for_closure = Arc::clone(&count);
        event.subscribe(move |_| {
            count_for_closure.fetch_add(1, Ordering::SeqCst);
        })
        // Subscription ID goes out of scope but subscription remains active
    };

    // Trigger after scope ends
    event.invoke(0);
    assert_eq!(count_clone.load(Ordering::SeqCst), 1);

    event.invoke(0);
    assert_eq!(count_clone.load(Ordering::SeqCst), 2);

    // Clean up
    event.unsubscribe(sub);
}

#[test]
fn test_subscriber_count() {
    let event = Event::<i32>::new();
    assert_eq!(event.subscriber_count(), 0);

    let sub1 = event.subscribe(|_| {});
    assert_eq!(event.subscriber_count(), 1);

    let sub2 = event.subscribe(|_| {});
    assert_eq!(event.subscriber_count(), 2);

    let sub3 = event.subscribe(|_| {});
    assert_eq!(event.subscriber_count(), 3);

    event.unsubscribe(sub2);
    assert_eq!(event.subscriber_count(), 2);

    event.unsubscribe(sub1);
    event.unsubscribe(sub3);
    assert_eq!(event.subscriber_count(), 0);
}

#[test]
fn test_stateful_callback() {
    let event = Event::<i32>::new();
    let sum = Arc::new(Mutex::new(0));
    let sum_clone = Arc::clone(&sum);

    let sub = event.subscribe(move |x| {
        let mut s = sum_clone.lock().unwrap();
        *s += x;
    });

    event.invoke(5);
    event.invoke(10);
    event.invoke(3);

    assert_eq!(*sum.lock().unwrap(), 18);

    event.unsubscribe(sub);
}

#[test]
fn test_event_clone() {
    let event1 = Event::<i32>::new();
    let event2 = event1.clone();

    let count = Arc::new(AtomicUsize::new(0));
    let count_clone = Arc::clone(&count);

    let sub = event1.subscribe(move |_| {
        count_clone.fetch_add(1, Ordering::SeqCst);
    });

    // Triggering cloned event should call the same handlers
    event2.invoke(0);
    assert_eq!(count.load(Ordering::SeqCst), 1);

    event1.invoke(0);
    assert_eq!(count.load(Ordering::SeqCst), 2);

    // Unsubscribe from cloned event
    event2.unsubscribe(sub);

    event1.invoke(0);
    assert_eq!(count.load(Ordering::SeqCst), 2); // Not called
}

#[test]
fn test_thread_safety_trigger_from_multiple_threads() {
    let event = Event::<i32>::new();
    let count = Arc::new(AtomicUsize::new(0));
    let count_clone = Arc::clone(&count);

    let sub = event.subscribe(move |x| {
        count_clone.fetch_add(x as usize, Ordering::SeqCst);
    });

    let event_clone1 = event.clone();
    let event_clone2 = event.clone();
    let event_clone3 = event.clone();

    let handle1 = thread::spawn(move || {
        for _ in 0..100 {
            event_clone1.invoke(1);
        }
    });

    let handle2 = thread::spawn(move || {
        for _ in 0..100 {
            event_clone2.invoke(1);
        }
    });

    let handle3 = thread::spawn(move || {
        for _ in 0..100 {
            event_clone3.invoke(1);
        }
    });

    handle1.join().unwrap();
    handle2.join().unwrap();
    handle3.join().unwrap();

    assert_eq!(count.load(Ordering::SeqCst), 300);

    event.unsubscribe(sub);
}

#[test]
fn test_thread_safety_subscribe_unsubscribe() {
    let event = Event::<i32>::new();
    let subs = Arc::new(Mutex::new(Vec::new()));

    let mut handles = vec![];

    for _ in 0..10 {
        let event_clone = event.clone();
        let subs_clone = Arc::clone(&subs);

        let handle = thread::spawn(move || {
            let sub = event_clone.subscribe(|_| {});
            subs_clone.lock().unwrap().push(sub);
            thread::sleep(Duration::from_millis(1));
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(event.subscriber_count(), 10);

    // Unsubscribe all
    for sub in subs.lock().unwrap().iter() {
        event.unsubscribe(*sub);
    }

    assert_eq!(event.subscriber_count(), 0);
}

#[test]
fn test_thread_safety_mixed_operations() {
    let event = Event::<i32>::new();
    let trigger_count = Arc::new(AtomicUsize::new(0));

    let count_clone = Arc::clone(&trigger_count);
    let sub1 = event.subscribe(move |_| {
        count_clone.fetch_add(1, Ordering::SeqCst);
    });

    let event_clone1 = event.clone();
    let event_clone2 = event.clone();
    let event_clone3 = event.clone();
    let subs = Arc::new(Mutex::new(Vec::new()));
    let subs_clone = Arc::clone(&subs);

    // Thread 1: Keep subscribing and unsubscribing
    let handle1 = thread::spawn(move || {
        for _ in 0..50 {
            let sub = event_clone1.subscribe(|_| {});
            thread::sleep(Duration::from_micros(100));
            event_clone1.unsubscribe(sub);
        }
    });

    // Thread 2: Keep triggering
    let handle2 = thread::spawn(move || {
        for i in 0..100 {
            event_clone2.invoke(i);
            thread::sleep(Duration::from_micros(50));
        }
    });

    // Thread 3: Subscribe with longer lifetime
    let handle3 = thread::spawn(move || {
        let sub = event_clone3.subscribe(|_| {});
        subs_clone.lock().unwrap().push(sub);
        thread::sleep(Duration::from_millis(10));
    });

    handle1.join().unwrap();
    handle2.join().unwrap();
    handle3.join().unwrap();

    event.unsubscribe(sub1);
    for sub in subs.lock().unwrap().iter() {
        event.unsubscribe(*sub);
    }

    // Should be able to trigger after all operations
    event.invoke(0);
    assert_eq!(event.subscriber_count(), 0);
}

#[test]
fn test_no_subscribers() {
    let event = Event::<i32>::new();

    // Should not panic when triggering with no subscribers
    event.invoke(42);
    assert_eq!(event.subscriber_count(), 0);
}

#[test]
fn test_event_default() {
    let event: Event<i32> = Event::default();
    assert_eq!(event.subscriber_count(), 0);

    let count = Arc::new(AtomicUsize::new(0));
    let count_clone = Arc::clone(&count);

    let sub = event.subscribe(move |_| {
        count_clone.fetch_add(1, Ordering::SeqCst);
    });

    event.invoke(0);
    assert_eq!(count.load(Ordering::SeqCst), 1);

    event.unsubscribe(sub);
}

#[test]
fn test_nested_event_trigger() {
    let event1 = Event::<i32>::new();
    let event2 = Event::<i32>::new();

    let count = Arc::new(AtomicUsize::new(0));
    let count_clone1 = Arc::clone(&count);
    let count_clone2 = Arc::clone(&count);

    let event2_clone = event2.clone();
    let sub1 = event1.subscribe(move |x| {
        count_clone1.fetch_add(1, Ordering::SeqCst);
        event2_clone.invoke(x * 2);
    });

    let sub2 = event2.subscribe(move |_| {
        count_clone2.fetch_add(1, Ordering::SeqCst);
    });

    event1.invoke(5);

    // Both handlers should be called
    assert_eq!(count.load(Ordering::SeqCst), 2);

    event1.unsubscribe(sub1);
    event2.unsubscribe(sub2);
}

#[test]
fn test_handler_receives_correct_values() {
    let event = Event::<(i32, String, bool)>::new();
    let received = Arc::new(Mutex::new(None));
    let received_clone = Arc::clone(&received);

    let sub = event.subscribe(move |(num, text, flag)| {
        *received_clone.lock().unwrap() = Some((num, text, flag));
    });

    event.invoke((42, "test".to_string(), true));

    let result = received.lock().unwrap();
    assert_eq!(*result, Some((42, "test".to_string(), true)));

    drop(result);
    event.unsubscribe(sub);
}

#[test]
fn test_clear() {
    let event = Event::<i32>::new();

    let _sub1 = event.subscribe(|_| {});
    let _sub2 = event.subscribe(|_| {});
    let _sub3 = event.subscribe(|_| {});

    assert_eq!(event.subscriber_count(), 3);

    event.unsubscribe_all();

    assert_eq!(event.subscriber_count(), 0);
}

#[test]
fn test_subscription_id_can_be_stored() {
    let event = Event::<i32>::new();

    // Test that SubscriptionId can be stored in various containers
    let mut ids = Vec::new();

    ids.push(event.subscribe(|_| {}));
    ids.push(event.subscribe(|_| {}));
    ids.push(event.subscribe(|_| {}));

    assert_eq!(event.subscriber_count(), 3);

    // Unsubscribe all
    for id in ids {
        event.unsubscribe(id);
    }

    assert_eq!(event.subscriber_count(), 0);
}
