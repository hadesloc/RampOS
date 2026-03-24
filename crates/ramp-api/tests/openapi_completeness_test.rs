//! OpenAPI spec completeness validation tests for F03
//!
//! Validates that the generated OpenAPI spec is complete, well-formed,
//! and documents all major API endpoints.

use ramp_api::openapi::ApiDoc;
use utoipa::OpenApi;

fn get_spec() -> serde_json::Value {
    let doc = ApiDoc::openapi();
    let json_str = doc
        .to_json()
        .expect("OpenAPI spec should serialize to JSON");
    serde_json::from_str(&json_str).expect("OpenAPI JSON should parse as serde_json::Value")
}

fn resolve_schema_ref<'a>(
    spec: &'a serde_json::Value,
    schema: &'a serde_json::Value,
) -> &'a serde_json::Value {
    if let Some(reference) = schema.get("$ref").and_then(|value| value.as_str()) {
        let schema_name = reference
            .strip_prefix("#/components/schemas/")
            .expect("schema ref should point into components/schemas");
        &spec["components"]["schemas"][schema_name]
    } else {
        schema
    }
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
    assert_eq!(
        title,
        Some("RampOS API"),
        "info.title should be 'RampOS API'"
    );

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
    let components = spec.get("components").expect("Spec must have 'components'");
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
        "venue-funding",
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
fn spec_documents_portal_venue_funding_paths() {
    let spec = get_spec();
    let paths = spec
        .get("paths")
        .and_then(|v| v.as_object())
        .expect("Spec must have 'paths' object");

    for path in [
        "/v1/portal/venue-funding/venues",
        "/v1/portal/venue-funding/connection",
        "/v1/portal/venue-funding/eligibility",
        "/v1/portal/venue-funding/prepare",
        "/v1/portal/venue-funding/{id}/submit",
        "/v1/portal/venue-funding/{id}/status",
    ] {
        assert!(paths.contains_key(path), "Spec must document {}", path);
    }
}

