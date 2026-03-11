//! GraphQL Mutation resolvers

use async_graphql::{Context, Object, Result as GqlResult};
use ramp_common::types::*;
use rust_decimal::Decimal;
use std::str::FromStr;
use std::sync::Arc;

use ramp_core::service::payin::{ConfirmPayinRequest, CreatePayinRequest, PayinService};
use ramp_core::service::payout::{CreatePayoutRequest, PayoutService};

use super::require_scoped_tenant;
use super::types::{
    ConfirmPayInInput, ConfirmPayInResult, CreatePayInInput, CreatePayInResult, CreatePayoutInput,
    CreatePayoutResult,
};

/// Root mutation object for the GraphQL API
pub struct MutationRoot;

#[Object]
impl MutationRoot {
    /// Create a new pay-in intent
    async fn create_pay_in(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Tenant ID for multi-tenant isolation")] tenant_id: String,
        input: CreatePayInInput,
    ) -> GqlResult<CreatePayInResult> {
        let payin_service = ctx.data::<Arc<PayinService>>()?;
        let tenant_id = require_scoped_tenant(ctx, &tenant_id)?;

        let amount = Decimal::from_str(&input.amount_vnd)
            .map_err(|_| async_graphql::Error::new("Invalid amount format"))?;

        let req = CreatePayinRequest {
            tenant_id,
            user_id: UserId(input.user_id),
            amount_vnd: VndAmount(amount),
            rails_provider: RailsProvider(input.rails_provider),
            idempotency_key: input.idempotency_key.map(IdempotencyKey),
            metadata: input.metadata.unwrap_or(serde_json::json!({})),
        };

        let resp = payin_service
            .create_payin(req)
            .await
            .map_err(|e| async_graphql::Error::new(format!("Failed to create pay-in: {}", e)))?;

        Ok(CreatePayInResult {
            intent_id: resp.intent_id.0,
            reference_code: resp.reference_code.0,
            status: resp.status.to_string(),
            expires_at: resp.expires_at.0,
            daily_limit: resp.daily_limit.to_string(),
            daily_remaining: resp.daily_remaining.to_string(),
        })
    }

    /// Confirm a pay-in from bank webhook data
    async fn confirm_pay_in(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Tenant ID for multi-tenant isolation")] tenant_id: String,
        input: ConfirmPayInInput,
    ) -> GqlResult<ConfirmPayInResult> {
        let payin_service = ctx.data::<Arc<PayinService>>()?;
        let tenant_id = require_scoped_tenant(ctx, &tenant_id)?;

        let amount = Decimal::from_str(&input.amount_vnd)
            .map_err(|_| async_graphql::Error::new("Invalid amount format"))?;

        let req = ConfirmPayinRequest {
            tenant_id,
            reference_code: ReferenceCode(input.reference_code),
            bank_tx_id: input.bank_tx_id,
            amount_vnd: VndAmount(amount),
            settled_at: Timestamp::now(),
            raw_payload_hash: input.raw_payload_hash,
        };

        let intent_id = payin_service
            .confirm_payin(req)
            .await
            .map_err(|e| async_graphql::Error::new(format!("Failed to confirm pay-in: {}", e)))?;

        Ok(ConfirmPayInResult {
            intent_id: intent_id.0,
            success: true,
        })
    }

    /// Create a new pay-out intent
    async fn create_payout(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Tenant ID for multi-tenant isolation")] tenant_id: String,
        input: CreatePayoutInput,
    ) -> GqlResult<CreatePayoutResult> {
        let payout_service = ctx.data::<Arc<PayoutService>>()?;
        let tenant_id = require_scoped_tenant(ctx, &tenant_id)?;

        let amount = Decimal::from_str(&input.amount_vnd)
            .map_err(|_| async_graphql::Error::new("Invalid amount format"))?;

        let req = CreatePayoutRequest {
            tenant_id,
            user_id: UserId(input.user_id),
            amount_vnd: VndAmount(amount),
            rails_provider: RailsProvider(input.rails_provider),
            bank_account: BankAccount {
                bank_code: input.bank_code,
                account_number: input.account_number,
                account_name: input.account_name,
            },
            idempotency_key: input.idempotency_key.map(IdempotencyKey),
            metadata: input.metadata.unwrap_or(serde_json::json!({})),
        };

        let resp = payout_service
            .create_payout(req)
            .await
            .map_err(|e| async_graphql::Error::new(format!("Failed to create payout: {}", e)))?;

        Ok(CreatePayoutResult {
            intent_id: resp.intent_id.0,
            status: resp.status.to_string(),
            daily_limit: resp.daily_limit.to_string(),
            daily_remaining: resp.daily_remaining.to_string(),
        })
    }
}
