use ramp_compliance::KybGraphService;

#[test]
fn kyb_graph_lists_review_queue() {
    let queue = KybGraphService::new().list_reviews(None);

    assert!(!queue.is_empty());
    assert_eq!(queue[0].review_status, "needs_review");
}

#[test]
fn kyb_graph_clean_scenario_has_no_missing_requirements() {
    let item = KybGraphService::new()
        .graph_for_entity("biz_clean_001", Some("clean"))
        .expect("clean graph");

    assert!(item.summary.missing_requirements.is_empty());
}
