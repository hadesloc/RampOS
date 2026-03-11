//! WebSocket Real-time Updates Handler
//!
//! Provides real-time streaming of intent status changes and user events
//! via WebSocket connections with JWT authentication.
//!
//! Endpoint: GET /v1/portal/ws?token=<jwt>
//!
//! Protocol:
//! 1. Client connects with JWT token as query parameter
//! 2. Server verifies JWT and associates connection with user
//! 3. Client sends subscription messages to filter events
//! 4. Server pushes matching events to client
//! 5. Ping/pong keepalive every 30 seconds

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::IntoResponse,
};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::middleware::portal_auth::PortalClaims;

/// Event broadcast to WebSocket clients
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsEvent {
    /// Event type (e.g., "intent.updated", "intent.completed")
    pub event_type: String,
    /// Intent ID (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent_id: Option<String>,
    /// User ID the event belongs to
    pub user_id: String,
    /// Tenant ID
    pub tenant_id: String,
    /// Event payload
    pub data: serde_json::Value,
    /// Timestamp (ISO 8601)
    pub timestamp: String,
}

/// Client-to-server subscription message
#[derive(Debug, Deserialize)]
#[serde(tag = "action")]
enum ClientMessage {
    /// Subscribe to specific intent updates
    #[serde(rename = "subscribe")]
    Subscribe {
        /// Intent IDs to subscribe to (empty = all user events)
        #[serde(default, alias = "intent_ids")]
        #[serde(rename = "intentIds")]
        intent_ids: Vec<String>,
    },
    /// Unsubscribe from intent updates
    #[serde(rename = "unsubscribe")]
    Unsubscribe {
        #[serde(default, alias = "intent_ids")]
        #[serde(rename = "intentIds")]
        intent_ids: Vec<String>,
    },
    /// Ping message
    #[serde(rename = "ping")]
    Ping,
}

/// Server-to-client response
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ServerMessage {
    #[serde(rename = "type")]
    msg_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Shared state for WebSocket connections
#[derive(Clone)]
pub struct WsState {
    /// Broadcast channel for sending events to all connected clients
    pub tx: broadcast::Sender<WsEvent>,
    /// JWT secret for token verification
    jwt_secret: String,
}

impl WsState {
    pub fn new(jwt_secret: String) -> Self {
        let (tx, _) = broadcast::channel(1024);
        Self { tx, jwt_secret }
    }

    /// Publish an event to all connected WebSocket clients
    pub fn publish(&self, event: WsEvent) {
        // Ignore send errors (no receivers connected)
        let _ = self.tx.send(event);
    }

    /// Publish a tenant-scoped incident timeline update using the existing event envelope.
    pub fn publish_incident_timeline_update(
        &self,
        tenant_id: impl Into<String>,
        user_id: impl Into<String>,
        incident_id: impl Into<String>,
        intent_id: Option<&str>,
        data: serde_json::Value,
    ) {
        self.publish(WsEvent {
            event_type: "incident.timeline.updated".to_string(),
            intent_id: intent_id.map(|value| value.to_string()),
            user_id: user_id.into(),
            tenant_id: tenant_id.into(),
            data: serde_json::json!({
                "incidentId": incident_id.into(),
                "timeline": data,
            }),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
    }
}

fn should_deliver_event_to_client(event: &WsEvent, user_id: &str, tenant_id: &str) -> bool {
    if event.tenant_id != tenant_id {
        return false;
    }

    if event.event_type == "incident.timeline.updated" {
        return event.user_id == user_id;
    }

    event.user_id == user_id || event.user_id == "system"
}

/// Query parameters for WebSocket connection
#[derive(Debug, Deserialize)]
pub struct WsQuery {
    /// JWT access token
    pub token: String,
}

/// WebSocket upgrade handler
///
/// Authenticates the user via JWT token in query params, then upgrades
/// the connection to WebSocket.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<WsQuery>,
    State(ws_state): State<Arc<WsState>>,
) -> impl IntoResponse {
    // Verify JWT token before upgrading
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;

    let decoding_key = DecodingKey::from_secret(ws_state.jwt_secret.as_bytes());

    let token_data = match decode::<PortalClaims>(&query.token, &decoding_key, &validation) {
        Ok(data) => data,
        Err(e) => {
            warn!(error = %e, "WebSocket auth failed: invalid JWT");
            return axum::response::Response::builder()
                .status(401)
                .body(axum::body::Body::from("Unauthorized: invalid token"))
                .unwrap()
                .into_response();
        }
    };

    let claims = token_data.claims;

    // Verify it's an access token
    if claims.token_type != "access" {
        return axum::response::Response::builder()
            .status(401)
            .body(axum::body::Body::from("Unauthorized: invalid token type"))
            .unwrap()
            .into_response();
    }

    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            return axum::response::Response::builder()
                .status(401)
                .body(axum::body::Body::from("Unauthorized: invalid user ID"))
                .unwrap()
                .into_response();
        }
    };

    let tenant_id = claims
        .tenant_id
        .as_deref()
        .and_then(|t| Uuid::parse_str(t).ok())
        .unwrap_or(Uuid::nil());

    info!(
        user_id = %user_id,
        tenant_id = %tenant_id,
        "WebSocket connection authenticated"
    );

    // Upgrade the connection
    ws.on_upgrade(move |socket| handle_socket(socket, user_id, tenant_id, ws_state))
}

