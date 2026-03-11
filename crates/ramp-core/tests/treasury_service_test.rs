use ramp_core::service::TreasuryService;

#[test]
fn treasury_control_tower_exposes_recommendation_only_snapshot() {
    let service = TreasuryService::new();
    let snapshot = service.build_control_tower(None);

    assert_eq!(snapshot.action_mode, "recommendation_only");
    assert_eq!(snapshot.forecast_window_hours, 24);
    assert!(!snapshot.float_slices.is_empty());
    assert!(!snapshot.alerts.is_empty());
    assert!(snapshot
        .recommendations
        .iter()
        .any(|item| item.id == "treasury_prefund_bank_vnd"));
    assert!(snapshot
        .recommendations
        .iter()
        .all(|item| item.mode == "recommendation_only"));
}

#[test]
fn treasury_control_tower_stable_scenario_avoids_shortage_forecast() {
    let service = TreasuryService::new();
    let snapshot = service.build_control_tower(Some("stable"));

    assert!(snapshot
        .forecasts
        .iter()
        .all(|forecast| forecast.shortage_amount == "0"));
    assert!(snapshot
        .alerts
        .iter()
        .all(|alert| alert.id != "alert_float_shortage"));
}
