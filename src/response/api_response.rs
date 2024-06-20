use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ApiSuccessResponse<T: Serialize> {
    code: u16,
    msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ApiErrorResponse {
    code: u32,
    msg: Option<String>,
    #[serde(skip)]
    status: u16,
}

impl<T: Serialize> ApiSuccessResponse<T>
where
    T: Serialize,
{
    pub(crate) fn send(data: T) -> Self {
        return ApiSuccessResponse {
            code: 0,
            msg: "success".to_string(),
            data: Some(data),
        };
    }

    pub fn from_with_nodata() -> Self {
        return ApiSuccessResponse {
            code: 0,
            msg: "success".to_string(),
            data: None,
        };
    }
}

impl ApiErrorResponse {
    pub(crate) fn new(status: u16, code: u32, msg: Option<String>) -> Self {
        return Self { code, msg, status };
    }

    pub(crate) fn send(status: u16, code: u32, msg: Option<String>) -> Response {
        return ApiErrorResponse { code, msg, status }.into_response();
    }

    fn get_status_code(&self) -> StatusCode {
        match self.code {
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for ApiErrorResponse {
    fn into_response(self) -> Response {
        (StatusCode::from_u16(self.status).unwrap(), Json(self)).into_response()
    }
}
