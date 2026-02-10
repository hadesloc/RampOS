//! GraphQL Subscription resolvers using tokio broadcast channels

use async_graphql::{Context, Subscription};
use chrono::Utc;
use futures::stream::{Stream, StreamExt};
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

use super::types::IntentStatusEvent;

/// Broadcast sender for intent status change events.
/// Clone the sender to obtain new receivers for each subscriber.
pub type IntentEventSender = broadcast::Sender<IntentStatusEvent>;

/// Create a new broadcast channel for intent status events.
/// Returns sender. New receivers are created via sender.subscribe().
pub fn create_intent_event_channel() -> IntentEventSender {
    let (tx, _rx) = broadcast::channel(256);
    tx
}

/// Publish an intent status change event to all active subscribers.
pub fn publish_intent_status(
    sender: &IntentEventSender,
    intent_id: String,
    tenant_id: String,
    new_status: String,
) {
    let event = IntentStatusEvent {
        intent_id,
        tenant_id,
        new_status,
        timestamp: Utc::now(),
    };
    // Ignore send errors (no active subscribers)
    let _ = sender.send(event);
}

/// A concrete stream type for subscriptions
type IntentEventStream = Pin<Box<dyn Stream<Item = IntentStatusEvent> + Send>>;

/// Root subscription object for the GraphQL API
pub struct SubscriptionRoot;

#[Subscription]
impl SubscriptionRoot {
    /// Subscribe to intent status changes for a specific tenant.
    ///
    /// Returns a stream of IntentStatusEvent that fires whenever
    /// an intent transitions to a new state within the given tenant.
    async fn intent_status_changed(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Tenant ID to subscribe to")] tenant_id: String,
    ) -> async_graphql::Result<IntentEventStream> {
        let sender = ctx.data::<Arc<IntentEventSender>>()?;
        let rx = sender.subscribe();

        let stream = BroadcastStream::new(rx)
            .filter_map(move |result| {
                let tenant_id = tenant_id.clone();
                async move {
                    match result {
                        Ok(event) if event.tenant_id == tenant_id => Some(event),
                        _ => None,
                    }
                }
            });

        Ok(Box::pin(stream))
    }
}
