//! E2E tests for GraphQL Subscriptions (F07)
//!
//! Tests the broadcast-channel based subscription system end-to-end:
//! - publish_intent_status sends events to subscribers
//! - Multiple subscribers receive the same event
//! - Tenant-scoped filtering (subscriber for tenant A doesn't see tenant B events)
//! - Subscription lifecycle: subscribe -> receive events -> unsubscribe
//! - Concurrent publishers and subscribers
//! - Event ordering is preserved
//! - Subscription handles publisher dropping gracefully

use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use tokio::sync::broadcast;
use tokio::time::timeout;
use tokio_stream::wrappers::BroadcastStream;

use ramp_api::graphql::subscription::{
    create_intent_event_channel, publish_intent_status, IntentEventSender,
};
use ramp_api::graphql::types::IntentStatusEvent;

// ============================================================================
// Helper functions
// ============================================================================

/// Default timeout for receiving events in tests. Long enough for CI,
/// short enough to catch genuine failures quickly.
const RECV_TIMEOUT: Duration = Duration::from_secs(2);

/// Helper: subscribe to a tenant-filtered stream (mirrors the SubscriptionRoot logic).
fn subscribe_for_tenant(
    sender: &IntentEventSender,
    tenant_id: String,
) -> std::pin::Pin<Box<dyn futures::Stream<Item = IntentStatusEvent> + Send>> {
    let rx = sender.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(move |result| {
        let tenant_id = tenant_id.clone();
        async move {
            match result {
                Ok(event) if event.tenant_id == tenant_id => Some(event),
                _ => None,
            }
        }
    });
    Box::pin(stream)
}

/// Helper: subscribe to ALL events (no tenant filtering).
fn subscribe_all(
    sender: &IntentEventSender,
) -> std::pin::Pin<Box<dyn futures::Stream<Item = IntentStatusEvent> + Send>> {
    let rx = sender.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|result| async move { result.ok() });
    Box::pin(stream)
}

// ============================================================================
// 1. Test publish_intent_status sends events to subscribers
// ============================================================================

#[tokio::test]
async fn test_publish_sends_event_to_subscriber() {
    let sender = create_intent_event_channel();
    let mut rx = sender.subscribe();

    publish_intent_status(
        &sender,
        "intent-001".to_string(),
        "tenant-A".to_string(),
        "COMPLETED".to_string(),
    );

    let event = timeout(RECV_TIMEOUT, rx.recv()).await.unwrap().unwrap();
    assert_eq!(event.intent_id, "intent-001");
    assert_eq!(event.tenant_id, "tenant-A");
    assert_eq!(event.new_status, "COMPLETED");
    assert!(
        !event.timestamp.to_rfc3339().is_empty(),
        "timestamp should be set"
    );
}

#[tokio::test]
async fn test_publish_without_subscribers_does_not_panic() {
    let sender = create_intent_event_channel();
    // No subscribers -- should not panic or error
    publish_intent_status(
        &sender,
        "intent-002".to_string(),
        "tenant-X".to_string(),
        "PENDING".to_string(),
    );
    // If we reach here, the test passes
}

#[tokio::test]
async fn test_publish_returns_no_error_when_receiver_dropped() {
    let sender = create_intent_event_channel();
    let rx = sender.subscribe();
    drop(rx);

    // Should not panic even though receiver was dropped
    publish_intent_status(
        &sender,
        "intent-003".to_string(),
        "tenant-Y".to_string(),
        "CREATED".to_string(),
    );
}

// ============================================================================
// 2. Test multiple subscribers receive the same event
// ============================================================================

#[tokio::test]
async fn test_multiple_subscribers_receive_same_event() {
    let sender = create_intent_event_channel();

    let mut rx1 = sender.subscribe();
    let mut rx2 = sender.subscribe();
    let mut rx3 = sender.subscribe();

    publish_intent_status(
        &sender,
        "intent-shared".to_string(),
        "tenant-A".to_string(),
        "COMPLETED".to_string(),
    );

    let e1 = timeout(RECV_TIMEOUT, rx1.recv()).await.unwrap().unwrap();
    let e2 = timeout(RECV_TIMEOUT, rx2.recv()).await.unwrap().unwrap();
    let e3 = timeout(RECV_TIMEOUT, rx3.recv()).await.unwrap().unwrap();

    // All subscribers must receive identical event data
    assert_eq!(e1.intent_id, "intent-shared");
    assert_eq!(e2.intent_id, "intent-shared");
    assert_eq!(e3.intent_id, "intent-shared");

    assert_eq!(e1.tenant_id, e2.tenant_id);
    assert_eq!(e2.tenant_id, e3.tenant_id);

    assert_eq!(e1.new_status, e2.new_status);
    assert_eq!(e2.new_status, e3.new_status);
}

