//! GraphQL API module for RampOS
//!
//! Provides a complete GraphQL API with:
//! - Query resolvers for intents, users, ledger entries, and dashboard stats
//! - Mutation resolvers for pay-in, pay-out operations
//! - Subscription resolvers for real-time intent status updates
//! - Cursor-based pagination (Relay Connection pattern)
//! - DataLoader for N+1 prevention
//!
//! # Usage
//!
//! ```rust,ignore
//! use ramp_api::graphql;
//!
//! let router = graphql::graphql_router(app_state);
//! // Mount at /graphql in your main router
//! ```

pub mod loaders;
pub mod mutation;
pub mod pagination;
pub mod query;
pub mod subscription;
pub mod types;

#[cfg(test)]
mod tests;

use async_graphql::http::GraphiQLSource;
use async_graphql::Schema;
use axum::{
    extract::{Request, State},
    middleware,
    response::{Html, IntoResponse},
    routing::get,
    Json, Router,
};
use std::sync::Arc;

use crate::middleware::auth_middleware;
use crate::middleware::tenant::TenantContext;
use crate::router::AppState;
use mutation::MutationRoot;
use query::QueryRoot;
use ramp_core::repository::tenant::TenantRepository;
use subscription::{create_intent_event_channel, SubscriptionRoot};

/// The complete GraphQL schema type
pub type RampOsSchema = Schema<QueryRoot, MutationRoot, SubscriptionRoot>;

/// Build the GraphQL schema with all resolvers and context data from AppState.
pub fn build_schema(state: &AppState) -> RampOsSchema {
    let intent_event_sender = Arc::new(create_intent_event_channel());

    Schema::build(QueryRoot, MutationRoot, SubscriptionRoot)
        .data(state.intent_repo.clone())
        .data(state.payin_service.clone())
        .data(state.payout_service.clone())
        .data(state.user_service.clone())
        .data(intent_event_sender)
        .finish()
}

/// Handler for GraphQL requests (POST /graphql)
///
/// Extracts the authenticated TenantContext from request extensions
/// (set by auth_middleware) and injects it into the GraphQL request data
/// so resolvers can access the tenant information.
async fn graphql_handler(
    State(schema): State<RampOsSchema>,
    req: Request,
) -> Json<async_graphql::Response> {
    let tenant_ctx = req.extensions().get::<TenantContext>().cloned();
    let body_bytes = match axum::body::to_bytes(req.into_body(), usize::MAX).await {
        Ok(bytes) => bytes,
        Err(_) => {
            let resp = async_graphql::Response::from_errors(vec![
                async_graphql::ServerError::new("Failed to read request body", None),
            ]);
            return Json(resp);
        }
    };
    let gql_request: async_graphql::Request = match serde_json::from_slice(&body_bytes) {
        Ok(r) => r,
        Err(_) => {
            let resp = async_graphql::Response::from_errors(vec![
                async_graphql::ServerError::new("Invalid GraphQL request JSON", None),
            ]);
            return Json(resp);
        }
    };
    let gql_request = if let Some(ctx) = tenant_ctx {
        gql_request.data(ctx)
    } else {
        gql_request
    };
    Json(schema.execute(gql_request).await)
}

/// Handler for GraphQL Playground (GET /graphql)
async fn graphql_playground() -> impl IntoResponse {
    Html(
        GraphiQLSource::build()
            .endpoint("/graphql")
            .subscription_endpoint("/graphql/ws")
            .finish(),
    )
}

/// Create a standalone GraphQL router that can be mounted on the main app.
///
/// This router provides:
/// - `POST /` - GraphQL query/mutation endpoint (auth required)
/// - `GET /` - GraphiQL Playground UI (no auth)
/// - `GET /playground` - GraphiQL Playground UI alias (no auth)
///
/// Authentication is applied to POST requests via the same auth_middleware
/// used by REST API routes, ensuring consistent API key + HMAC validation.
///
/// # Example
///
/// ```rust,ignore
/// let gql_router = graphql::graphql_router(app_state);
/// let app = Router::new().nest("/graphql", gql_router);
/// ```
pub fn graphql_router(state: AppState) -> Router {
    let schema = build_schema(&state);
    graphql_router_with_auth(schema, state.tenant_repo.clone())
}

/// Create a GraphQL router with auth middleware applied to POST routes.
/// Playground (GET) routes remain unauthenticated.
pub fn graphql_router_with_auth(
    schema: RampOsSchema,
    tenant_repo: Arc<dyn TenantRepository>,
) -> Router {
    // POST routes require authentication
    let post_routes = Router::new()
        .route("/", axum::routing::post(graphql_handler))
        .layer(middleware::from_fn_with_state(
            tenant_repo,
            auth_middleware,
        ))
        .with_state(schema);

    // GET routes (playground) do not require authentication
    let get_routes = Router::new()
        .route("/", get(graphql_playground))
        .route("/playground", get(graphql_playground));

    Router::new().merge(post_routes).merge(get_routes)
}

/// Create a GraphQL router from an already-built schema (no auth).
/// Useful for testing or when you need custom schema configuration.
pub fn graphql_router_from_schema(schema: RampOsSchema) -> Router {
    Router::new()
        .route("/", get(graphql_playground).post(graphql_handler))
        .route("/playground", get(graphql_playground))
        .with_state(schema)
}
