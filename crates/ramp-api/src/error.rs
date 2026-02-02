use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

/// API error type
#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    NotFound(String),
    Conflict(String),
    Gone(String),
    UnprocessableEntity(String),
    TooManyRequests(String),
    Internal(String),
    Validation(String),
    Business(String),
}

#[derive(Serialize)]
struct ErrorResponse {
    error: ErrorBody,
}

#[derive(Serialize)]
struct ErrorBody {
    code: String,
    message: String,
}

impl ApiError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            ApiError::Forbidden(_) => StatusCode::FORBIDDEN,
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::Conflict(_) => StatusCode::CONFLICT,
            ApiError::Gone(_) => StatusCode::GONE,
            ApiError::UnprocessableEntity(_) => StatusCode::UNPROCESSABLE_ENTITY,
            ApiError::TooManyRequests(_) => StatusCode::TOO_MANY_REQUESTS,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Validation(_) => StatusCode::BAD_REQUEST,
            ApiError::Business(_) => StatusCode::UNPROCESSABLE_ENTITY,
        }
    }

    pub fn code(&self) -> &str {
        match self {
            ApiError::BadRequest(_) => "BAD_REQUEST",
            ApiError::Unauthorized(_) => "UNAUTHORIZED",
            ApiError::Forbidden(_) => "FORBIDDEN",
            ApiError::NotFound(_) => "NOT_FOUND",
            ApiError::Conflict(_) => "CONFLICT",
            ApiError::Gone(_) => "GONE",
            ApiError::UnprocessableEntity(_) => "UNPROCESSABLE_ENTITY",
            ApiError::TooManyRequests(_) => "TOO_MANY_REQUESTS",
            ApiError::Internal(_) => "INTERNAL_ERROR",
            ApiError::Validation(_) => "VALIDATION_ERROR",
            ApiError::Business(_) => "BUSINESS_ERROR",
        }
    }

    pub fn message(&self) -> &str {
        match self {
            ApiError::BadRequest(m) => m,
            ApiError::Unauthorized(m) => m,
            ApiError::Forbidden(m) => m,
            ApiError::NotFound(m) => m,
            ApiError::Conflict(m) => m,
            ApiError::Gone(m) => m,
            ApiError::UnprocessableEntity(m) => m,
            ApiError::TooManyRequests(m) => m,
            ApiError::Internal(m) => m,
            ApiError::Validation(m) => m,
            ApiError::Business(m) => m,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = ErrorResponse {
            error: ErrorBody {
                code: self.code().to_string(),
                message: self.message().to_string(),
            },
        };

        (self.status_code(), Json(body)).into_response()
    }
}

impl From<ramp_common::Error> for ApiError {
    fn from(err: ramp_common::Error) -> Self {
        match &err {
            ramp_common::Error::IntentNotFound(_) => ApiError::NotFound(err.to_string()),
            ramp_common::Error::InvalidStateTransition { .. } => {
                ApiError::Conflict(err.to_string())
            }
            ramp_common::Error::IntentExpired(_) => ApiError::Gone(err.to_string()),
            ramp_common::Error::DuplicateIntent(_) => ApiError::Conflict(err.to_string()),
            ramp_common::Error::TenantNotFound(_) => ApiError::NotFound(err.to_string()),
            ramp_common::Error::TenantSuspended(_) => ApiError::Forbidden(err.to_string()),
            ramp_common::Error::UserNotFound(_) => ApiError::NotFound(err.to_string()),
            ramp_common::Error::UserKycNotVerified(_) => ApiError::Forbidden(err.to_string()),
            ramp_common::Error::UserLimitExceeded { .. } => ApiError::Forbidden(err.to_string()),
            ramp_common::Error::InsufficientBalance { .. } => {
                ApiError::UnprocessableEntity(err.to_string())
            }
            ramp_common::Error::AmlCheckFailed { .. } => ApiError::Forbidden(err.to_string()),
            ramp_common::Error::WebhookSignatureInvalid => ApiError::Unauthorized(err.to_string()),
            ramp_common::Error::Validation(_) => ApiError::BadRequest(err.to_string()),
            ramp_common::Error::Database(_) => ApiError::Internal(err.to_string()),
            ramp_common::Error::Internal(_) => ApiError::Internal(err.to_string()),
            _ => ApiError::Internal(err.to_string()),
        }
    }
}
