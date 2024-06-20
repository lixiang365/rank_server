use crate::response::api_response::ApiErrorResponse;
use crate::error::error_code;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum UserError {
    #[error("User not found")]
    UserNotFound,
    #[error("User already exists")]
    UserAlreadyExists,
    #[error("Invalid password")]
    InvalidPassword,
}

impl UserError {
    fn get_code(&self) -> u32 {
        match self {
            UserError::UserNotFound => error_code::USER_NOT_FOUND,
            UserError::UserAlreadyExists => error_code::USER_ALREADY_EXISTS,
            UserError::InvalidPassword => error_code::INVALID_PASSWORD,
        }
    }
}

impl IntoResponse for UserError {
    fn into_response(self) -> Response {
        let status_code = match self {
            UserError::UserNotFound => StatusCode::NOT_FOUND,
            UserError::UserAlreadyExists => StatusCode::BAD_REQUEST,
            UserError::InvalidPassword => StatusCode::BAD_REQUEST,
        };

        ApiErrorResponse::send(
            status_code.as_u16(),
            self.get_code(),
            Some(self.to_string()),
        )
    }
}
