use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use thiserror::Error;
use tracing::error;

/// 应用错误类型。
#[derive(Debug, Error)]
pub enum AppError {
    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("unauthorized")]
    Unauthorized,

    #[error("forbidden")]
    Forbidden,

    #[error("too many requests: {0}")]
    TooManyRequests(String),

    #[error("not found")]
    NotFound,

    #[error("internal error: {0}")]
    Internal(String),

    #[error("database error: {0}")]
    Database(#[from] sea_orm::DbErr),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, code, message) = match &self {
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, 400, msg.clone()),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, 401, "unauthorized".to_string()),
            AppError::Forbidden => (StatusCode::FORBIDDEN, 403, "forbidden".to_string()),
            AppError::TooManyRequests(msg) => (StatusCode::TOO_MANY_REQUESTS, 429, msg.clone()),
            AppError::NotFound => (StatusCode::NOT_FOUND, 404, "not found".to_string()),
            AppError::Internal(msg) => {
                // 记录详细错误到日志，但不暴露给客户端
                error!("Internal error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, 500, "internal server error".to_string())
            }
            AppError::Database(err) => {
                // 记录详细数据库错误到日志，但不暴露给客户端
                error!("Database error: {}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, 500, "internal server error".to_string())
            }
        };
        let body = Json(json!({
            "code": code,
            "message": message,
            "data": null,
        }));
        (status, body).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bad_request_display() {
        let err = AppError::BadRequest("invalid input".into());
        assert_eq!(err.to_string(), "bad request: invalid input");
    }

    #[test]
    fn test_unauthorized_display() {
        let err = AppError::Unauthorized;
        assert_eq!(err.to_string(), "unauthorized");
    }

    #[test]
    fn test_forbidden_display() {
        let err = AppError::Forbidden;
        assert_eq!(err.to_string(), "forbidden");
    }

    #[test]
    fn test_too_many_requests_display() {
        let err = AppError::TooManyRequests("slow down".into());
        assert_eq!(err.to_string(), "too many requests: slow down");
    }

    #[test]
    fn test_not_found_display() {
        let err = AppError::NotFound;
        assert_eq!(err.to_string(), "not found");
    }

    #[test]
    fn test_internal_display() {
        let err = AppError::Internal("something broke".into());
        assert_eq!(err.to_string(), "internal error: something broke");
    }

    #[test]
    fn test_bad_request_into_response() {
        let err = AppError::BadRequest("bad".into());
        let resp = err.into_response();
        assert_eq!(resp.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_unauthorized_into_response() {
        let err = AppError::Unauthorized;
        let resp = err.into_response();
        assert_eq!(resp.status(), axum::http::StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_forbidden_into_response() {
        let err = AppError::Forbidden;
        let resp = err.into_response();
        assert_eq!(resp.status(), axum::http::StatusCode::FORBIDDEN);
    }

    #[test]
    fn test_too_many_requests_into_response() {
        let err = AppError::TooManyRequests("rate limited".into());
        let resp = err.into_response();
        assert_eq!(resp.status(), axum::http::StatusCode::TOO_MANY_REQUESTS);
    }

    #[test]
    fn test_not_found_into_response() {
        let err = AppError::NotFound;
        let resp = err.into_response();
        assert_eq!(resp.status(), axum::http::StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_internal_into_response() {
        let err = AppError::Internal("boom".into());
        let resp = err.into_response();
        assert_eq!(resp.status(), axum::http::StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_app_result_ok() {
        let r: AppResult<i32> = Ok(42);
        assert_eq!(r.ok(), Some(42));
    }

    #[test]
    fn test_app_result_err() {
        let r: AppResult<i32> = Err(AppError::NotFound);
        assert!(r.is_err());
    }
}
