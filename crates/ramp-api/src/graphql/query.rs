//! GraphQL Query resolvers

use async_graphql::{Context, Object, Result as GqlResult, ID};
use ramp_common::types::{IntentId, UserId};
use ramp_core::service::user::UserService;
use std::sync::Arc;

use ramp_core::repository::intent::IntentRepository;

use super::require_scoped_tenant;
use super::pagination::{self, IntentConnection, UserConnection};
use super::types::{DashboardStatsType, IntentFilter, IntentType, UserType};

/// Root query object for the GraphQL API
pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Get a single intent by ID
    async fn intent(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Tenant ID for multi-tenant isolation")] tenant_id: String,
        #[graphql(desc = "Intent ID")] id: ID,
    ) -> GqlResult<Option<IntentType>> {
        let intent_repo = ctx.data::<Arc<dyn IntentRepository>>()?;
        let tid = require_scoped_tenant(ctx, &tenant_id)?;
        let iid = IntentId(id.to_string());

        let intent = intent_repo
            .get_by_id(&tid, &iid)
            .await
            .map_err(|e| async_graphql::Error::new(format!("Failed to fetch intent: {}", e)))?;

        Ok(intent.map(IntentType))
    }

    /// List intents with optional filtering and cursor-based pagination
    async fn intents(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Tenant ID for multi-tenant isolation")] tenant_id: String,
        #[graphql(desc = "Filter criteria")] filter: Option<IntentFilter>,
        #[graphql(desc = "Number of items to return (max 100)", default = 20)] first: Option<i32>,
        #[graphql(desc = "Cursor to start after")] after: Option<String>,
    ) -> GqlResult<IntentConnection> {
        let intent_repo = ctx.data::<Arc<dyn IntentRepository>>()?;
        let tid = require_scoped_tenant(ctx, &tenant_id)?;

        let offset = after
            .as_ref()
            .and_then(|c| pagination::decode_cursor(c))
            .map(|o| o + 1)
            .unwrap_or(0);

        let limit = first.unwrap_or(20).max(1).min(100) as i64;

        // Determine user_id filter
        let filter = filter.unwrap_or_default();
        let user_id = filter.user_id.as_deref().unwrap_or("*");

        // Use list_by_user if user_id is specified
        let items = if user_id != "*" {
            let uid = UserId(user_id.to_string());
            intent_repo
                .list_by_user(&tid, &uid, limit + 1, offset as i64)
                .await
                .map_err(|e| async_graphql::Error::new(format!("Failed to list intents: {}", e)))?
        } else {
            // Without a user_id, return empty since the repo requires user scoping
            Vec::new()
        };

        // Apply additional in-memory filters
        let items: Vec<IntentType> = items
            .into_iter()
            .filter(|i| {
                if let Some(ref it) = filter.intent_type {
                    if i.intent_type != *it {
                        return false;
                    }
                }
                if let Some(ref state) = filter.state {
                    if i.state != *state {
                        return false;
                    }
                }
                true
            })
            .map(IntentType)
            .collect();

        Ok(pagination::build_intent_connection(
            items, first, after, None,
        ))
    }

    /// Get a single user by ID
    async fn user(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Tenant ID for multi-tenant isolation")] tenant_id: String,
        #[graphql(desc = "User ID")] id: ID,
    ) -> GqlResult<Option<UserType>> {
        let user_service = ctx.data::<Arc<UserService>>()?;
        let tid = require_scoped_tenant(ctx, &tenant_id)?;
        let uid = UserId(id.to_string());

        match user_service.get_user(&tid, &uid).await {
            Ok(user) => Ok(Some(UserType(user))),
            Err(_) => Ok(None),
        }
    }

    /// List users with cursor-based pagination
    async fn users(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Tenant ID for multi-tenant isolation")] tenant_id: String,
        #[graphql(desc = "Number of items to return (max 100)", default = 20)] first: Option<i32>,
        #[graphql(desc = "Cursor to start after")] after: Option<String>,
    ) -> GqlResult<UserConnection> {
        let user_service = ctx.data::<Arc<UserService>>()?;
        let tid = require_scoped_tenant(ctx, &tenant_id)?;

        let offset = after
            .as_ref()
            .and_then(|c| pagination::decode_cursor(c))
            .map(|o| o + 1)
            .unwrap_or(0);

        let limit = first.unwrap_or(20).max(1).min(100) as i64;

        let (users, total) = user_service
            .list_users(&tid, limit + 1, offset as i64, None, None, None)
            .await
            .map_err(|e| async_graphql::Error::new(format!("Failed to list users: {}", e)))?;

        let items: Vec<UserType> = users.into_iter().map(UserType).collect();

        Ok(pagination::build_user_connection(
            items,
            first,
            after,
            Some(total),
        ))
    }

    /// Get dashboard summary statistics
    async fn dashboard_stats(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Tenant ID for multi-tenant isolation")] tenant_id: String,
    ) -> GqlResult<DashboardStatsType> {
        let user_service = ctx.data::<Arc<UserService>>()?;
        let tid = require_scoped_tenant(ctx, &tenant_id)?;

        let (_, total_users) = user_service
            .list_users(&tid, 0, 0, None, None, None)
            .await
            .unwrap_or((vec![], 0));

        let active_users = user_service
            .count_users_by_status(&tid, "ACTIVE")
            .await
            .unwrap_or(0);

        Ok(DashboardStatsType {
            total_users,
            active_users,
            total_intents_today: 0,
            total_payin_volume_today: "0".to_string(),
            total_payout_volume_today: "0".to_string(),
            pending_intents: 0,
        })
    }
}