#[test]
fn spec_documents_portal_venue_funding_prepare_and_status_contracts() {
    let spec = get_spec();

    let venues = spec["paths"]["/v1/portal/venue-funding/venues"]["get"]
        .as_object()
        .expect("spec must document GET /v1/portal/venue-funding/venues");
    assert!(
        venues
            .get("responses")
            .and_then(|value| value.get("200"))
            .is_some(),
        "venues operation must define a 200 response"
    );

    let connection = spec["paths"]["/v1/portal/venue-funding/connection"]["post"]
        .as_object()
        .expect("spec must document POST /v1/portal/venue-funding/connection");
    assert!(
        connection.get("requestBody").is_some(),
        "connection operation must define a request body"
    );
    assert!(
        connection
            .get("responses")
            .and_then(|value| value.get("200"))
            .is_some(),
        "connection operation must define a 200 response"
    );

    let prepare = spec["paths"]["/v1/portal/venue-funding/prepare"]["post"]
        .as_object()
        .expect("spec must document POST /v1/portal/venue-funding/prepare");
    assert!(
        prepare.get("requestBody").is_some(),
        "prepare operation must define a request body"
    );
    assert!(
        prepare
            .get("responses")
            .and_then(|value| value.get("200"))
            .is_some(),
        "prepare operation must define a 200 response"
    );
    let prepare_request_schema = resolve_schema_ref(
        &spec,
        &prepare["requestBody"]["content"]["application/json"]["schema"],
    )
    .as_object()
    .expect("prepare request schema must exist");
    let prepare_required = prepare_request_schema["required"]
        .as_array()
        .expect("prepare request schema must define required fields")
        .iter()
        .map(|value| value.as_str().expect("required field must be string"))
        .collect::<Vec<_>>();
    for field in [
        "venueKey",
        "jurisdiction",
        "asset",
        "network",
        "venueConnectionId",
        "venueAccountId",
        "walletAttestationId",
        "amount",
    ] {
        assert!(
            prepare_required.contains(&field),
            "prepare request must require '{}'",
            field
        );
    }

    let submit = spec["paths"]["/v1/portal/venue-funding/{id}/submit"]["post"]
        .as_object()
        .expect("spec must document POST /v1/portal/venue-funding/{id}/submit");
    let submit_params = submit["parameters"]
        .as_array()
        .expect("submit operation must define path parameters");
    assert!(
        submit_params.iter().any(|param| param["name"] == "id"),
        "submit operation must document 'id' parameter"
    );
    assert!(
        submit.get("requestBody").is_some(),
        "submit operation must define a request body"
    );
    assert!(
        submit
            .get("responses")
            .and_then(|value| value.get("200"))
            .is_some(),
        "submit operation must define a 200 response"
    );
    let submit_request_schema = resolve_schema_ref(
        &spec,
        &submit["requestBody"]["content"]["application/json"]["schema"],
    )
    .as_object()
    .expect("submit request schema must exist");
    let submit_required = submit_request_schema["required"]
        .as_array()
        .expect("submit request schema must define required fields")
        .iter()
        .map(|value| value.as_str().expect("required field must be string"))
        .collect::<Vec<_>>();
    assert!(
        submit_required.contains(&"walletTransferReference"),
        "submit request must require walletTransferReference"
    );

    let status = spec["paths"]["/v1/portal/venue-funding/{id}/status"]["get"]
        .as_object()
        .expect("spec must document GET /v1/portal/venue-funding/{id}/status");
    let params = status["parameters"]
        .as_array()
        .expect("status operation must define path parameters");
    for expected in ["id"] {
        assert!(
            params.iter().any(|param| param["name"] == expected),
            "status operation must document '{}' parameter",
            expected
        );
    }
    assert!(
        status
            .get("responses")
            .and_then(|value| value.get("200"))
            .is_some(),
        "status operation must define a 200 response"
    );
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
    let components = spec.get("components").expect("Spec must have 'components'");
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
    let paths = spec["paths"].as_object().expect("Spec must have 'paths'");

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

// ============================================================================
// NEW TESTS: Tag completeness
// ============================================================================

#[test]
fn spec_tags_cover_all_endpoint_groups() {
    let spec = get_spec();
    let tags = spec
        .get("tags")
        .and_then(|v| v.as_array())
        .expect("Spec must have 'tags' array");

    let tag_names: Vec<&str> = tags
        .iter()
        .filter_map(|t| t.get("name").and_then(|v| v.as_str()))
        .collect();

    let required_tags = [
        "intents",
        "events",
        "users",
        "admin",
        "health",
        "account-abstraction",
    ];

    for tag in &required_tags {
        assert!(
            tag_names.contains(tag),
            "Tag '{}' must be defined. Available tags: {:?}",
            tag,
            tag_names
        );
    }
}

#[test]
fn spec_tags_have_descriptions() {
    let spec = get_spec();
    let tags = spec
        .get("tags")
        .and_then(|v| v.as_array())
        .expect("Spec must have 'tags' array");

    for tag in tags {
        let name = tag
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let description = tag.get("description").and_then(|v| v.as_str());
        assert!(
            description.is_some() && !description.unwrap().is_empty(),
            "Tag '{}' must have a non-empty description",
            name
        );
    }
}

// ============================================================================
// NEW TESTS: Security scheme detailed validation
// ============================================================================

#[test]
fn spec_has_idempotency_key_scheme() {
    let spec = get_spec();
    let security_schemes = &spec["components"]["securitySchemes"];

    assert!(
        security_schemes.get("idempotency_key").is_some(),
        "Security scheme 'idempotency_key' must be defined for safe retries"
    );

    let idem = &security_schemes["idempotency_key"];
    assert_eq!(
        idem.get("type").and_then(|v| v.as_str()),
        Some("apiKey"),
        "idempotency_key should be type 'apiKey'"
    );
    assert_eq!(
        idem.get("in").and_then(|v| v.as_str()),
        Some("header"),
        "idempotency_key should be in 'header'"
    );
}

#[test]
fn spec_hmac_signature_scheme_details() {
    let spec = get_spec();
    let hmac = &spec["components"]["securitySchemes"]["hmac_signature"];

    assert_eq!(
        hmac.get("type").and_then(|v| v.as_str()),
        Some("apiKey"),
        "hmac_signature should be type 'apiKey'"
    );
    assert_eq!(
        hmac.get("in").and_then(|v| v.as_str()),
        Some("header"),
        "hmac_signature should be in 'header'"
    );
    assert_eq!(
        hmac.get("name").and_then(|v| v.as_str()),
        Some("X-Signature"),
        "hmac_signature header name should be 'X-Signature'"
    );
}

// ============================================================================
// NEW TESTS: Server definitions
// ============================================================================

#[test]
fn spec_servers_have_descriptions() {
    let spec = get_spec();
    let servers = spec
        .get("servers")
        .and_then(|v| v.as_array())
        .expect("Spec must have 'servers' array");

    for server in servers {
        let url = server
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let description = server.get("description").and_then(|v| v.as_str());
        assert!(
            description.is_some() && !description.unwrap().is_empty(),
            "Server '{}' must have a non-empty description",
            url
        );
    }
}

#[test]
fn spec_has_production_server() {
    let spec = get_spec();
    let servers = spec
        .get("servers")
        .and_then(|v| v.as_array())
        .expect("Spec must have 'servers' array");

    let has_production = servers.iter().any(|s| {
        let url = s.get("url").and_then(|v| v.as_str()).unwrap_or("");
        url.starts_with("https://") && !url.contains("staging") && !url.contains("localhost")
    });

    assert!(
        has_production,
        "Spec must include a production HTTPS server"
    );
}

// ============================================================================
// NEW TESTS: Operation-level validation
// ============================================================================

#[test]
fn spec_all_operations_have_tags() {
    let spec = get_spec();
    let paths = spec["paths"].as_object().expect("Spec must have 'paths'");

    let http_methods = ["get", "post", "put", "patch", "delete"];

    for (path, path_item) in paths {
        let path_obj = path_item.as_object().unwrap();
        for method in &http_methods {
            if let Some(operation) = path_obj.get(*method) {
                let tags = operation.get("tags").and_then(|v| v.as_array());
                assert!(
                    tags.is_some() && !tags.unwrap().is_empty(),
                    "Operation {} {} must have at least one tag",
                    method.to_uppercase(),
                    path
                );
            }
        }
    }
}

#[test]
fn spec_post_operations_have_request_body() {
    let spec = get_spec();
    let paths = spec["paths"].as_object().expect("Spec must have 'paths'");

    // Action endpoints that trigger operations without a request body
    let action_endpoints: Vec<&str> = vec![
        "/v1/admin/domains/{domain_id}/verify-dns",
        "/v1/admin/domains/{domain_id}/provision-ssl",
    ];

    for (path, path_item) in paths {
        let path_obj = path_item.as_object().unwrap();
        if let Some(operation) = path_obj.get("post") {
            if action_endpoints.iter().any(|e| path == e) {
                continue;
            }
            let has_request_body = operation.get("requestBody").is_some();
            assert!(
                has_request_body,
                "POST {} must have a 'requestBody' defined",
                path
            );
        }
    }
}

#[test]
fn spec_operation_ids_are_unique() {
    let spec = get_spec();
    let paths = spec["paths"].as_object().expect("Spec must have 'paths'");

    let http_methods = ["get", "post", "put", "patch", "delete"];
    let mut seen_ids: Vec<String> = Vec::new();

    for (_path, path_item) in paths {
        let path_obj = path_item.as_object().unwrap();
        for method in &http_methods {
            if let Some(operation) = path_obj.get(*method) {
                if let Some(op_id) = operation.get("operationId").and_then(|v| v.as_str()) {
                    assert!(
                        !seen_ids.contains(&op_id.to_string()),
                        "Duplicate operationId found: '{}'",
                        op_id
                    );
                    seen_ids.push(op_id.to_string());
                }
            }
        }
    }

    assert!(
        !seen_ids.is_empty(),
        "Spec should have at least one operation with an operationId"
    );
}

// ============================================================================
// NEW TESTS: Schema completeness
// ============================================================================

#[test]
fn spec_admin_dto_schemas_registered() {
    let spec = get_spec();
    let schemas = spec["components"]["schemas"]
        .as_object()
        .expect("components.schemas must exist");

    let admin_schemas = [
        "CreateTenantRequest",
        "UpdateTenantRequest",
        "SuspendTenantRequest",
        "TierChangeRequest",
    ];

    for name in &admin_schemas {
        assert!(
            schemas.contains_key(*name),
            "Admin schema '{}' must be registered in components.schemas",
            name
        );
    }
}

#[test]
fn spec_aa_dto_schemas_registered() {
    let spec = get_spec();
    let schemas = spec["components"]["schemas"]
        .as_object()
        .expect("components.schemas must exist");

    let aa_schemas = [
        "CreateAccountRequest",
        "CreateAccountResponse",
        "GetAccountResponse",
        "UserOperationDto",
        "SendUserOpRequest",
        "SendUserOpResponse",
        "EstimateGasRequest",
        "EstimateGasResponse",
        "UserOpReceiptDto",
    ];

    for name in &aa_schemas {
        assert!(
            schemas.contains_key(*name),
            "Account Abstraction schema '{}' must be registered",
            name
        );
    }
}

#[test]
fn spec_pagination_schemas_registered() {
    let spec = get_spec();
    let schemas = spec["components"]["schemas"]
        .as_object()
        .expect("components.schemas must exist");

    assert!(
        schemas.contains_key("PaginationInfo"),
        "PaginationInfo schema must be registered for paginated responses"
    );
    assert!(
        schemas.contains_key("ListIntentsResponse"),
        "ListIntentsResponse schema must be registered"
    );
}

#[test]
fn spec_schema_properties_have_types() {
    let spec = get_spec();
    let schemas = spec["components"]["schemas"]
        .as_object()
        .expect("components.schemas must exist");

    // Check a few key schemas have proper properties with types
    let schemas_to_check = ["CreatePayinRequest", "HealthResponse", "ErrorBody"];

    for schema_name in &schemas_to_check {
        let schema = schemas
            .get(*schema_name)
            .unwrap_or_else(|| panic!("Schema '{}' must exist", schema_name));
        let properties = schema.get("properties").and_then(|v| v.as_object());
        assert!(
            properties.is_some() && !properties.unwrap().is_empty(),
            "Schema '{}' must have non-empty properties",
            schema_name
        );

        for (prop_name, prop_def) in properties.unwrap() {
            let has_type = prop_def.get("type").is_some()
                || prop_def.get("$ref").is_some()
                || prop_def.get("allOf").is_some()
                || prop_def.get("oneOf").is_some()
                || prop_def.get("anyOf").is_some();
            assert!(
                has_type,
                "Property '{}' in schema '{}' must have a type or $ref",
                prop_name, schema_name
            );
        }
    }
}

// ============================================================================
// NEW TESTS: Response content-type and status codes
// ============================================================================

#[test]
fn spec_success_responses_have_content_type() {
    let spec = get_spec();
    let paths = spec["paths"].as_object().expect("Spec must have 'paths'");

    let http_methods = ["get", "post", "put", "patch", "delete"];

    for (path, path_item) in paths {
        let path_obj = path_item.as_object().unwrap();
        for method in &http_methods {
            if let Some(operation) = path_obj.get(*method) {
                if let Some(responses) = operation.get("responses").and_then(|v| v.as_object()) {
                    // Check 200 or 201 response has content type
                    for code in &["200", "201"] {
                        if let Some(response) = responses.get(*code) {
                            if let Some(content) =
                                response.get("content").and_then(|v| v.as_object())
                            {
                                assert!(
                                    content.contains_key("application/json"),
                                    "Response {} for {} {} should have application/json content type",
                                    code,
                                    method.to_uppercase(),
                                    path
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn spec_info_has_contact_url() {
    let spec = get_spec();
    let contact = spec["info"]
        .get("contact")
        .expect("info.contact must exist");

    let url = contact.get("url").and_then(|v| v.as_str());
    assert!(
        url.is_some() && !url.unwrap().is_empty(),
        "info.contact.url must be present and non-empty"
    );
}

#[test]
fn spec_openapi_version_is_3x() {
    let spec = get_spec();
    let version = spec["openapi"]
        .as_str()
        .expect("openapi version must be a string");

    assert!(
        version.starts_with("3.0") || version.starts_with("3.1"),
        "OpenAPI version should be 3.0.x or 3.1.x, got: {}",
        version
    );
}

#[test]
fn spec_includes_reconciliation_admin_paths() {
    let spec = get_spec();
    let paths = spec["paths"]
        .as_object()
        .expect("spec.paths must be an object");

    for path in [
        "/v1/admin/reconciliation/workbench",
        "/v1/admin/reconciliation/export",
        "/v1/admin/reconciliation/evidence/{id}",
        "/v1/admin/reconciliation/evidence/{id}/export",
    ] {
        assert!(
            paths.contains_key(path),
            "OpenAPI spec should include reconciliation admin path {}",
            path
        );
    }
}

#[test]
fn spec_includes_treasury_admin_paths() {
    let spec = get_spec();
    let paths = spec["paths"]
        .as_object()
        .expect("spec.paths must be an object");

    for path in ["/v1/admin/treasury/workbench", "/v1/admin/treasury/export"] {
        assert!(
            paths.contains_key(path),
            "OpenAPI spec should include treasury admin path {}",
            path
        );
    }
}

#[test]
fn spec_includes_settlement_admin_paths() {
    let spec = get_spec();
    let paths = spec["paths"]
        .as_object()
        .expect("spec.paths must be an object");

    for path in [
        "/v1/admin/settlement/workbench",
        "/v1/admin/settlement/export",
    ] {
        assert!(
            paths.contains_key(path),
            "OpenAPI spec should include settlement admin path {}",
            path
        );
    }
}

#[test]
fn spec_includes_passport_admin_paths() {
    let spec = get_spec();
    let paths = spec["paths"]
        .as_object()
        .expect("spec.paths must be an object");

    for path in [
        "/v1/admin/passport/queue",
        "/v1/admin/passport/packages/{id}",
    ] {
        assert!(
            paths.contains_key(path),
            "OpenAPI spec should include passport admin path {}",
            path
        );
    }
}

#[test]
fn spec_includes_kyb_admin_paths() {
    let spec = get_spec();
    let paths = spec["paths"]
        .as_object()
        .expect("spec.paths must be an object");

    for path in ["/v1/admin/kyb/reviews", "/v1/admin/kyb/graph/{id}"] {
        assert!(
            paths.contains_key(path),
            "OpenAPI spec should include kyb admin path {}",
            path
        );
    }
}

#[test]
fn spec_includes_config_bundle_admin_paths() {
    let spec = get_spec();
    let paths = spec["paths"]
        .as_object()
        .expect("spec.paths must be an object");

    for path in ["/v1/admin/config-bundles/export", "/v1/admin/extensions"] {
        assert!(
            paths.contains_key(path),
            "OpenAPI spec should include config bundle admin path {}",
            path
        );
    }
}

#[test]
fn spec_extensions_actions_document_governance_metadata() {
    let spec = get_spec();
    let extensions_get = spec["paths"]["/v1/admin/extensions"]["get"]
        .as_object()
        .expect("spec must document GET /v1/admin/extensions");
    let action_item = extensions_get["responses"]["200"]["content"]["application/json"]["schema"]
        ["properties"]["actions"]["items"]
        .as_object()
        .expect("extensions actions items schema must exist");
    let required = action_item["required"]
        .as_array()
        .expect("extensions actions required fields must be documented");
    let properties = action_item["properties"]
        .as_object()
        .expect("extensions action properties must be documented");

    assert!(
        required.iter().any(|value| value == "approvalStatus"),
        "extensions action schema must require approvalStatus"
    );
    assert!(
        required.iter().any(|value| value == "provenance"),
        "extensions action schema must require provenance"
    );
    assert!(
        properties.contains_key("approvalStatus"),
        "extensions action schema must document approvalStatus"
    );
    assert!(
        properties.contains_key("provenance"),
        "extensions action schema must document provenance"
    );
}

#[test]
fn spec_includes_provider_routing_admin_paths() {
    let spec = get_spec();
    let paths = spec["paths"]
        .as_object()
        .expect("spec.paths must be an object");

    for path in [
        "/v1/admin/provider-routing/snapshot",
        "/v1/admin/provider-routing/evaluate",
    ] {
        assert!(
            paths.contains_key(path),
            "OpenAPI spec should include provider routing admin path {}",
            path
        );
    }
}

#[test]
fn spec_provider_routing_evaluate_keeps_provider_family_optional() {
    let spec = get_spec();
    let request_schema = spec["paths"]["/v1/admin/provider-routing/evaluate"]["post"]
        ["requestBody"]["content"]["application/json"]["schema"]
        .as_object()
        .expect("provider routing request schema must exist");
    let required = request_schema
        .get("required")
        .and_then(|value| value.as_array())
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    assert!(
        !required.contains(&"providerFamily"),
        "providerFamily must remain optional for backward-compatible fallback evaluation"
    );
}

#[test]
fn spec_includes_commercial_readiness_admin_paths() {
    let spec = get_spec();
    let paths = spec["paths"]
        .as_object()
        .expect("spec.paths must be an object");

    for path in [
        "/v1/admin/commercial-readiness/snapshot",
        "/v1/admin/commercial-readiness/check",
    ] {
        assert!(
            paths.contains_key(path),
            "OpenAPI spec should include commercial readiness admin path {}",
            path
        );
    }
}

#[test]
fn spec_commercial_readiness_snapshot_contract() {
    let spec = get_spec();
    let snapshot_schema = spec["paths"]["/v1/admin/commercial-readiness/snapshot"]["get"]
        ["responses"]["200"]["content"]["application/json"]["schema"]
        .as_object()
        .expect("snapshot response schema must exist");
    let required = snapshot_schema["required"]
        .as_array()
        .expect("snapshot schema required array must exist")
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(
        required.contains(&"provenance"),
        "snapshot response must include provenance"
    );
    assert!(
        required.contains(&"extensions"),
        "snapshot response must include extensions list"
    );
    let properties = snapshot_schema["properties"]
        .as_object()
        .expect("snapshot schema properties must exist");
    assert_eq!(properties["extensions"]["type"], "array");
    assert_eq!(properties["provenance"]["type"], "object");
}

#[test]
fn spec_commercial_readiness_check_contract() {
    let spec = get_spec();
    let check = &spec["paths"]["/v1/admin/commercial-readiness/check"]["post"];
    let request_schema = check["requestBody"]["content"]["application/json"]["schema"]
        .as_object()
        .expect("check request schema must exist");
    let required = request_schema["required"]
        .as_array()
        .expect("check request schema required array must exist")
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(required.contains(&"extensionId"));
    assert!(required.contains(&"availablePartnerCapabilities"));
    assert!(request_schema["properties"]["availablePartnerCapabilities"]["type"] == "array");
    let response_schema = check["responses"]["200"]["content"]["application/json"]["schema"]
        .as_object()
        .expect("check response schema must exist");
    let response_required = response_schema["required"]
        .as_array()
        .expect("check response schema required array must exist")
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(response_required.contains(&"missingCapabilities"));
    assert!(response_required.contains(&"provenance"));
}

#[test]
fn spec_includes_commercialization_pack_admin_paths() {
    let spec = get_spec();
    let paths = spec["paths"]
        .as_object()
        .expect("spec.paths must be an object");

    assert!(
        paths.contains_key("/v1/admin/commercialization-packs"),
        "OpenAPI spec should include commercialization pack admin path"
    );
}

#[test]
fn spec_commercialization_pack_list_contract() {
    let spec = get_spec();
    let list_schema = spec["paths"]["/v1/admin/commercialization-packs"]["get"]["responses"]["200"]
        ["content"]["application/json"]["schema"]
        .as_object()
        .expect("commercialization pack list schema must exist");
    let required = list_schema["required"]
        .as_array()
        .expect("list schema required array must exist")
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(required.contains(&"packs"));
    assert!(required.contains(&"provenance"));
    assert_eq!(list_schema["properties"]["packs"]["type"], "array");
}

#[test]
fn spec_commercialization_pack_upsert_contract() {
    let spec = get_spec();
    let post = &spec["paths"]["/v1/admin/commercialization-packs"]["post"];
    let request_schema = post["requestBody"]["content"]["application/json"]["schema"]
        .as_object()
        .expect("commercialization pack request schema must exist");
    let required = request_schema["required"]
        .as_array()
        .expect("commercialization pack request required array must exist")
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(required.contains(&"commercializationPackId"));
    assert!(required.contains(&"packCode"));
    assert!(required.contains(&"partnerId"));
    assert!(required.contains(&"commercialExtensionId"));
    assert!(required.contains(&"corridorCode"));
    let response_schema = post["responses"]["200"]["content"]["application/json"]["schema"]
        .as_object()
        .expect("commercialization pack response schema must exist");
    let response_required = response_schema["required"]
        .as_array()
        .expect("commercialization pack response required array must exist")
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(response_required.contains(&"packs"));
    assert!(response_required.contains(&"provenance"));
}

#[test]
fn spec_execution_explainability_admin_paths() {
    let spec = get_spec();
    let paths = spec["paths"]
        .as_object()
        .expect("spec.paths must be an object");

    for path in [
        "/v1/admin/execution-explainability/explain",
        "/v1/admin/execution-explainability/compare",
    ] {
        assert!(
            paths.contains_key(path),
            "OpenAPI spec should include execution explainability admin path {}",
            path
        );
    }
}

#[test]
fn spec_execution_explainability_explain_contract() {
    let spec = get_spec();
    let explain_post = spec["paths"]["/v1/admin/execution-explainability/explain"]["post"]
        .as_object()
        .expect("spec must document POST /v1/admin/execution-explainability/explain");

    let request_schema = explain_post["requestBody"]["content"]["application/json"]["schema"]
        .as_object()
        .expect("explain request schema must exist");
    let request_required = request_schema["required"]
        .as_array()
        .expect("explain request schema must list required fields")
        .iter()
        .map(|value| value.as_str().expect("required field must be string"))
        .collect::<Vec<_>>();
    let explain_required = [
        "routeId",
        "lpId",
        "direction",
        "asset",
        "quotedRate",
        "quotedVndAmount",
        "treasuryStressActive",
        "corridorPolicyEligible",
        "complianceEligible",
    ];
    for required in explain_required {
        assert!(
            request_required.contains(&required),
            "Explain request must require '{}'",
            required
        );
    }

    let response_schema = explain_post["responses"]["200"]["content"]["application/json"]["schema"]
        .as_object()
        .expect("explain response schema must exist");
    let response_required = response_schema["required"]
        .as_array()
        .expect("explain response schema must list required fields")
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("response required field must be string")
        })
        .collect::<Vec<_>>();
    for required in &["routeId", "eligible", "factors", "provenance"] {
        assert!(
            response_required.contains(required),
            "Explain response must require '{}'",
            required
        );
    }
    let properties = response_schema["properties"]
        .as_object()
        .expect("explain response must document properties");
    assert!(
        properties.contains_key("provenance"),
        "Explain response must document provenance"
    );
    assert!(
        properties.contains_key("factors"),
        "Explain response must document factors"
    );
    let factor_schema = properties["factors"]["items"]
        .as_object()
        .expect("Explain factors must document item schema");
    let factor_required = factor_schema["required"]
        .as_array()
        .expect("Explain factors must list required fields")
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("factor required field must be string")
        })
        .collect::<Vec<_>>();
    for required in &[
        "factorName",
        "weight",
        "rawValue",
        "contribution",
        "explanation",
    ] {
        assert!(
            factor_required.contains(required),
            "Explain factors must require '{}'",
            required
        );
    }
}

#[test]
fn spec_execution_explainability_compare_contract() {
    let spec = get_spec();
    let compare_post = spec["paths"]["/v1/admin/execution-explainability/compare"]["post"]
        .as_object()
        .expect("spec must document POST /v1/admin/execution-explainability/compare");

    let request_schema = compare_post["requestBody"]["content"]["application/json"]["schema"]
        .as_object()
        .expect("compare request schema must exist");
    let request_required = request_schema["required"]
        .as_array()
        .expect("compare request schema must list required fields")
        .iter()
        .map(|value| value.as_str().expect("required field must be string"))
        .collect::<Vec<_>>();
    for required in &["direction", "inputs"] {
        assert!(
            request_required.contains(required),
            "Compare request must require '{}'",
            required
        );
    }
    let inputs_items = request_schema["properties"]["inputs"]["items"]
        .as_object()
        .expect("compare inputs items schema must exist");
    let input_required = inputs_items["required"]
        .as_array()
        .expect("compare input items must list required fields")
        .iter()
        .map(|value| value.as_str().expect("required field must be string"))
        .collect::<Vec<_>>();
    for required in &[
        "routeId",
        "lpId",
        "direction",
        "asset",
        "quotedRate",
        "quotedVndAmount",
        "treasuryStressActive",
        "corridorPolicyEligible",
        "complianceEligible",
    ] {
        assert!(
            input_required.contains(required),
            "Compare input must require '{}'",
            required
        );
    }

    let response_schema = compare_post["responses"]["200"]["content"]["application/json"]["schema"]
        .as_object()
        .expect("compare response schema must exist");
    let response_required = response_schema["required"]
        .as_array()
        .expect("compare response schema must list required fields")
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("response required field must be string")
        })
        .collect::<Vec<_>>();
    for required in &["candidates", "winningRouteId", "provenance"] {
        assert!(
            response_required.contains(required),
            "Compare response must require '{}'",
            required
        );
    }
    let response_props = response_schema["properties"]
        .as_object()
        .expect("compare response must document properties");
    assert!(
        response_props.contains_key("provenance"),
        "Compare response must document provenance"
    );
    assert!(
        response_props.contains_key("candidates"),
        "Compare response must document candidates"
    );
    let candidate_schema = response_props["candidates"]["items"]
        .as_object()
        .expect("Compare response must document candidate item schema");
    let candidate_required = candidate_schema["required"]
        .as_array()
        .expect("Compare candidates must list required fields")
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("candidate required field must be string")
        })
        .collect::<Vec<_>>();
    for required in &[
        "routeId",
        "lpId",
        "direction",
        "compositeScore",
        "factors",
        "eligible",
        "ineligibilityReasons",
        "summary",
        "provenance",
    ] {
        assert!(
            candidate_required.contains(required),
            "Compare candidates must require '{}'",
            required
        );
    }
}

#[test]
fn spec_includes_venue_trust_admin_paths() {
    let spec = get_spec();
    let paths = spec["paths"]
        .as_object()
        .expect("spec.paths must be an object");

    for path in [
        "/v1/admin/venue-trust/connections/{id}/review",
        "/v1/admin/venue-trust/subjects/{subject_type}/{subject_id}",
        "/v1/admin/venue-trust/transfers/{id}",
        "/v1/admin/venue-trust/transfers/{id}/review",
        "/v1/admin/venue-trust/source-of-funds-packages/{id}/review",
        "/v1/admin/venue-trust/wallet-attestations/{id}/review",
    ] {
        assert!(
            paths.contains_key(path),
            "OpenAPI spec should include venue trust admin path {}",
            path
        );
    }
}

#[test]
fn spec_venue_trust_operator_review_paths_require_review_body() {
    let spec = get_spec();

    for path in [
        "/v1/admin/venue-trust/connections/{id}/review",
        "/v1/admin/venue-trust/transfers/{id}/review",
        "/v1/admin/venue-trust/source-of-funds-packages/{id}/review",
    ] {
        let review_post = spec["paths"][path]["post"]
            .as_object()
            .unwrap_or_else(|| panic!("spec must document POST {}", path));

        let request_body = review_post
            .get("requestBody")
            .and_then(|value| value.as_object())
            .expect("review endpoint must define a request body");
        assert_eq!(
            request_body
                .get("required")
                .and_then(|value| value.as_bool()),
            Some(true),
            "review request body must be required for {}",
            path
        );

        let request_schema = &review_post["requestBody"]["content"]["application/json"]["schema"];
        let required = request_schema["required"]
            .as_array()
            .expect("review request schema must list required fields")
            .iter()
            .map(|value| value.as_str().expect("required field must be string"))
            .collect::<Vec<_>>();
        for field in ["status", "reviewReason"] {
            assert!(
                required.contains(&field),
                "review request must require '{}' for {}",
                field,
                path
            );
        }

        let properties = request_schema["properties"]
            .as_object()
            .expect("review request schema must document properties");
        for field in ["status", "reviewReason", "failureReason", "provenance"] {
            assert!(
                properties.contains_key(field),
                "review request must document '{}' for {}",
                field,
                path
            );
        }
        assert_eq!(
            properties["status"]["type"].as_str(),
            Some("string"),
            "status must be string for {}",
            path
        );
        assert_eq!(
            properties["reviewReason"]["type"].as_str(),
            Some("string"),
            "reviewReason must be string for {}",
            path
        );
        assert_eq!(
            properties["failureReason"]["type"].as_str(),
            Some("string"),
            "failureReason must be string for {}",
            path
        );
        assert_eq!(
            properties["provenance"]["type"].as_str(),
            Some("object"),
            "provenance must be object for {}",
            path
        );
    }
}

#[test]
fn spec_venue_trust_wallet_attestation_review_contract() {
    let spec = get_spec();
    let review_post = spec["paths"]["/v1/admin/venue-trust/wallet-attestations/{id}/review"]
        ["post"]
        .as_object()
        .expect("spec must document POST /v1/admin/venue-trust/wallet-attestations/{id}/review");

    let request_body = review_post
        .get("requestBody")
        .and_then(|value| value.as_object())
        .expect("review endpoint must define a request body");
    assert_eq!(
        request_body
            .get("required")
            .and_then(|value| value.as_bool()),
        Some(true),
        "review request body must be required"
    );

    let request_schema = &review_post["requestBody"]["content"]["application/json"]["schema"];
    let required = request_schema["required"]
        .as_array()
        .expect("review request schema must list required fields")
        .iter()
        .map(|value| value.as_str().expect("required field must be string"))
        .collect::<Vec<_>>();
    for field in ["status", "reviewReason"] {
        assert!(
            required.contains(&field),
            "review request must require '{}'",
            field
        );
    }

    let properties = request_schema["properties"]
        .as_object()
        .expect("review request schema must document properties");
    for field in ["status", "reviewReason", "failureReason", "provenance"] {
        assert!(
            properties.contains_key(field),
            "review request must document '{}'",
            field
        );
    }
    assert_eq!(properties["status"]["type"].as_str(), Some("string"));
    assert_eq!(properties["reviewReason"]["type"].as_str(), Some("string"));
    assert_eq!(properties["failureReason"]["type"].as_str(), Some("string"));
    assert_eq!(properties["provenance"]["type"].as_str(), Some("object"));
}
