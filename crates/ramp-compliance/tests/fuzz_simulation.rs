use ramp_compliance::rule_parser::RuleParser;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

#[test]
fn fuzz_rule_parser_simulation() {
    let mut rng = thread_rng();
    let start_time = std::time::Instant::now();
    let max_duration = std::time::Duration::from_secs(60); // Run for 60 seconds as requested

    let mut iterations = 0;

    println!("Starting fuzz simulation for 60 seconds...");

    while start_time.elapsed() < max_duration {
        // 1. Completely random alphanumeric strings
        let len = rng.gen_range(0..1024);
        let s: String = (0..len).map(|_| rng.sample(Alphanumeric) as char).collect();

        let _ = RuleParser::parse_json(&s);
        let _ = RuleParser::parse(&s);

        // 2. JSON-like structures with random values
        let json_template = r#"
        {
            "id": "RULE_ID",
            "name": "RULE_NAME",
            "type": "velocity",
            "enabled": ENABLED,
            "parameters": {
                "max_count": MAX_COUNT,
                "window_hours": WINDOW,
                "min_total_vnd": AMOUNT
            },
            "conditions": [
                {"field": "FIELD", "operator": "OP", "value": VAL}
            ]
        }
        "#;

        let corrupted_json = json_template
            .replace("RULE_ID", &s)
            .replace("RULE_NAME", &format!("Name_{}", s))
            .replace("ENABLED", if rng.gen() { "true" } else { "false" })
            .replace("MAX_COUNT", &rng.gen_range(1..100).to_string())
            .replace("WINDOW", &rng.gen_range(1..24).to_string())
            .replace("AMOUNT", &rng.gen_range(1000..1000000).to_string())
            .replace("FIELD", "amount")
            .replace("OP", "gt")
            .replace("VAL", &rng.gen_range(1..100).to_string());

        // Randomly truncate or corrupt the JSON
        if rng.gen_bool(0.1) {
            let cut_idx = rng.gen_range(0..corrupted_json.len());
            let (part1, _) = corrupted_json.split_at(cut_idx);
            let _ = RuleParser::parse_json(part1);
        } else {
            let _ = RuleParser::parse_json(&corrupted_json);
        }

        iterations += 1;
        if iterations % 10000 == 0 {
            // Avoid printing too much
        }
    }

    println!("Completed {} iterations without crashing.", iterations);
}
