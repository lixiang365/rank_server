use crate::response::api_response::ApiErrorResponse;
use super::error_code;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;


#[derive(Error, Debug)]
pub enum TokenError {
    #[error("Invalid token")]
    InvalidToken(String),
    #[error("Token has expired")]
    TokenExpired,
    #[error("Missing Bearer token")]
    MissingToken,
    #[error("Token error: {0}")]
    TokenCreationError(String),
}

impl TokenError {
    fn get_code(&self) -> u32 {
        match self {
            TokenError::InvalidToken(_) => error_code::INVALID_TOKEN,
            TokenError::TokenExpired => error_code::TOKEN_EXPIRED,
            TokenError::MissingToken => error_code::MISSING_TOKEN,
            TokenError::TokenCreationError(_) => error_code::TOKEN_CREATION_ERROR,
        }
    }
}

impl IntoResponse for TokenError {
    fn into_response(self) -> Response {
        let status_code = match self {
            TokenError::InvalidToken(_) => StatusCode::UNAUTHORIZED,
            TokenError::TokenExpired => StatusCode::UNAUTHORIZED,
            TokenError::MissingToken => StatusCode::UNAUTHORIZED,
            TokenError::TokenCreationError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        ApiErrorResponse::send(
            status_code.as_u16(),
            self.get_code(),
            Some(self.to_string()),
        )
    }
}