/// Handle an individual WebSocket connection
async fn handle_socket(socket: WebSocket, user_id: Uuid, tenant_id: Uuid, ws_state: Arc<WsState>) {
    let (mut sender, mut receiver) = socket.split();

    // Track subscriptions for this client
    // None = subscribe to all user events, Some(set) = only specific intent IDs
    let subscriptions: Arc<RwLock<Option<HashSet<String>>>> = Arc::new(RwLock::new(None));

    // Subscribe to the broadcast channel
    let mut rx = ws_state.tx.subscribe();

    // Send welcome message
    let welcome = ServerMessage {
        msg_type: "connected".to_string(),
        data: Some(serde_json::json!({
            "userId": user_id.to_string(),
            "message": "WebSocket connected. Send {\"action\":\"subscribe\"} to receive all events or {\"action\":\"subscribe\",\"intentIds\":[\"...\"]} for specific intents."
        })),
        error: None,
    };
    if let Ok(msg) = serde_json::to_string(&welcome) {
        let _ = sender.send(Message::Text(msg)).await;
    }

    let user_id_str = user_id.to_string();
    let tenant_id_str = tenant_id.to_string();

    // Spawn task to forward broadcast events to this client
    let subs_clone = subscriptions.clone();
    let user_id_filter = user_id_str.clone();
    let tenant_id_filter = tenant_id_str.clone();
    let mut send_task = tokio::spawn(async move {
        // Keepalive interval
        let mut ping_interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        ping_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                result = rx.recv() => {
                    match result {
                        Ok(event) => {
                            if !should_deliver_event_to_client(
                                &event,
                                &user_id_filter,
                                &tenant_id_filter,
                            ) {
                                continue;
                            }

                            // Check subscription filter
                            let subs = subs_clone.read().await;
                            if let Some(ref intent_ids) = *subs {
                                if let Some(ref event_intent_id) = event.intent_id {
                                    if !intent_ids.contains(event_intent_id) {
                                        continue;
                                    }
                                } else {
                                    // Event has no intent_id, skip if filtering by intent
                                    continue;
                                }
                            }
                            // If subs is None, user hasn't subscribed yet - skip
                            // (they need to send a subscribe message first)
                            if subs.is_none() {
                                continue;
                            }
                            drop(subs);

                            let msg = ServerMessage {
                                msg_type: "event".to_string(),
                                data: Some(serde_json::to_value(&event).unwrap_or_default()),
                                error: None,
                            };
                            if let Ok(text) = serde_json::to_string(&msg) {
                                if sender.send(Message::Text(text)).await.is_err() {
                                    break;
                                }
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            warn!(user_id = %user_id_filter, lagged = n, "WebSocket client lagged, skipped events");
                            let msg = ServerMessage {
                                msg_type: "warning".to_string(),
                                data: Some(serde_json::json!({
                                    "message": format!("Missed {} events due to slow processing", n)
                                })),
                                error: None,
                            };
                            if let Ok(text) = serde_json::to_string(&msg) {
                                let _ = sender.send(Message::Text(text)).await;
                            }
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            break;
                        }
                    }
                }
                _ = ping_interval.tick() => {
                    if sender.send(Message::Ping(vec![1, 2, 3, 4])).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    // Handle incoming messages from client
    let subs_clone = subscriptions.clone();
    let user_id_recv = user_id_str.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    match serde_json::from_str::<ClientMessage>(&text) {
                        Ok(ClientMessage::Subscribe { intent_ids }) => {
                            let mut subs = subs_clone.write().await;
                            if intent_ids.is_empty() {
                                // Subscribe to all user events
                                *subs = Some(HashSet::new());
                                debug!(user_id = %user_id_recv, "Subscribed to all events");
                            } else {
                                // Subscribe to specific intents
                                let set = subs.get_or_insert_with(HashSet::new);
                                for id in &intent_ids {
                                    set.insert(id.clone());
                                }
                                debug!(
                                    user_id = %user_id_recv,
                                    intent_ids = ?intent_ids,
                                    "Subscribed to specific intents"
                                );
                            }
                        }
                        Ok(ClientMessage::Unsubscribe { intent_ids }) => {
                            let mut subs = subs_clone.write().await;
                            if intent_ids.is_empty() {
                                *subs = None;
                                debug!(user_id = %user_id_recv, "Unsubscribed from all events");
                            } else if let Some(ref mut set) = *subs {
                                for id in &intent_ids {
                                    set.remove(id);
                                }
                                debug!(
                                    user_id = %user_id_recv,
                                    intent_ids = ?intent_ids,
                                    "Unsubscribed from specific intents"
                                );
                            }
                        }
                        Ok(ClientMessage::Ping) => {
                            // Client ping - no action needed, the pong is automatic
                            debug!(user_id = %user_id_recv, "Received client ping");
                        }
                        Err(e) => {
                            debug!(
                                user_id = %user_id_recv,
                                error = %e,
                                "Invalid WebSocket message"
                            );
                        }
                    }
                }
                Message::Pong(_) => {
                    // Client responded to our ping - connection is alive
                }
                Message::Close(_) => {
                    break;
                }
                _ => {}
            }
        }
    });

    // Wait for either task to complete (connection closed)
    tokio::select! {
        _ = &mut send_task => {
            recv_task.abort();
        }
        _ = &mut recv_task => {
            send_task.abort();
        }
    }

    info!(user_id = %user_id_str, "WebSocket connection closed");
}

