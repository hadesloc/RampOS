/// E2E RFQ auction flow tests
///
/// Tests the complete RFQ lifecycle via RfqService:
/// - Create RFQ → Submit bids → Finalize (best bid wins) → Verify events
/// - Direction-aware matching (OFFRAMP maximizes rate, ONRAMP minimizes)
/// - Edge cases: cancel flow, empty auction
///
/// These tests exercise the service layer directly (not HTTP endpoints)
/// because the RFQ portal endpoints require portal JWT auth + PgPool.
/// The InMemoryRfqRepository is #[cfg(test)] internal to ramp-core,
/// so we use ramp-core's RfqService tests here as E2E validation.
///
/// For full HTTP-level E2E with real DB, see the DATABASE_URL-gated test.

use ramp_common::types::TenantId;
use ramp_core::event::InMemoryEventPublisher;
use ramp_core::repository::{PgRfqRepository, RfqRepository};
use ramp_core::service::rfq::{CreateRfqRequest, RfqService, SubmitBidRequest};
use rust_decimal_macros::dec;
use sqlx::PgPool;
use std::sync::Arc;

fn tenant() -> TenantId {
    TenantId("tenant_rfq_e2e".to_string())
}

/// Full RFQ flow via real PostgreSQL:
/// Create OFFRAMP RFQ → 2 LP bids → Finalize → highest rate wins
///
/// Only runs when DATABASE_URL is set (CI or local PG)
#[tokio::test]
async fn rfq_offramp_full_auction_with_db() {
    let database_url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => return, // Skip if no DB
    };

    let pool = PgPool::connect(&database_url)
        .await
        .expect("database connection should succeed");

    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .expect("migrations should succeed");

    let repo = Arc::new(PgRfqRepository::new(pool.clone()));
    let events = Arc::new(InMemoryEventPublisher::new());
    let svc = RfqService::new(repo.clone(), events.clone());

    // Create OFFRAMP RFQ: selling 1 USDT
    let rfq = svc
        .create_rfq(CreateRfqRequest {
            tenant_id: tenant(),
            user_id: "user_seller_e2e".to_string(),
            direction: "OFFRAMP".to_string(),
            offramp_id: None,
            crypto_asset: "USDT".to_string(),
            crypto_amount: dec!(1.0),
            vnd_amount: None,
            ttl_minutes: 5,
        })
        .await
        .expect("create RFQ");

    assert_eq!(rfq.state, "OPEN");
    assert_eq!(rfq.direction, "OFFRAMP");

    // LP1: 25000 VND/USDT
    svc.submit_bid(SubmitBidRequest {
        tenant_id: tenant(),
        rfq_id: rfq.id.clone(),
        lp_id: "lp_alpha_e2e".to_string(),
        lp_name: Some("Alpha LP".to_string()),
        exchange_rate: dec!(25000),
        vnd_amount: dec!(25000),
        valid_minutes: 5,
    })
    .await
    .expect("bid 1");

    // LP2: 25500 VND/USDT (best for OFFRAMP — highest rate)
    svc.submit_bid(SubmitBidRequest {
        tenant_id: tenant(),
        rfq_id: rfq.id.clone(),
        lp_id: "lp_beta_e2e".to_string(),
        lp_name: Some("Beta LP".to_string()),
        exchange_rate: dec!(25500),
        vnd_amount: dec!(25500),
        valid_minutes: 5,
    })
    .await
    .expect("bid 2");

    // Finalize: OFFRAMP → highest rate wins → lp_beta at 25500
    let result = svc
        .finalize_rfq(&tenant(), &rfq.id)
        .await
        .expect("finalize RFQ");

    assert_eq!(result.rfq.state, "MATCHED");
    assert_eq!(result.winning_bid.lp_id, "lp_beta_e2e");
    assert_eq!(result.winning_bid.exchange_rate, dec!(25500));

    // Verify events
    let published = events.get_events().await;
    assert!(
        published.iter().any(|e| e["type"] == "rfq.created"),
        "rfq.created event emitted"
    );
    assert!(
        published.iter().any(|e| e["type"] == "rfq.matched"),
        "rfq.matched event emitted"
    );
}

