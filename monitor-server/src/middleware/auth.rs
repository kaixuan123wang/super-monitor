//! JWT 鉴权中间件。

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::router::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentUser {
    pub id: i32,
    pub username: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Claims {
    sub: i32,
    username: String,
    role: String,
    token_type: String,
    exp: usize,
}

pub async fn require_auth(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> AppResult<Response> {
    let token = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(AppError::Unauthorized)?;

    let claims = decode::<Claims>(
        token,
        &DecodingKey::from_secret(state.config.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| AppError::Unauthorized)?
    .claims;

    if claims.token_type != "access" {
        return Err(AppError::Unauthorized);
    }

    req.extensions_mut().insert(CurrentUser {
        id: claims.sub,
        username: claims.username,
        role: claims.role,
    });

    Ok(next.run(req).await)
}