// We need futures::StreamExt for receiver.next()
use futures::SinkExt as _;
use futures::StreamExt as _;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_state_creation() {
        let state = WsState::new("test-secret".to_string());
        // Should be able to publish without errors (no receivers)
        state.publish(WsEvent {
            event_type: "intent.updated".to_string(),
            intent_id: Some("test-intent-123".to_string()),
            user_id: "user-1".to_string(),
            tenant_id: "tenant-1".to_string(),
            data: serde_json::json!({"status": "PROCESSING"}),
            timestamp: "2026-02-08T00:00:00Z".to_string(),
        });
    }

    #[test]
    fn test_ws_event_serialization() {
        let event = WsEvent {
            event_type: "intent.completed".to_string(),
            intent_id: Some("abc-123".to_string()),
            user_id: "user-1".to_string(),
            tenant_id: "tenant-1".to_string(),
            data: serde_json::json!({
                "status": "COMPLETED",
                "amount": "1000000"
            }),
            timestamp: "2026-02-08T12:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&event).expect("serialization should work");
        assert!(json.contains("\"eventType\""));
        assert!(json.contains("\"intentId\""));
        assert!(json.contains("intent.completed"));
    }

    #[test]
    fn test_client_message_deserialization() {
        // Subscribe to all
        let msg: ClientMessage =
            serde_json::from_str(r#"{"action":"subscribe"}"#).expect("should parse");
        match msg {
            ClientMessage::Subscribe { intent_ids } => assert!(intent_ids.is_empty()),
            _ => panic!("expected Subscribe"),
        }

        // Subscribe to specific
        let msg: ClientMessage =
            serde_json::from_str(r#"{"action":"subscribe","intentIds":["abc","def"]}"#)
                .expect("should parse");
        match msg {
            ClientMessage::Subscribe { intent_ids } => {
                assert_eq!(intent_ids.len(), 2);
                assert_eq!(intent_ids[0], "abc");
            }
            _ => panic!("expected Subscribe"),
        }

        // Unsubscribe
        let msg: ClientMessage =
            serde_json::from_str(r#"{"action":"unsubscribe","intentIds":["abc"]}"#)
                .expect("should parse");
        match msg {
            ClientMessage::Unsubscribe { intent_ids } => {
                assert_eq!(intent_ids.len(), 1);
            }
            _ => panic!("expected Unsubscribe"),
        }

        // Ping
        let msg: ClientMessage =
            serde_json::from_str(r#"{"action":"ping"}"#).expect("should parse");
        matches!(msg, ClientMessage::Ping);
    }

    #[test]
    fn test_server_message_serialization() {
        let msg = ServerMessage {
            msg_type: "event".to_string(),
            data: Some(serde_json::json!({"key": "value"})),
            error: None,
        };
        let json = serde_json::to_string(&msg).expect("should serialize");
        assert!(json.contains("\"type\":\"event\""));
        assert!(!json.contains("\"error\""));

        let msg = ServerMessage {
            msg_type: "error".to_string(),
            data: None,
            error: Some("something went wrong".to_string()),
        };
        let json = serde_json::to_string(&msg).expect("should serialize");
        assert!(json.contains("\"error\":\"something went wrong\""));
        assert!(!json.contains("\"data\""));
    }

    #[test]
    fn test_ws_broadcast() {
        let state = WsState::new("secret".to_string());
        let mut rx = state.tx.subscribe();

        state.publish(WsEvent {
            event_type: "intent.updated".to_string(),
            intent_id: Some("intent-1".to_string()),
            user_id: "user-1".to_string(),
            tenant_id: "tenant-1".to_string(),
            data: serde_json::json!({"status": "PROCESSING"}),
            timestamp: "2026-02-08T00:00:00Z".to_string(),
        });

        let event = rx.try_recv().expect("should receive event");
        assert_eq!(event.event_type, "intent.updated");
        assert_eq!(event.intent_id.unwrap(), "intent-1");
    }

    #[test]
    fn test_ws_query_deserialization() {
        let query: WsQuery = serde_json::from_str(r#"{"token":"eyJ..."}"#).expect("should parse");
        assert_eq!(query.token, "eyJ...");
    }

    #[test]
    fn test_publish_incident_timeline_update_uses_existing_ws_event_shape() {
        let state = WsState::new("secret".to_string());
        let mut rx = state.tx.subscribe();

        state.publish_incident_timeline_update(
            "tenant-incident",
            "system",
            "incident_intent_001",
            Some("intent_001"),
            serde_json::json!({
                "entryCount": 3,
                "recommendationCount": 1,
            }),
        );

        let event = rx.try_recv().expect("should receive incident update");
        assert_eq!(event.event_type, "incident.timeline.updated");
        assert_eq!(event.intent_id.as_deref(), Some("intent_001"));
        assert_eq!(event.tenant_id, "tenant-incident");
        assert_eq!(event.data["incidentId"], "incident_intent_001");
        assert_eq!(event.data["timeline"]["entryCount"], 3);
    }

    #[test]
    fn test_incident_timeline_updates_are_not_delivered_by_tenant_match_alone() {
        let event = WsEvent {
            event_type: "incident.timeline.updated".to_string(),
            intent_id: Some("intent-incident".to_string()),
            user_id: "system".to_string(),
            tenant_id: "tenant-incident".to_string(),
            data: serde_json::json!({"incidentId": "incident_intent_001"}),
            timestamp: "2026-02-08T00:00:00Z".to_string(),
        };

        assert!(
            !should_deliver_event_to_client(&event, "user-1", "tenant-incident"),
            "incident timeline updates must not fan out to same-tenant portal clients"
        );
    }

    #[test]
    fn test_non_incident_events_still_allow_tenant_scoped_delivery() {
        let event = WsEvent {
            event_type: "intent.updated".to_string(),
            intent_id: Some("intent-1".to_string()),
            user_id: "system".to_string(),
            tenant_id: "tenant-incident".to_string(),
            data: serde_json::json!({"status": "PROCESSING"}),
            timestamp: "2026-02-08T00:00:00Z".to_string(),
        };

        assert!(should_deliver_event_to_client(
            &event,
            "user-1",
            "tenant-incident"
        ));
    }
}