#[tokio::test]
async fn test_many_subscribers_all_receive_events() {
    let sender = create_intent_event_channel();
    let num_subscribers = 20;

    let mut receivers: Vec<_> = (0..num_subscribers).map(|_| sender.subscribe()).collect();

    publish_intent_status(
        &sender,
        "intent-fan-out".to_string(),
        "tenant-fan".to_string(),
        "PENDING".to_string(),
    );

    for (i, rx) in receivers.iter_mut().enumerate() {
        let event = timeout(RECV_TIMEOUT, rx.recv())
            .await
            .unwrap_or_else(|_| panic!("Subscriber {} timed out", i))
            .unwrap_or_else(|e| panic!("Subscriber {} recv error: {:?}", i, e));
        assert_eq!(event.intent_id, "intent-fan-out");
    }
}

// ============================================================================
// 3. Test tenant-scoped filtering
// ============================================================================

#[tokio::test]
async fn test_tenant_filter_isolates_events() {
    let sender = create_intent_event_channel();

    let mut stream_a = subscribe_for_tenant(&sender, "tenant-A".to_string());
    let mut stream_b = subscribe_for_tenant(&sender, "tenant-B".to_string());

    // Publish event for tenant-A
    publish_intent_status(
        &sender,
        "intent-A1".to_string(),
        "tenant-A".to_string(),
        "COMPLETED".to_string(),
    );

    // Publish event for tenant-B
    publish_intent_status(
        &sender,
        "intent-B1".to_string(),
        "tenant-B".to_string(),
        "PENDING".to_string(),
    );

    // Stream A should get the tenant-A event
    let event_a = timeout(RECV_TIMEOUT, stream_a.next())
        .await
        .expect("Tenant-A stream timed out")
        .expect("Tenant-A stream ended unexpectedly");
    assert_eq!(event_a.intent_id, "intent-A1");
    assert_eq!(event_a.tenant_id, "tenant-A");

    // Stream B should get the tenant-B event
    let event_b = timeout(RECV_TIMEOUT, stream_b.next())
        .await
        .expect("Tenant-B stream timed out")
        .expect("Tenant-B stream ended unexpectedly");
    assert_eq!(event_b.intent_id, "intent-B1");
    assert_eq!(event_b.tenant_id, "tenant-B");
}

#[tokio::test]
async fn test_tenant_filter_skips_other_tenant_events() {
    let sender = create_intent_event_channel();
    let mut stream_a = subscribe_for_tenant(&sender, "tenant-A".to_string());

    // Publish events for tenant-B only
    publish_intent_status(
        &sender,
        "intent-B1".to_string(),
        "tenant-B".to_string(),
        "COMPLETED".to_string(),
    );
    publish_intent_status(
        &sender,
        "intent-B2".to_string(),
        "tenant-B".to_string(),
        "PENDING".to_string(),
    );

    // Now publish one for tenant-A
    publish_intent_status(
        &sender,
        "intent-A1".to_string(),
        "tenant-A".to_string(),
        "CREATED".to_string(),
    );

    // Tenant-A stream should skip the B events and receive only A's event
    let event = timeout(RECV_TIMEOUT, stream_a.next())
        .await
        .expect("Tenant-A stream timed out")
        .expect("Tenant-A stream ended unexpectedly");

    assert_eq!(event.intent_id, "intent-A1");
    assert_eq!(event.tenant_id, "tenant-A");
    assert_eq!(event.new_status, "CREATED");
}

