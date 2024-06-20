use crate::response::api_response::ApiErrorResponse;
use async_trait::async_trait;
use axum::extract::{rejection::JsonRejection, FromRequest};
use axum::response::{IntoResponse, Response};
use axum::{body::HttpBody, extract::Request, BoxError, Json};
use serde::de::DeserializeOwned;
use thiserror::Error;
use validator::Validate;

use super::error_code;

#[derive(Debug, Error)]
pub enum RequestError {
	// 通用错误
	#[error("common request error:{0}")]
    CommonError(String),
    #[error(transparent)]
    ValidationError(#[from] validator::ValidationErrors),
    #[error(transparent)]
    JsonRejection(#[from] JsonRejection),
    #[error("signature error")]
    SignatureError,
}

impl RequestError {
    fn get_code(&self) -> u32 {
        match self {
            RequestError::CommonError(_) => error_code::COMMON_REQUEST_ERROR,
            RequestError::ValidationError(_) => error_code::VALIDATION_ERROR,
            RequestError::JsonRejection(_) => error_code::JSON_REJECTION,
            RequestError::SignatureError => error_code::SIGNATURE_ERROR,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatedRequest<T>(pub T);

#[async_trait]
impl<T, S> FromRequest<S> for ValidatedRequest<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = RequestError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state).await?;
        value.validate()?;
        Ok(ValidatedRequest(value))
    }
}

impl IntoResponse for RequestError {
    fn into_response(self) -> Response {
        match self {
            RequestError::ValidationError(_) => ApiErrorResponse::send(
                400,
                self.get_code(),
                Some(self.to_string().replace('\n', ", ")),
            ),
            RequestError::JsonRejection(_) => {
                ApiErrorResponse::send(400, self.get_code(), Some(self.to_string()))
            }
            RequestError::SignatureError => {
                ApiErrorResponse::send(400, self.get_code(), Some(self.to_string()))
            },
			RequestError::CommonError(_) => {
                ApiErrorResponse::send(400, self.get_code(), Some(self.to_string()))
            }
        }
    }
}
