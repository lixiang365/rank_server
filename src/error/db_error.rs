use crate::error::error_code;
use crate::response::api_response::ApiErrorResponse;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("{0}")]
    SomethingWentWrong(String),
    #[error("Duplicate entry exists")]
    UniqueConstraintViolation(String),
}

impl DbError {
    fn get_code(&self) -> u32 {
        match self {
            DbError::SomethingWentWrong(_) => error_code::SOMETHING_WENT_WRONG,
            DbError::UniqueConstraintViolation(_) => error_code::UNIQUE_CONSTRAINT_VIOLATION,
        }
    }
}

impl IntoResponse for DbError {
    fn into_response(self) -> Response {
        let status_code = match self {
            DbError::SomethingWentWrong(_) => StatusCode::INTERNAL_SERVER_ERROR,
            DbError::UniqueConstraintViolation(_) => StatusCode::CONFLICT,
        };

        ApiErrorResponse::send(
            status_code.as_u16(),
            self.get_code(),
            Some(self.to_string()),
        )
    }
}