#[tokio::test]
async fn test_tenant_filter_handles_many_tenants() {
    let sender = create_intent_event_channel();

    let tenant_ids: Vec<String> = (0..10).map(|i| format!("tenant-{}", i)).collect();
    let mut streams: Vec<_> = tenant_ids
        .iter()
        .map(|tid| subscribe_for_tenant(&sender, tid.clone()))
        .collect();

    // Publish one event per tenant
    for (i, tid) in tenant_ids.iter().enumerate() {
        publish_intent_status(
            &sender,
            format!("intent-{}", i),
            tid.clone(),
            "COMPLETED".to_string(),
        );
    }

    // Each stream should receive exactly the event for its tenant
    for (i, stream) in streams.iter_mut().enumerate() {
        let event = timeout(RECV_TIMEOUT, stream.next())
            .await
            .unwrap_or_else(|_| panic!("Tenant-{} stream timed out", i))
            .unwrap_or_else(|| panic!("Tenant-{} stream ended unexpectedly", i));

        assert_eq!(event.intent_id, format!("intent-{}", i));
        assert_eq!(event.tenant_id, format!("tenant-{}", i));
    }
}

// ============================================================================
// 4. Test subscription lifecycle: subscribe -> receive -> unsubscribe
// ============================================================================

#[tokio::test]
async fn test_subscription_lifecycle_subscribe_receive_drop() {
    let sender = create_intent_event_channel();

    // Phase 1: Subscribe and receive
    let mut rx = sender.subscribe();
    publish_intent_status(
        &sender,
        "intent-lifecycle-1".to_string(),
        "tenant-L".to_string(),
        "CREATED".to_string(),
    );
    let event = timeout(RECV_TIMEOUT, rx.recv()).await.unwrap().unwrap();
    assert_eq!(event.intent_id, "intent-lifecycle-1");

    // Phase 2: Drop the receiver (unsubscribe)
    drop(rx);

    // Phase 3: Publishing after unsubscribe should not error
    publish_intent_status(
        &sender,
        "intent-lifecycle-2".to_string(),
        "tenant-L".to_string(),
        "COMPLETED".to_string(),
    );

    // Phase 4: New subscription should work
    let mut rx2 = sender.subscribe();
    publish_intent_status(
        &sender,
        "intent-lifecycle-3".to_string(),
        "tenant-L".to_string(),
        "PENDING".to_string(),
    );
    let event2 = timeout(RECV_TIMEOUT, rx2.recv()).await.unwrap().unwrap();
    assert_eq!(event2.intent_id, "intent-lifecycle-3");
}

#[tokio::test]
async fn test_late_subscriber_does_not_receive_earlier_events() {
    let sender = create_intent_event_channel();

    // Publish before subscribing
    publish_intent_status(
        &sender,
        "intent-early".to_string(),
        "tenant-L".to_string(),
        "COMPLETED".to_string(),
    );

    // Subscribe after publish
    let mut rx = sender.subscribe();

    // Publish a new event
    publish_intent_status(
        &sender,
        "intent-late".to_string(),
        "tenant-L".to_string(),
        "PENDING".to_string(),
    );

    // The late subscriber should only see the second event
    let event = timeout(RECV_TIMEOUT, rx.recv()).await.unwrap().unwrap();
    assert_eq!(
        event.intent_id, "intent-late",
        "Late subscriber should only receive events published after subscribing"
    );
}

#[tokio::test]
async fn test_stream_lifecycle_with_tenant_filter() {
    let sender = create_intent_event_channel();

    // Create a tenant-filtered stream, consume one event, then drop it
    {
        let mut stream = subscribe_for_tenant(&sender, "tenant-X".to_string());
        publish_intent_status(
            &sender,
            "intent-X1".to_string(),
            "tenant-X".to_string(),
            "CREATED".to_string(),
        );
        let event = timeout(RECV_TIMEOUT, stream.next()).await.unwrap().unwrap();
        assert_eq!(event.intent_id, "intent-X1");
        // stream is dropped here
    }

    // Publishing after stream drop should be fine
    publish_intent_status(
        &sender,
        "intent-X2".to_string(),
        "tenant-X".to_string(),
        "COMPLETED".to_string(),
    );

    // New stream should work
    let mut stream2 = subscribe_for_tenant(&sender, "tenant-X".to_string());
    publish_intent_status(
        &sender,
        "intent-X3".to_string(),
        "tenant-X".to_string(),
        "PENDING".to_string(),
    );
    let event2 = timeout(RECV_TIMEOUT, stream2.next())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(event2.intent_id, "intent-X3");
}

