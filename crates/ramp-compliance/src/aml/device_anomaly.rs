use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ramp_common::types::UserId;
use ramp_common::Result;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

use crate::rules::{AmlRule, RuleContext, RuleResult};
use crate::types::{CaseSeverity, CaseType, RiskScore};

// ============================================================================
// Data Structures
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceContext {
    pub device_id: String,
    pub device_fingerprint: String,
    pub ip_address: String,
    pub country: Option<String>,
    pub city: Option<String>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub user_agent: String,
    #[serde(default)]
    pub is_vpn: bool,
    #[serde(default)]
    pub is_proxy: bool,
    #[serde(default = "Utc::now")]
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeviceHistory {
    pub last_devices: Vec<DeviceRecord>,
    pub last_ips: Vec<IpRecord>,
    pub last_locations: Vec<LocationRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRecord {
    pub device_id: String,
    pub device_fingerprint: String,
    pub last_seen: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpRecord {
    pub ip_address: String,
    pub last_seen: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationRecord {
    pub country: String,
    pub city: Option<String>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub last_seen: DateTime<Utc>,
}

// ============================================================================
// Storage Trait
// ============================================================================

#[async_trait]
pub trait DeviceHistoryStore: Send + Sync {
    async fn get_history(&self, user_id: &UserId) -> Result<DeviceHistory>;
    async fn update_history(&self, user_id: &UserId, context: &DeviceContext) -> Result<()>;
    async fn get_users_on_device(&self, device_fingerprint: &str) -> Result<Vec<UserId>>;
}

// ============================================================================
// Rule Implementation
// ============================================================================

pub struct DeviceAnomalyRule {
    store: Arc<dyn DeviceHistoryStore>,
    #[allow(dead_code)]
    max_history_items: usize,
    max_devices_per_user: u32,
    max_ips_per_day: u32,
    suspicious_countries: Vec<String>,
}

impl DeviceAnomalyRule {
    pub fn new(store: Arc<dyn DeviceHistoryStore>) -> Self {
        Self {
            store,
            max_history_items: 10,
            max_devices_per_user: 5,
            max_ips_per_day: 5,
            suspicious_countries: vec![
                "NK".to_string(), // North Korea
                "IR".to_string(), // Iran
                "SY".to_string(), // Syria
                "CU".to_string(), // Cuba
            ],
        }
    }

    pub fn with_config(
        store: Arc<dyn DeviceHistoryStore>,
        max_devices_per_user: u32,
        max_ips_per_day: u32,
        suspicious_countries: Vec<String>,
    ) -> Self {
        Self {
            store,
            max_history_items: 10,
            max_devices_per_user,
            max_ips_per_day,
            suspicious_countries,
        }
    }

    fn calculate_distance_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
        const R: f64 = 6371.0; // Earth radius in km
        let d_lat = (lat2 - lat1).to_radians();
        let d_lon = (lon2 - lon1).to_radians();
        let a = (d_lat / 2.0).sin().powi(2)
            + lat1.to_radians().cos() * lat2.to_radians().cos() * (d_lon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
        R * c
    }
}

#[async_trait]
impl AmlRule for DeviceAnomalyRule {
    fn name(&self) -> &str {
        "device_anomaly"
    }

    fn case_type(&self) -> CaseType {
        CaseType::DeviceAnomaly
    }

    async fn evaluate(&self, ctx: &RuleContext) -> Result<RuleResult> {
        // 1. Extract device context from metadata
        let device_ctx: DeviceContext = match serde_json::from_value(
            ctx.metadata
                .get("device")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
        ) {
            Ok(d) => d,
            Err(_) => {
                // FAIL-SAFE: If device info is missing, REJECT or REQUIRE REVIEW
                // Do NOT pass silently.
                return Ok(RuleResult {
                    passed: false,
                    reason: "Missing device information".to_string(),
                    risk_score: Some(RiskScore::new(80.0)),
                    severity: Some(CaseSeverity::High),
                    create_case: true,
                });
            }
        };

        let mut risk_score = 0.0;
        let mut reasons = Vec::new();
        let mut create_case = false;

        // 2. Get history
        let history = self.store.get_history(&ctx.user_id).await?;

        // 3. Run Checks

        // A. Suspicious Country
        if let Some(country) = &device_ctx.country {
            if self.suspicious_countries.contains(country) {
                risk_score += 80.0; // Increased to be high risk (>= 70.0)
                reasons.push(format!("Suspicious country detected: {}", country));
                create_case = true;
            }
        }

        // B. New device for user -> low risk bump
        let known_device = history
            .last_devices
            .iter()
            .any(|d| d.device_fingerprint == device_ctx.device_fingerprint);
        if !known_device && !history.last_devices.is_empty() {
            risk_score += 10.0;
            reasons.push(format!("New device detected: {}", device_ctx.device_id));
        }

        // C. Max Devices Check
        let mut unique_devices: std::collections::HashSet<String> = history
            .last_devices
            .iter()
            .map(|d| d.device_fingerprint.clone())
            .collect();
        // Add current check
        unique_devices.insert(device_ctx.device_fingerprint.clone());

        if unique_devices.len() as u32 > self.max_devices_per_user {
            risk_score += 20.0;
            reasons.push(format!(
                "Too many unique devices used: {}",
                unique_devices.len()
            ));
        }

        // D. Max IPs Per Day Check
        let one_day_ago = device_ctx.timestamp - chrono::Duration::days(1);
        let mut unique_ips_24h: std::collections::HashSet<String> = history
            .last_ips
            .iter()
            .filter(|i| i.last_seen > one_day_ago)
            .map(|i| i.ip_address.clone())
            .collect();
        unique_ips_24h.insert(device_ctx.ip_address.clone());

        if unique_ips_24h.len() as u32 > self.max_ips_per_day {
            risk_score += 20.0;
            reasons.push(format!(
                "Too many unique IPs used in 24h: {}",
                unique_ips_24h.len()
            ));
        }

        // E. Known VPN/proxy IP -> medium risk
        if device_ctx.is_vpn || device_ctx.is_proxy {
            risk_score += 30.0;
            reasons.push("Transaction from known VPN/Proxy".to_string());
        }

        // F. Different country in short time / Impossible travel
        if let Some(current_country) = &device_ctx.country {
            if let Some(last_loc) = history.last_locations.first() {
                if &last_loc.country != current_country {
                    // Country change
                    risk_score += 50.0;
                    reasons.push(format!(
                        "Location changed from {} to {}",
                        last_loc.country, current_country
                    ));
                    create_case = true; // Significant change

                    // Impossible travel check
                    if let (Some(lat1), Some(lon1), Some(lat2), Some(lon2)) =
                        (last_loc.lat, last_loc.lon, device_ctx.lat, device_ctx.lon)
                    {
                        let distance = Self::calculate_distance_km(lat1, lon1, lat2, lon2);
                        let time_diff = device_ctx
                            .timestamp
                            .signed_duration_since(last_loc.last_seen);
                        let hours = time_diff.num_minutes().abs() as f64 / 60.0;

                        // Simple check: > 800km/h implies impossible travel (plane speed approx)
                        // Allow some buffer, say 1000km/h
                        if hours > 0.1 {
                            let speed = distance / hours;
                            if speed > 1000.0 {
                                risk_score += 40.0; // Cumulative with country change
                                reasons.push(format!(
                                    "Impossible travel: {:.0}km in {:.2}h ({:.0}km/h)",
                                    distance, hours, speed
                                ));
                                create_case = true;
                            }
                        }
                    }
                }
            }
        }

        // G. Multiple users same device -> flag
        let users_on_device = self
            .store
            .get_users_on_device(&device_ctx.device_fingerprint)
            .await?;
        // If there are other users besides current one
        let other_users = users_on_device
            .iter()
            .filter(|u| *u != &ctx.user_id)
            .count();
        if other_users > 0 {
            risk_score += 20.0 * (other_users as f64);
            reasons.push(format!("Device used by {} other user(s)", other_users));
            if other_users > 2 {
                create_case = true;
            }
        }

        // Update history (fire and forget or wait? wait for now)
        self.store.update_history(&ctx.user_id, &device_ctx).await?;

        if risk_score > 0.0 {
            Ok(RuleResult {
                passed: risk_score < 70.0, // Fail if high risk
                reason: reasons.join("; "),
                risk_score: Some(RiskScore::new(risk_score)),
                severity: if risk_score >= 80.0 {
                    Some(CaseSeverity::High)
                } else {
                    Some(CaseSeverity::Medium)
                },
                create_case: create_case || risk_score >= 50.0,
            })
        } else {
            Ok(RuleResult::pass())
        }
    }
}

// ============================================================================
// Mock Implementation (for testing/default)
// ============================================================================

pub struct MockDeviceHistoryStore {
    history: Mutex<std::collections::HashMap<String, DeviceHistory>>,
    device_users: Mutex<std::collections::HashMap<String, std::collections::HashSet<String>>>,
}

impl Default for MockDeviceHistoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MockDeviceHistoryStore {
    pub fn new() -> Self {
        Self {
            history: Mutex::new(std::collections::HashMap::new()),
            device_users: Mutex::new(std::collections::HashMap::new()),
        }
    }
}

#[async_trait]
impl DeviceHistoryStore for MockDeviceHistoryStore {
    async fn get_history(&self, user_id: &UserId) -> Result<DeviceHistory> {
        let map = self.history.lock().expect("History lock poisoned");
        Ok(map.get(&user_id.to_string()).cloned().unwrap_or_default())
    }

    async fn update_history(&self, user_id: &UserId, context: &DeviceContext) -> Result<()> {
        let mut map = self.history.lock().expect("History lock poisoned");
        let mut device_map = self.device_users.lock().expect("Device users lock poisoned");

        let history = map.entry(user_id.to_string()).or_default();

        // Update Devices
        if !history
            .last_devices
            .iter()
            .any(|d| d.device_fingerprint == context.device_fingerprint)
        {
            history.last_devices.insert(
                0,
                DeviceRecord {
                    device_id: context.device_id.clone(),
                    device_fingerprint: context.device_fingerprint.clone(),
                    last_seen: context.timestamp,
                },
            );
            if history.last_devices.len() > 10 {
                history.last_devices.pop();
            }
        }

        // Update IPs
        if !history
            .last_ips
            .iter()
            .any(|i| i.ip_address == context.ip_address)
        {
            history.last_ips.insert(
                0,
                IpRecord {
                    ip_address: context.ip_address.clone(),
                    last_seen: context.timestamp,
                },
            );
            if history.last_ips.len() > 10 {
                history.last_ips.pop();
            }
        }

        // Update Locations
        if let Some(country) = &context.country {
            // Simplified: New location if country or city differs
            let is_new = match history.last_locations.first() {
                Some(last) => last.country != *country || last.city != context.city,
                None => true,
            };

            if is_new {
                history.last_locations.insert(
                    0,
                    LocationRecord {
                        country: country.clone(),
                        city: context.city.clone(),
                        lat: context.lat,
                        lon: context.lon,
                        last_seen: context.timestamp,
                    },
                );
                if history.last_locations.len() > 10 {
                    history.last_locations.pop();
                }
            }
        }

        // Update Device -> Users mapping
        let users = device_map
            .entry(context.device_fingerprint.clone())
            .or_default();
        users.insert(user_id.to_string());

        Ok(())
    }

    async fn get_users_on_device(&self, device_fingerprint: &str) -> Result<Vec<UserId>> {
        let map = self.device_users.lock().expect("Device users lock poisoned");
        if let Some(users) = map.get(device_fingerprint) {
            Ok(users.iter().map(UserId::new).collect())
        } else {
            Ok(Vec::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aml::TransactionType;
    use crate::rules::RuleContext;
    use chrono::Duration;
    use ramp_common::types::TenantId;
    use rust_decimal::dec;

    fn create_context(user_id: &str, device_ctx: DeviceContext) -> RuleContext {
        RuleContext {
            tenant_id: TenantId::new("tenant1"),
            user_id: UserId::new(user_id),
            current_amount: dec!(100_000),
            transaction_type: TransactionType::Payin,
            timestamp: Utc::now(),
            metadata: serde_json::json!({
                "device": device_ctx
            }),
            user_full_name: None,
            user_country: None,
            user_address: None,
        }
    }

    fn create_device_ctx(device_id: &str, ip: &str, country: &str) -> DeviceContext {
        DeviceContext {
            device_id: device_id.to_string(),
            device_fingerprint: format!("fp_{}", device_id),
            ip_address: ip.to_string(),
            country: Some(country.to_string()),
            city: Some("City".to_string()),
            lat: Some(10.0),
            lon: Some(100.0),
            user_agent: "Mozilla".to_string(),
            is_vpn: false,
            is_proxy: false,
            timestamp: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_new_device_detection() {
        let store = Arc::new(MockDeviceHistoryStore::new());
        let rule = DeviceAnomalyRule::new(store);

        let device1 = create_device_ctx("d1", "1.1.1.1", "VN");
        let ctx1 = create_context("u1", device1.clone());

        // First time - new device but no history, so maybe okay or just logged
        let result1 = rule.evaluate(&ctx1).await.expect("Failed to evaluate rule");
        // Since history is empty, new device check: !known && !empty -> false.
        // So passed.
        assert!(result1.passed);

        // Second time - same device
        let result2 = rule.evaluate(&ctx1).await.expect("Failed to evaluate rule");
        assert!(result2.passed);

        // Third time - new device
        let device2 = create_device_ctx("d2", "2.2.2.2", "VN");
        let ctx2 = create_context("u1", device2);
        let result3 = rule.evaluate(&ctx2).await.expect("Failed to evaluate rule");

        // Should flag new device (risk +10)
        // 10 < 50 so pass, but risk score > 0
        assert!(result3.risk_score.expect("Risk score missing").0 >= 10.0);
        assert!(result3.reason.contains("New device"));
    }

    #[tokio::test]
    async fn test_impossible_travel() {
        let store = Arc::new(MockDeviceHistoryStore::new());
        let rule = DeviceAnomalyRule::new(store);

        // 1. Login in Vietnam
        let mut device1 = create_device_ctx("d1", "1.1.1.1", "VN");
        device1.lat = Some(21.0); // Hanoi
        device1.lon = Some(105.0);
        let ctx1 = create_context("u1", device1);
        rule.evaluate(&ctx1).await.expect("Failed to evaluate rule");

        // 2. Login in US 1 hour later
        let mut device2 = create_device_ctx("d2", "3.3.3.3", "US");
        device2.lat = Some(37.0); // San Francisco approx
        device2.lon = Some(-122.0);
        device2.timestamp = Utc::now() + Duration::hours(1);

        let ctx2 = create_context("u1", device2);
        let result = rule.evaluate(&ctx2).await.expect("Failed to evaluate rule");

        assert!(!result.passed); // Should fail due to high risk (country change 50 + impossible travel 40 = 90)
        assert!(result.reason.contains("Impossible travel"));
        assert!(result.create_case);
    }

    #[tokio::test]
    async fn test_vpn_detection() {
        let store = Arc::new(MockDeviceHistoryStore::new());
        let rule = DeviceAnomalyRule::new(store);

        let mut device = create_device_ctx("d1", "1.1.1.1", "VN");
        device.is_vpn = true;
        let ctx = create_context("u1", device);

        let result = rule.evaluate(&ctx).await.expect("Failed to evaluate rule");
        assert!(result.risk_score.expect("Risk score missing").0 >= 30.0);
        assert!(result.reason.contains("VPN"));
    }

    #[tokio::test]
    async fn test_multiple_users_device() {
        let store = Arc::new(MockDeviceHistoryStore::new());
        let rule = DeviceAnomalyRule::new(store);

        let device = create_device_ctx("shared_device", "1.1.1.1", "VN");

        // User 1 uses device
        let ctx1 = create_context("u1", device.clone());
        rule.evaluate(&ctx1).await.expect("Failed to evaluate rule");

        // User 2 uses same device
        let ctx2 = create_context("u2", device.clone());
        let result = rule.evaluate(&ctx2).await.expect("Failed to evaluate rule");

        // Should flag shared device (20 points for 1 other user)
        assert!(result.risk_score.expect("Risk score missing").0 >= 20.0);
        assert!(result.reason.contains("Device used by 1 other user"));
    }

    #[tokio::test]
    async fn test_suspicious_country() {
        let store = Arc::new(MockDeviceHistoryStore::new());
        let rule = DeviceAnomalyRule::new(store);

        let device = create_device_ctx("d1", "1.1.1.1", "NK"); // North Korea
        let ctx = create_context("u1", device);

        let result = rule.evaluate(&ctx).await.expect("Failed to evaluate rule");
        assert!(!result.passed);
        assert!(result.reason.contains("Suspicious country"));
    }

    #[tokio::test]
    async fn test_max_devices_limit() {
        let store = Arc::new(MockDeviceHistoryStore::new());
        let rule = DeviceAnomalyRule::with_config(store, 2, 10, vec![]); // limit 2 devices

        // Device 1
        let d1 = create_device_ctx("d1", "1.1.1.1", "VN");
        rule.evaluate(&create_context("u1", d1)).await.expect("Failed to evaluate rule");

        // Device 2
        let d2 = create_device_ctx("d2", "1.1.1.1", "VN");
        rule.evaluate(&create_context("u1", d2)).await.expect("Failed to evaluate rule");

        // Device 3 (should trigger)
        let d3 = create_device_ctx("d3", "1.1.1.1", "VN");
        let result = rule.evaluate(&create_context("u1", d3)).await.expect("Failed to evaluate rule");

        assert!(result.risk_score.expect("Risk score missing").0 >= 20.0);
        assert!(result.reason.contains("Too many unique devices"));
    }
}
