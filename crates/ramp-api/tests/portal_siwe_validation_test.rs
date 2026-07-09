use chrono::{Duration, TimeZone, Utc};
use ramp_api::handlers::portal::auth::siwe::{parse_and_validate_message, SiweValidationConfig};

#[test]
fn siwe_message_validation_enforces_server_contract() {
    let now = Utc
        .with_ymd_and_hms(2026, 6, 25, 12, 0, 0)
        .single()
        .unwrap();
    let config = SiweValidationConfig {
        domain: "localhost:3000".to_string(),
        uri: "http://localhost:3000".to_string(),
        chain_id: 137,
        max_age: Duration::minutes(10),
        clock_skew: Duration::seconds(30),
    };

    let valid = message(
        "localhost:3000",
        "http://localhost:3000",
        137,
        "2026-06-25T11:55:00Z",
    );
    let parsed =
        parse_and_validate_message(&valid, &config, now).expect("valid SIWE message should pass");
    assert_eq!(parsed.address, "0xabcdef1234567890abcdef1234567890abcdef12");
    assert_eq!(parsed.nonce, "AbCdEfGhIjKlMnOp12345678");

    let wrong_domain = message(
        "evil.example",
        "http://localhost:3000",
        137,
        "2026-06-25T11:55:00Z",
    );
    assert!(parse_and_validate_message(&wrong_domain, &config, now).is_err());

    let wrong_uri = message(
        "localhost:3000",
        "https://evil.example",
        137,
        "2026-06-25T11:55:00Z",
    );
    assert!(parse_and_validate_message(&wrong_uri, &config, now).is_err());

    let wrong_chain = message(
        "localhost:3000",
        "http://localhost:3000",
        1,
        "2026-06-25T11:55:00Z",
    );
    assert!(parse_and_validate_message(&wrong_chain, &config, now).is_err());

    let expired = message(
        "localhost:3000",
        "http://localhost:3000",
        137,
        "2026-06-25T11:40:00Z",
    );
    assert!(parse_and_validate_message(&expired, &config, now).is_err());

    let future = message(
        "localhost:3000",
        "http://localhost:3000",
        137,
        "2026-06-25T12:01:00Z",
    );
    assert!(parse_and_validate_message(&future, &config, now).is_err());
}

fn message(domain: &str, uri: &str, chain_id: u64, issued_at: &str) -> String {
    format!(
        "{domain} wants you to sign in with your Ethereum account:\n\
         0xabcdef1234567890abcdef1234567890abcdef12\n\
         \n\
         Sign in to RampOS Portal.\n\
         \n\
         URI: {uri}\n\
         Version: 1\n\
         Chain ID: {chain_id}\n\
         Nonce: AbCdEfGhIjKlMnOp12345678\n\
         Issued At: {issued_at}"
    )
}