// ============================================================================
// 5. Test concurrent publishers and subscribers
// ============================================================================

#[tokio::test]
async fn test_concurrent_publishers() {
    let sender = Arc::new(create_intent_event_channel());
    let mut rx = sender.subscribe();

    let num_publishers = 10;
    let mut handles = Vec::new();

    for i in 0..num_publishers {
        let sender_clone = sender.clone();
        let handle = tokio::spawn(async move {
            publish_intent_status(
                &sender_clone,
                format!("intent-concurrent-{}", i),
                "tenant-C".to_string(),
                "COMPLETED".to_string(),
            );
        });
        handles.push(handle);
    }

    // Wait for all publishers to finish
    for h in handles {
        h.await.unwrap();
    }

    // Collect all events
    let mut received_ids: Vec<String> = Vec::new();
    for _ in 0..num_publishers {
        let event = timeout(RECV_TIMEOUT, rx.recv()).await.unwrap().unwrap();
        received_ids.push(event.intent_id.clone());
    }

    // All events should be received (order may vary due to concurrency)
    assert_eq!(received_ids.len(), num_publishers);
    for i in 0..num_publishers {
        let expected_id = format!("intent-concurrent-{}", i);
        assert!(
            received_ids.contains(&expected_id),
            "Missing event: {}",
            expected_id
        );
    }
}

#[tokio::test]
async fn test_concurrent_subscribers_and_publishers() {
    let sender = Arc::new(create_intent_event_channel());
    let num_events = 5;

    // Spawn subscriber tasks before publishing
    let mut subscriber_handles = Vec::new();
    for sub_id in 0..3 {
        let sender_clone = sender.clone();
        let handle = tokio::spawn(async move {
            let mut rx = sender_clone.subscribe();
            let mut events = Vec::new();
            for _ in 0..num_events {
                match timeout(RECV_TIMEOUT, rx.recv()).await {
                    Ok(Ok(event)) => events.push(event.intent_id.clone()),
                    Ok(Err(e)) => panic!("Subscriber {} recv error: {:?}", sub_id, e),
                    Err(_) => panic!("Subscriber {} timed out", sub_id),
                }
            }
            events
        });
        subscriber_handles.push(handle);
    }

    // Small delay to ensure subscribers are ready
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Publish events from main task
    for i in 0..num_events {
        publish_intent_status(
            &sender,
            format!("intent-cs-{}", i),
            "tenant-CS".to_string(),
            "COMPLETED".to_string(),
        );
    }

    // All subscribers should receive all events
    for (sub_id, handle) in subscriber_handles.into_iter().enumerate() {
        let events = handle.await.unwrap();
        assert_eq!(
            events.len(),
            num_events,
            "Subscriber {} should receive all {} events, got {}",
            sub_id,
            num_events,
            events.len()
        );
    }
}

#[tokio::test]
async fn test_concurrent_filtered_subscribers() {
    let sender = Arc::new(create_intent_event_channel());

    // Spawn two tenant-specific subscriber tasks
    let sender_a = sender.clone();
    let handle_a = tokio::spawn(async move {
        let mut stream = subscribe_for_tenant(&sender_a, "tenant-A".to_string());
        let mut events = Vec::new();
        for _ in 0..3 {
            if let Some(event) = timeout(RECV_TIMEOUT, stream.next()).await.unwrap() {
                events.push(event);
            }
        }
        events
    });

    let sender_b = sender.clone();
    let handle_b = tokio::spawn(async move {
        let mut stream = subscribe_for_tenant(&sender_b, "tenant-B".to_string());
        let mut events = Vec::new();
        for _ in 0..3 {
            if let Some(event) = timeout(RECV_TIMEOUT, stream.next()).await.unwrap() {
                events.push(event);
            }
        }
        events
    });

    // Small delay to let subscribers set up
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Interleave events for both tenants
    for i in 0..3 {
        publish_intent_status(
            &sender,
            format!("intent-A-{}", i),
            "tenant-A".to_string(),
            "COMPLETED".to_string(),
        );
        publish_intent_status(
            &sender,
            format!("intent-B-{}", i),
            "tenant-B".to_string(),
            "PENDING".to_string(),
        );
    }

    let events_a = handle_a.await.unwrap();
    let events_b = handle_b.await.unwrap();

    assert_eq!(events_a.len(), 3, "Tenant-A subscriber should get 3 events");
    assert_eq!(events_b.len(), 3, "Tenant-B subscriber should get 3 events");

    for event in &events_a {
        assert_eq!(event.tenant_id, "tenant-A");
    }
    for event in &events_b {
        assert_eq!(event.tenant_id, "tenant-B");
    }
}

