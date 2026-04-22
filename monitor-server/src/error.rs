use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use thiserror::Error;

/// 应用错误类型。
#[derive(Debug, Error)]
pub enum AppError {
    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("unauthorized")]
    Unauthorized,

    #[error("forbidden")]
    Forbidden,

    #[error("not found")]
    NotFound,

    #[error("internal error: {0}")]
    Internal(String),

    #[error("database error: {0}")]
    Database(#[from] sea_orm::DbErr),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, code) = match &self {
            AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, 400),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, 401),
            AppError::Forbidden => (StatusCode::FORBIDDEN, 403),
            AppError::NotFound => (StatusCode::NOT_FOUND, 404),
            AppError::Internal(_) | AppError::Database(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, 500)
            }
        };
        let body = Json(json!({
            "code": code,
            "message": self.to_string(),
            "data": null,
        }));
        (status, body).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
