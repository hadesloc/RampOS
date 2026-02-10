//! OpenAPI spec completeness validation tests for F03
//!
//! Validates that the generated OpenAPI spec is complete, well-formed,
//! and documents all major API endpoints.

use ramp_api::openapi::ApiDoc;
use utoipa::OpenApi;

fn get_spec() -> serde_json::Value {
    let doc = ApiDoc::openapi();
    let json_str = doc.to_json().expect("OpenAPI spec should serialize to JSON");
    serde_json::from_str(&json_str).expect("OpenAPI JSON should parse as serde_json::Value")
}

#[test]
fn spec_generates_valid_json() {
    let spec = get_spec();
    assert!(spec.is_object(), "OpenAPI spec should be a JSON object");
    assert!(
        spec.get("openapi").is_some(),
        "Spec must have an 'openapi' version field"
    );
    let version = spec["openapi"].as_str().unwrap();
    assert!(
        version.starts_with("3."),
        "OpenAPI version should be 3.x, got: {}",
        version
    );
}

#[test]
fn spec_has_info_fields() {
    let spec = get_spec();
    let info = spec.get("info").expect("Spec must have 'info' section");

    let title = info.get("title").and_then(|v| v.as_str());
    assert_eq!(title, Some("RampOS API"), "info.title should be 'RampOS API'");

    let version = info.get("version").and_then(|v| v.as_str());
    assert_eq!(version, Some("1.0.0"), "info.version should be '1.0.0'");

    assert!(
        info.get("description").and_then(|v| v.as_str()).is_some(),
        "info.description should be present"
    );

    let contact = info.get("contact").expect("info.contact should be present");
    assert!(
        contact.get("name").and_then(|v| v.as_str()).is_some(),
        "contact.name should be present"
    );
    assert!(
        contact.get("email").and_then(|v| v.as_str()).is_some(),
        "contact.email should be present"
    );

    let license = info.get("license").expect("info.license should be present");
    assert_eq!(
        license.get("name").and_then(|v| v.as_str()),
        Some("MIT"),
        "license.name should be 'MIT'"
    );
}

#[test]
fn spec_has_servers_defined() {
    let spec = get_spec();
    let servers = spec
        .get("servers")
        .and_then(|v| v.as_array())
        .expect("Spec must have 'servers' array");

    assert!(
        servers.len() >= 2,
        "Should have at least 2 server entries (production + dev), got: {}",
        servers.len()
    );

    let urls: Vec<&str> = servers
        .iter()
        .filter_map(|s| s.get("url").and_then(|v| v.as_str()))
        .collect();

    assert!(
        urls.iter().any(|u| u.contains("localhost")),
        "Should have a localhost development server"
    );
}

#[test]
fn spec_has_security_schemes() {
    let spec = get_spec();
    let components = spec
        .get("components")
        .expect("Spec must have 'components'");
    let security_schemes = components
        .get("securitySchemes")
        .expect("components must have 'securitySchemes'");

    assert!(
        security_schemes.get("bearer_auth").is_some(),
        "Security scheme 'bearer_auth' must be defined"
    );
    assert!(
        security_schemes.get("hmac_signature").is_some(),
        "Security scheme 'hmac_signature' must be defined"
    );

    let bearer = &security_schemes["bearer_auth"];
    assert_eq!(
        bearer.get("type").and_then(|v| v.as_str()),
        Some("http"),
        "bearer_auth should be type 'http'"
    );
    assert_eq!(
        bearer.get("scheme").and_then(|v| v.as_str()),
        Some("bearer"),
        "bearer_auth scheme should be 'bearer'"
    );
}

#[test]
fn spec_documents_major_endpoint_paths() {
    let spec = get_spec();
    let paths = spec
        .get("paths")
        .and_then(|v| v.as_object())
        .expect("Spec must have 'paths' object");

    let path_keys: Vec<&String> = paths.keys().collect();

    let required_path_fragments = [
        "payin",
        "payout",
        "intent",
        "health",
        "balance",
        "trade",
        "account",
        "user-operation",
    ];

    for fragment in &required_path_fragments {
        let found = path_keys.iter().any(|p| p.contains(fragment));
        assert!(
            found,
            "Expected a path containing '{}' in the spec. Paths: {:?}",
            fragment, path_keys
        );
    }
}