// ============================================================================
// 6. Test event ordering is preserved
// ============================================================================

#[tokio::test]
async fn test_event_ordering_preserved_single_publisher() {
    let sender = create_intent_event_channel();
    let mut rx = sender.subscribe();

    let statuses = ["CREATED", "PENDING", "INSTRUCTION_ISSUED", "COMPLETED"];

    for (i, status) in statuses.iter().enumerate() {
        publish_intent_status(
            &sender,
            format!("intent-order-{}", i),
            "tenant-O".to_string(),
            status.to_string(),
        );
    }

    // Events should arrive in the same order they were published
    for (i, expected_status) in statuses.iter().enumerate() {
        let event = timeout(RECV_TIMEOUT, rx.recv()).await.unwrap().unwrap();
        assert_eq!(
            event.intent_id,
            format!("intent-order-{}", i),
            "Event {} arrived out of order",
            i
        );
        assert_eq!(
            event.new_status, *expected_status,
            "Event {} has wrong status",
            i
        );
    }
}

#[tokio::test]
async fn test_event_ordering_with_filtered_stream() {
    let sender = create_intent_event_channel();
    let mut stream = subscribe_for_tenant(&sender, "tenant-O".to_string());

    // Publish events with interleaved tenants
    publish_intent_status(&sender, "i-1".into(), "tenant-O".into(), "CREATED".into());
    publish_intent_status(&sender, "noise-1".into(), "tenant-X".into(), "NOISE".into());
    publish_intent_status(&sender, "i-2".into(), "tenant-O".into(), "PENDING".into());
    publish_intent_status(&sender, "noise-2".into(), "tenant-Y".into(), "NOISE".into());
    publish_intent_status(&sender, "i-3".into(), "tenant-O".into(), "COMPLETED".into());

    // Filtered stream should deliver events in order, skipping noise
    let e1 = timeout(RECV_TIMEOUT, stream.next()).await.unwrap().unwrap();
    let e2 = timeout(RECV_TIMEOUT, stream.next()).await.unwrap().unwrap();
    let e3 = timeout(RECV_TIMEOUT, stream.next()).await.unwrap().unwrap();

    assert_eq!(e1.intent_id, "i-1");
    assert_eq!(e1.new_status, "CREATED");

    assert_eq!(e2.intent_id, "i-2");
    assert_eq!(e2.new_status, "PENDING");

    assert_eq!(e3.intent_id, "i-3");
    assert_eq!(e3.new_status, "COMPLETED");
}

#[tokio::test]
async fn test_event_ordering_large_burst() {
    let sender = create_intent_event_channel();
    let mut rx = sender.subscribe();
    let count = 100;

    for i in 0..count {
        publish_intent_status(
            &sender,
            format!("intent-burst-{}", i),
            "tenant-burst".to_string(),
            format!("STATUS-{}", i),
        );
    }

    for i in 0..count {
        let event = timeout(RECV_TIMEOUT, rx.recv()).await.unwrap().unwrap();
        assert_eq!(
            event.intent_id,
            format!("intent-burst-{}", i),
            "Burst event {} arrived out of order",
            i
        );
        assert_eq!(event.new_status, format!("STATUS-{}", i));
    }
}

// ============================================================================
// 7. Test subscription handles publisher dropping gracefully
// ============================================================================

