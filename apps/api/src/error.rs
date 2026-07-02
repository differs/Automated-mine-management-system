use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

#[derive(Serialize)]
struct ErrorBody<'a> {
    code: &'a str,
    message: &'a str,
}

impl ApiError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "bad_request",
            message: message.into(),
        }
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            code: "unauthorized",
            message: message.into(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: "not_found",
            message: message.into(),
        }
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            code: "conflict",
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "internal_error",
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorBody {
                code: self.code,
                message: &self.message,
            }),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_status_codes() {
        assert_eq!(ApiError::bad_request("x").status, StatusCode::BAD_REQUEST);
        assert_eq!(ApiError::unauthorized("x").status, StatusCode::UNAUTHORIZED);
        assert_eq!(ApiError::not_found("x").status, StatusCode::NOT_FOUND);
        assert_eq!(ApiError::conflict("x").status, StatusCode::CONFLICT);
        assert_eq!(ApiError::internal("x").status, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(ApiError::bad_request("x").code, "bad_request");
        assert_eq!(ApiError::unauthorized("x").code, "unauthorized");
        assert_eq!(ApiError::not_found("x").code, "not_found");
        assert_eq!(ApiError::conflict("x").code, "conflict");
        assert_eq!(ApiError::internal("x").code, "internal_error");
    }

    #[test]
    fn test_error_messages() {
        let err = ApiError::bad_request("invalid input");
        assert_eq!(err.message, "invalid input");
    }
}