#[test]
fn spec_has_reasonable_path_count() {
    let spec = get_spec();
    let paths = spec
        .get("paths")
        .and_then(|v| v.as_object())
        .expect("Spec must have 'paths' object");

    assert!(
        paths.len() > 5,
        "Should have more than 5 documented endpoints, got: {}",
        paths.len()
    );
    assert!(
        paths.len() >= 10,
        "Should have at least 10 documented endpoints for a comprehensive API, got: {}",
        paths.len()
    );
}

#[test]
fn spec_has_component_schemas() {
    let spec = get_spec();
    let components = spec
        .get("components")
        .expect("Spec must have 'components'");
    let schemas = components
        .get("schemas")
        .and_then(|v| v.as_object())
        .expect("components must have 'schemas' object");

    let required_schemas = [
        "CreatePayinRequest",
        "CreatePayoutRequest",
        "IntentResponse",
        "ErrorResponse",
        "HealthResponse",
        "CreateAccountRequest",
        "SendUserOpRequest",
    ];

    for schema_name in &required_schemas {
        assert!(
            schemas.contains_key(*schema_name),
            "Schema '{}' must be defined in components.schemas. Available: {:?}",
            schema_name,
            schemas.keys().collect::<Vec<_>>()
        );
    }

    assert!(
        schemas.len() >= 15,
        "Should have at least 15 component schemas, got: {}",
        schemas.len()
    );
}

#[test]
fn all_path_operations_have_operation_id() {
    let spec = get_spec();
    let paths = spec
        .get("paths")
        .and_then(|v| v.as_object())
        .expect("Spec must have 'paths' object");

    let http_methods = ["get", "post", "put", "patch", "delete"];

    for (path, path_item) in paths {
        let path_obj = path_item.as_object().expect("path item should be object");
        for method in &http_methods {
            if let Some(operation) = path_obj.get(*method) {
                let op_id = operation.get("operationId").and_then(|v| v.as_str());
                assert!(
                    op_id.is_some() && !op_id.unwrap().is_empty(),
                    "Operation {} {} must have a non-empty operationId",
                    method.to_uppercase(),
                    path
                );
            }
        }
    }
}

#[test]
fn spec_documents_error_response_schemas() {
    let spec = get_spec();
    let schemas = spec["components"]["schemas"]
        .as_object()
        .expect("components.schemas must exist");

    let error_schemas = [
        "ErrorResponse",
        "ErrorBody",
        "ValidationErrorResponse",
        "ValidationErrorBody",
        "ValidationFieldError",
    ];

    for name in &error_schemas {
        assert!(
            schemas.contains_key(*name),
            "Error schema '{}' must be documented for consistent error handling",
            name
        );
    }

    // ErrorBody must have 'code' and 'message' properties
    let error_body = &schemas["ErrorBody"];
    let props = error_body
        .get("properties")
        .and_then(|v| v.as_object())
        .expect("ErrorBody must have properties");
    assert!(
        props.contains_key("code"),
        "ErrorBody must have 'code' property"
    );
    assert!(
        props.contains_key("message"),
        "ErrorBody must have 'message' property"
    );

    // ValidationErrorBody must have 'details' for field-level errors
    let val_body = &schemas["ValidationErrorBody"];
    let val_props = val_body
        .get("properties")
        .and_then(|v| v.as_object())
        .expect("ValidationErrorBody must have properties");
    assert!(
        val_props.contains_key("details"),
        "ValidationErrorBody must have 'details' for field-level error info"
    );
}

#[test]
fn spec_endpoints_have_response_schemas() {
    let spec = get_spec();
    let paths = spec["paths"]
        .as_object()
        .expect("Spec must have 'paths'");

    let http_methods = ["get", "post", "put", "patch", "delete"];

    for (path, path_item) in paths {
        let path_obj = path_item.as_object().unwrap();
        for method in &http_methods {
            if let Some(operation) = path_obj.get(*method) {
                let responses = operation.get("responses");
                assert!(
                    responses.is_some(),
                    "Operation {} {} must have 'responses' defined",
                    method.to_uppercase(),
                    path
                );
                let resp_obj = responses
                    .unwrap()
                    .as_object()
                    .expect("responses should be an object");
                assert!(
                    !resp_obj.is_empty(),
                    "Operation {} {} must have at least one response code",
                    method.to_uppercase(),
                    path
                );
            }
        }
    }
}