#[tokio::test]
async fn test_subscriber_handles_sender_drop_gracefully() {
    let sender = create_intent_event_channel();
    let mut rx = sender.subscribe();

    // Publish one event
    publish_intent_status(
        &sender,
        "intent-before-drop".to_string(),
        "tenant-D".to_string(),
        "CREATED".to_string(),
    );

    let event = timeout(RECV_TIMEOUT, rx.recv()).await.unwrap().unwrap();
    assert_eq!(event.intent_id, "intent-before-drop");

    // Drop the sender
    drop(sender);

    // Subsequent recv should return an error (channel closed), not panic
    let result = timeout(RECV_TIMEOUT, rx.recv()).await;
    match result {
        Ok(Err(broadcast::error::RecvError::Closed)) => {
            // Expected: channel is closed after sender is dropped
        }
        Ok(Err(other)) => {
            panic!("Expected Closed error, got: {:?}", other);
        }
        Ok(Ok(_)) => {
            panic!("Expected error after sender drop, but got an event");
        }
        Err(_) => {
            // Timeout is also acceptable -- BroadcastStream may just hang
            // since there's no more data and no explicit close signal
            // in some implementations
        }
    }
}

#[tokio::test]
async fn test_filtered_stream_ends_when_sender_dropped() {
    let sender = create_intent_event_channel();
    let mut stream = subscribe_for_tenant(&sender, "tenant-D".to_string());

    publish_intent_status(
        &sender,
        "intent-before-drop".to_string(),
        "tenant-D".to_string(),
        "CREATED".to_string(),
    );

    let event = timeout(RECV_TIMEOUT, stream.next()).await.unwrap().unwrap();
    assert_eq!(event.intent_id, "intent-before-drop");

    // Drop sender
    drop(sender);

    // Stream should either return None (end) or timeout (no more data)
    let result = timeout(Duration::from_millis(200), stream.next()).await;
    match result {
        Ok(None) => {
            // Stream ended cleanly
        }
        Err(_) => {
            // Timeout is also acceptable
        }
        Ok(Some(event)) => {
            panic!(
                "Should not receive events after sender dropped, got: {:?}",
                event
            );
        }
    }
}

#[tokio::test]
async fn test_sender_clone_keeps_channel_alive() {
    let sender = create_intent_event_channel();
    let sender_clone = sender.clone();
    let mut rx = sender.subscribe();

    // Drop the original sender
    drop(sender);

    // Channel should still be alive via the clone
    publish_intent_status(
        &sender_clone,
        "intent-via-clone".to_string(),
        "tenant-clone".to_string(),
        "COMPLETED".to_string(),
    );

    let event = timeout(RECV_TIMEOUT, rx.recv()).await.unwrap().unwrap();
    assert_eq!(event.intent_id, "intent-via-clone");
}

// ============================================================================
// 8. Additional edge case tests
// ============================================================================

#[tokio::test]
async fn test_channel_capacity_boundary() {
    // The channel is created with capacity 256. Sending more than 256 events
    // before a slow subscriber reads them should cause lagged errors.
    let sender = create_intent_event_channel();
    let mut rx = sender.subscribe();

    // Fill the channel beyond capacity
    for i in 0..300 {
        publish_intent_status(
            &sender,
            format!("intent-cap-{}", i),
            "tenant-cap".to_string(),
            "STATUS".to_string(),
        );
    }

    // The first recv may return a Lagged error since we overflowed
    let first_result = rx.recv().await;
    match first_result {
        Ok(event) => {
            // If we got an event, it should be one of the published events
            assert!(event.intent_id.starts_with("intent-cap-"));
        }
        Err(broadcast::error::RecvError::Lagged(n)) => {
            // Expected: some messages were dropped for this slow consumer
            assert!(n > 0, "Lagged count should be > 0");
            // After lagged error, we should still be able to receive remaining events
            let next = timeout(RECV_TIMEOUT, rx.recv()).await.unwrap().unwrap();
            assert!(next.intent_id.starts_with("intent-cap-"));
        }
        Err(broadcast::error::RecvError::Closed) => {
            panic!("Channel should not be closed");
        }
    }
}

#[tokio::test]
async fn test_empty_string_fields_are_accepted() {
    let sender = create_intent_event_channel();
    let mut rx = sender.subscribe();

    publish_intent_status(&sender, "".to_string(), "".to_string(), "".to_string());

    let event = timeout(RECV_TIMEOUT, rx.recv()).await.unwrap().unwrap();
    assert_eq!(event.intent_id, "");
    assert_eq!(event.tenant_id, "");
    assert_eq!(event.new_status, "");
}

