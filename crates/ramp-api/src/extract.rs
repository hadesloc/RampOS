use axum::{
    async_trait,
    extract::{FromRequest, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::collections::HashMap;
use validator::{Validate, ValidationErrors, ValidationErrorsKind};

/// A wrapper around `axum::Json` that validates the request body.
pub struct ValidatedJson<T>(pub T);

/// Structured validation error response
#[derive(Debug, Serialize)]
pub struct ValidationErrorResponse {
    pub error: ValidationErrorBody,
}

#[derive(Debug, Serialize)]
pub struct ValidationErrorBody {
    pub code: String,
    pub message: String,
    pub details: HashMap<String, Vec<FieldError>>,
}

#[derive(Debug, Serialize, Clone)]
pub struct FieldError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// Validation rejection type with structured error response
#[derive(Debug)]
pub struct ValidationRejection {
    errors: ValidationErrors,
}

impl ValidationRejection {
    pub fn new(errors: ValidationErrors) -> Self {
        Self { errors }
    }

    fn format_errors(&self) -> HashMap<String, Vec<FieldError>> {
        let mut result = HashMap::new();
        Self::collect_errors(&self.errors, String::new(), &mut result);
        result
    }

    fn collect_errors(
        errors: &ValidationErrors,
        prefix: String,
        result: &mut HashMap<String, Vec<FieldError>>,
    ) {
        for (field, error_kind) in errors.errors() {
            let field_name = if prefix.is_empty() {
                field.to_string()
            } else {
                format!("{}.{}", prefix, field)
            };

            match error_kind {
                ValidationErrorsKind::Field(field_errors) => {
                    let formatted_errors: Vec<FieldError> = field_errors
                        .iter()
                        .map(|e| {
                            let message = e
                                .message
                                .as_ref()
                                .map(|m| m.to_string())
                                .unwrap_or_else(|| Self::default_message(&e.code, &e.params));

                            FieldError {
                                code: e.code.to_string(),
                                message,
                                params: if e.params.is_empty() {
                                    None
                                } else {
                                    Some(serde_json::to_value(&e.params).unwrap_or_default())
                                },
                            }
                        })
                        .collect();
                    result.insert(field_name, formatted_errors);
                }
                ValidationErrorsKind::Struct(nested) => {
                    Self::collect_errors(nested, field_name, result);
                }
                ValidationErrorsKind::List(list) => {
                    for (index, nested) in list {
                        let indexed_prefix = format!("{}[{}]", field_name, index);
                        Self::collect_errors(nested, indexed_prefix, result);
                    }
                }
            }
        }
    }

    fn default_message(
        code: &str,
        params: &HashMap<std::borrow::Cow<'static, str>, serde_json::Value>,
    ) -> String {
        match code {
            "length" => {
                let min = params.get("min").and_then(|v| v.as_u64());
                let max = params.get("max").and_then(|v| v.as_u64());
                match (min, max) {
                    (Some(min), Some(max)) => {
                        format!("Length must be between {} and {} characters", min, max)
                    }
                    (Some(min), None) => format!("Length must be at least {} characters", min),
                    (None, Some(max)) => format!("Length must be at most {} characters", max),
                    (None, None) => "Invalid length".to_string(),
                }
            }
            "range" => {
                let min = params.get("min").and_then(|v| v.as_f64());
                let max = params.get("max").and_then(|v| v.as_f64());
                match (min, max) {
                    (Some(min), Some(max)) => format!("Value must be between {} and {}", min, max),
                    (Some(min), None) => format!("Value must be at least {}", min),
                    (None, Some(max)) => format!("Value must be at most {}", max),
                    (None, None) => "Invalid range".to_string(),
                }
            }
            "email" => "Invalid email address".to_string(),
            "url" => "Invalid URL".to_string(),
            "required" => "This field is required".to_string(),
            "regex" => "Value does not match the required pattern".to_string(),
            "custom" => "Validation failed".to_string(),
            _ => format!("Validation failed: {}", code),
        }
    }
}

impl IntoResponse for ValidationRejection {
    fn into_response(self) -> Response {
        let details = self.format_errors();
        let field_count = details.len();

        let message = if field_count == 1 {
            "Validation failed for 1 field".to_string()
        } else {
            format!("Validation failed for {} fields", field_count)
        };

        let body = ValidationErrorResponse {
            error: ValidationErrorBody {
                code: "VALIDATION_ERROR".to_string(),
                message,
                details,
            },
        };

        (StatusCode::BAD_REQUEST, Json(body)).into_response()
    }
}

/// JSON parsing error response
#[derive(Debug)]
pub struct JsonParseRejection {
    message: String,
}

impl IntoResponse for JsonParseRejection {
    fn into_response(self) -> Response {
        let body = serde_json::json!({
            "error": {
                "code": "INVALID_JSON",
                "message": self.message
            }
        });
        (StatusCode::BAD_REQUEST, Json(body)).into_response()
    }
}

/// Combined rejection type for ValidatedJson
#[derive(Debug)]
pub enum ValidatedJsonRejection {
    JsonParse(JsonParseRejection),
    Validation(ValidationRejection),
}

impl IntoResponse for ValidatedJsonRejection {
    fn into_response(self) -> Response {
        match self {
            ValidatedJsonRejection::JsonParse(r) => r.into_response(),
            ValidatedJsonRejection::Validation(r) => r.into_response(),
        }
    }
}

#[async_trait]
impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: Validate + 'static,
    Json<T>: FromRequest<S, Rejection = axum::extract::rejection::JsonRejection>,
    S: Send + Sync,
{
    type Rejection = ValidatedJsonRejection;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state).await.map_err(|err| {
            ValidatedJsonRejection::JsonParse(JsonParseRejection {
                message: format_json_error(&err),
            })
        })?;

        value
            .validate()
            .map_err(|err| ValidatedJsonRejection::Validation(ValidationRejection::new(err)))?;

        Ok(ValidatedJson(value))
    }
}

/// Format JSON parsing errors to be more user-friendly
fn format_json_error(err: &axum::extract::rejection::JsonRejection) -> String {
    use axum::extract::rejection::JsonRejection;

    match err {
        JsonRejection::JsonDataError(e) => {
            let msg = e.body_text();
            // Try to extract the field name and provide a cleaner message
            if msg.contains("missing field") || msg.contains("invalid type") {
                msg.to_string()
            } else {
                format!("Invalid JSON data: {}", msg)
            }
        }
        JsonRejection::JsonSyntaxError(e) => {
            format!("Invalid JSON syntax: {}", e.body_text())
        }
        JsonRejection::MissingJsonContentType(_) => {
            "Request body must be JSON (Content-Type: application/json)".to_string()
        }
        JsonRejection::BytesRejection(_) => "Failed to read request body".to_string(),
        _ => err.to_string(),
    }
}

#[cfg(test)]
#[path = "extract_test.rs"]
mod tests;
