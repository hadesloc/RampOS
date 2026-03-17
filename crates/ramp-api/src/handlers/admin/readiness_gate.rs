//! Production Readiness Gate
//!
//! Replaces the misleading "98% complete" claim with a machine-readable
//! readiness assessment endpoint.

use axum::{extract::State, http::HeaderMap, Json};
use serde::Serialize;
use tracing::info;

use crate::error::ApiError;

// ============================================================================
// DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadinessReport {
    pub overall: ReadinessStatus,
    pub gates: Vec<GateResult>,
    pub summary: String,
    pub checked_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReadinessStatus {
    Ready,
    NotReady,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GateResult {
    pub name: String,
    pub status: GateStatus,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GateStatus {
    Pass,
    Warn,
    Fail,
}

// ============================================================================
// Handler
// ============================================================================

/// GET /v1/admin/readiness — Production readiness gate
///
/// Returns a structured report of all readiness gates.
/// Overall status is READY only if all gates are PASS or WARN.
pub async fn get_readiness(
    headers: HeaderMap,
    State(app_state): State<crate::router::AppState>,
) -> Result<Json<ReadinessReport>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!("Checking production readiness gates");

    let mut gates = Vec::new();

    // Gate 1: Admin authentication
    let admin_auth_gate = if std::env::var("RAMPOS_ADMIN_JWT_SECRET").is_ok() {
        GateResult {
            name: "admin_auth".to_string(),
            status: GateStatus::Pass,
            detail: "JWT-based admin auth configured".to_string(),
        }
    } else if std::env::var("RAMPOS_ADMIN_KEY").is_ok() {
        GateResult {
            name: "admin_auth".to_string(),
            status: GateStatus::Warn,
            detail: "Using legacy RAMPOS_ADMIN_KEY — migrate to JWT admin sessions".to_string(),
        }
    } else {
        GateResult {
            name: "admin_auth".to_string(),
            status: GateStatus::Fail,
            detail: "No admin authentication configured".to_string(),
        }
    };
    gates.push(admin_auth_gate);

    // Gate 2: Secrets management
    let secrets_gate = match std::env::var("SECRET_PROVIDER").as_deref() {
        Ok("vault") | Ok("aws-sm") => GateResult {
            name: "secrets_management".to_string(),
            status: GateStatus::Pass,
            detail: "Production secrets provider active".to_string(),
        },
        _ => GateResult {
            name: "secrets_management".to_string(),
            status: GateStatus::Warn,
            detail: "Using EnvSecretProvider — acceptable for Phase 1, migrate to Vault for Phase 2".to_string(),
        },
    };
    gates.push(secrets_gate);

    // Gate 3: Database connectivity
    let db_gate = if let Some(ref pool) = app_state.db_pool {
        match sqlx::query("SELECT 1").execute(pool).await {
            Ok(_) => GateResult {
                name: "database".to_string(),
                status: GateStatus::Pass,
                detail: "PostgreSQL connected and responsive".to_string(),
            },
            Err(e) => GateResult {
                name: "database".to_string(),
                status: GateStatus::Fail,
                detail: format!("Database health check failed: {}", e),
            },
        }
    } else {
        GateResult {
            name: "database".to_string(),
            status: GateStatus::Fail,
            detail: "No database pool configured".to_string(),
        }
    };
    gates.push(db_gate);

    // Gate 4: Event publisher
    let event_gate = GateResult {
        name: "event_publisher".to_string(),
        status: if std::env::var("NATS_URL").is_ok() {
            GateStatus::Pass
        } else {
            GateStatus::Warn
        },
        detail: if std::env::var("NATS_URL").is_ok() {
            "NATS event publisher configured".to_string()
        } else {
            "Using in-memory event publisher — not suitable for multi-instance deployment".to_string()
        },
    };
    gates.push(event_gate);

    // Gate 5: KYC/KYT providers
    let kyc_gate = GateResult {
        name: "kyc_provider".to_string(),
        status: if app_state.kyc_service.is_some() {
            GateStatus::Pass
        } else {
            GateStatus::Warn
        },
        detail: if app_state.kyc_service.is_some() {
            "KYC service active".to_string()
        } else {
            "KYC service not configured".to_string()
        },
    };
    gates.push(kyc_gate);

    // Gate 6: Penetration testing
    gates.push(GateResult {
        name: "pentest".to_string(),
        status: GateStatus::Fail,
        detail: "No penetration test completed — required before production launch".to_string(),
    });

    // Gate 7: Load testing
    gates.push(GateResult {
        name: "load_test".to_string(),
        status: GateStatus::Fail,
        detail: "No load test completed — required before production launch".to_string(),
    });

    // Determine overall status
    let has_failures = gates.iter().any(|g| g.status == GateStatus::Fail);
    let overall = if has_failures {
        ReadinessStatus::NotReady
    } else {
        ReadinessStatus::Ready
    };

    let pass_count = gates.iter().filter(|g| g.status == GateStatus::Pass).count();
    let warn_count = gates.iter().filter(|g| g.status == GateStatus::Warn).count();
    let fail_count = gates.iter().filter(|g| g.status == GateStatus::Fail).count();

    let report = ReadinessReport {
        overall,
        summary: format!(
            "{}/{} gates passing, {} warnings, {} failures",
            pass_count, gates.len(), warn_count, fail_count
        ),
        checked_at: chrono::Utc::now().to_rfc3339(),
        gates,
    };

    Ok(Json(report))
}