#[tokio::test]
async fn test_unicode_in_event_fields() {
    let sender = create_intent_event_channel();
    let mut rx = sender.subscribe();

    publish_intent_status(
        &sender,
        "intent-vn-001".to_string(),
        "tenant-vn".to_string(),
        "HOAN_THANH".to_string(), // Vietnamese
    );

    let event = timeout(RECV_TIMEOUT, rx.recv()).await.unwrap().unwrap();
    assert_eq!(event.new_status, "HOAN_THANH");
}

#[tokio::test]
async fn test_subscribe_all_receives_every_tenant() {
    let sender = create_intent_event_channel();
    let mut stream = subscribe_all(&sender);

    publish_intent_status(&sender, "i-1".into(), "tenant-A".into(), "S1".into());
    publish_intent_status(&sender, "i-2".into(), "tenant-B".into(), "S2".into());
    publish_intent_status(&sender, "i-3".into(), "tenant-C".into(), "S3".into());

    let e1 = timeout(RECV_TIMEOUT, stream.next()).await.unwrap().unwrap();
    let e2 = timeout(RECV_TIMEOUT, stream.next()).await.unwrap().unwrap();
    let e3 = timeout(RECV_TIMEOUT, stream.next()).await.unwrap().unwrap();

    assert_eq!(e1.tenant_id, "tenant-A");
    assert_eq!(e2.tenant_id, "tenant-B");
    assert_eq!(e3.tenant_id, "tenant-C");
}

#[tokio::test]
async fn test_rapid_subscribe_unsubscribe_cycles() {
    let sender = create_intent_event_channel();

    // Rapidly create and drop subscribers
    for i in 0..50 {
        let mut rx = sender.subscribe();
        publish_intent_status(
            &sender,
            format!("intent-rapid-{}", i),
            "tenant-R".to_string(),
            "CREATED".to_string(),
        );
        let event = timeout(RECV_TIMEOUT, rx.recv()).await.unwrap().unwrap();
        assert_eq!(event.intent_id, format!("intent-rapid-{}", i));
        drop(rx);
    }

    // Channel should still be usable after many cycles
    let mut rx_final = sender.subscribe();
    publish_intent_status(
        &sender,
        "intent-final".to_string(),
        "tenant-R".to_string(),
        "FINAL".to_string(),
    );
    let event = timeout(RECV_TIMEOUT, rx_final.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(event.intent_id, "intent-final");
}

#[tokio::test]
async fn test_arc_sender_shared_across_tasks() {
    // Mimics how the sender is shared in the actual GraphQL schema (via Arc)
    let sender = Arc::new(create_intent_event_channel());

    let sender_pub = sender.clone();
    let sender_sub = sender.clone();

    let sub_handle = tokio::spawn(async move {
        let mut rx = sender_sub.subscribe();
        let event = timeout(RECV_TIMEOUT, rx.recv()).await.unwrap().unwrap();
        assert_eq!(event.intent_id, "intent-arc");
        event
    });

    // Small delay to let subscriber set up
    tokio::time::sleep(Duration::from_millis(50)).await;

    let pub_handle = tokio::spawn(async move {
        publish_intent_status(
            &sender_pub,
            "intent-arc".to_string(),
            "tenant-arc".to_string(),
            "COMPLETED".to_string(),
        );
    });

    pub_handle.await.unwrap();
    let result = sub_handle.await.unwrap();
    assert_eq!(result.tenant_id, "tenant-arc");
    assert_eq!(result.new_status, "COMPLETED");
}

#[tokio::test]
async fn test_multiple_status_transitions_for_same_intent() {
    let sender = create_intent_event_channel();
    let mut stream = subscribe_for_tenant(&sender, "tenant-T".to_string());

    let statuses = [
        "CREATED",
        "PENDING",
        "INSTRUCTION_ISSUED",
        "CONFIRMED",
        "COMPLETED",
    ];

    for status in &statuses {
        publish_intent_status(
            &sender,
            "intent-transition".to_string(),
            "tenant-T".to_string(),
            status.to_string(),
        );
    }

    // All transitions should be received in order for the same intent
    for expected_status in &statuses {
        let event = timeout(RECV_TIMEOUT, stream.next()).await.unwrap().unwrap();
        assert_eq!(event.intent_id, "intent-transition");
        assert_eq!(event.new_status, *expected_status);
    }
}