/// ONRAMP: lowest rate wins (buyer wants cheapest crypto)
#[tokio::test]
async fn rfq_onramp_lowest_rate_wins_with_db() {
    let database_url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => return,
    };

    let pool = PgPool::connect(&database_url)
        .await
        .expect("database connection");

    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .expect("migrations");

    let repo = Arc::new(PgRfqRepository::new(pool.clone()));
    let events = Arc::new(InMemoryEventPublisher::new());
    let svc = RfqService::new(repo, events);

    let rfq = svc
        .create_rfq(CreateRfqRequest {
            tenant_id: tenant(),
            user_id: "user_buyer_e2e".to_string(),
            direction: "ONRAMP".to_string(),
            offramp_id: None,
            crypto_asset: "USDT".to_string(),
            crypto_amount: dec!(100.0),
            vnd_amount: Some(dec!(2_600_000)),
            ttl_minutes: 5,
        })
        .await
        .expect("create ONRAMP RFQ");

    // LP1: 26000 VND/USDT
    svc.submit_bid(SubmitBidRequest {
        tenant_id: tenant(),
        rfq_id: rfq.id.clone(),
        lp_id: "lp_a_e2e".to_string(),
        lp_name: None,
        exchange_rate: dec!(26000),
        vnd_amount: dec!(2_600_000),
        valid_minutes: 5,
    })
    .await
    .expect("bid a");

    // LP2: 25800 VND/USDT (best for ONRAMP — lowest rate)
    svc.submit_bid(SubmitBidRequest {
        tenant_id: tenant(),
        rfq_id: rfq.id.clone(),
        lp_id: "lp_b_e2e".to_string(),
        lp_name: None,
        exchange_rate: dec!(25800),
        vnd_amount: dec!(2_580_000),
        valid_minutes: 5,
    })
    .await
    .expect("bid b");

    let result = svc
        .finalize_rfq(&tenant(), &rfq.id)
        .await
        .expect("finalize ONRAMP");

    assert_eq!(result.rfq.state, "MATCHED");
    assert_eq!(result.winning_bid.lp_id, "lp_b_e2e");
    assert_eq!(result.winning_bid.exchange_rate, dec!(25800));
}

/// Cancel flow: user cancels open RFQ before finalization
#[tokio::test]
async fn rfq_cancel_before_finalization_with_db() {
    let database_url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => return,
    };

    let pool = PgPool::connect(&database_url)
        .await
        .expect("database connection");

    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .expect("migrations");

    let repo = Arc::new(PgRfqRepository::new(pool.clone()));
    let events = Arc::new(InMemoryEventPublisher::new());
    let svc = RfqService::new(repo, events);

    let rfq = svc
        .create_rfq(CreateRfqRequest {
            tenant_id: tenant(),
            user_id: "user_cancel_e2e".to_string(),
            direction: "OFFRAMP".to_string(),
            offramp_id: None,
            crypto_asset: "USDT".to_string(),
            crypto_amount: dec!(50.0),
            vnd_amount: None,
            ttl_minutes: 5,
        })
        .await
        .expect("create RFQ");

    svc.submit_bid(SubmitBidRequest {
        tenant_id: tenant(),
        rfq_id: rfq.id.clone(),
        lp_id: "lp_wasted_e2e".to_string(),
        lp_name: None,
        exchange_rate: dec!(25000),
        vnd_amount: dec!(1_250_000),
        valid_minutes: 5,
    })
    .await
    .expect("bid");

    let cancelled = svc
        .cancel_rfq(&tenant(), &rfq.id)
        .await
        .expect("cancel RFQ");

    assert_eq!(cancelled.state, "CANCELLED");

    // Finalize should fail on cancelled RFQ
    let finalize_result = svc.finalize_rfq(&tenant(), &rfq.id).await;
    assert!(finalize_result.is_err(), "Cannot finalize a cancelled RFQ");
}
