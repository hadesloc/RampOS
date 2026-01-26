use axum::{
    async_trait,
    extract::{FromRequest, Request},
    Json,
};
use validator::Validate;
use crate::error::ApiError;

/// A wrapper around `axum::Json` that validates the request body.
pub struct ValidatedJson<T>(pub T);

#[async_trait]
impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: Validate + 'static,
    Json<T>: FromRequest<S, Rejection = axum::extract::rejection::JsonRejection>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(|err| ApiError::BadRequest(err.to_string()))?;

        value.validate().map_err(|err| ApiError::BadRequest(err.to_string()))?;

        Ok(ValidatedJson(value))
    }
}

#[cfg(test)]
#[path = "extract_test.rs"]
mod tests;
