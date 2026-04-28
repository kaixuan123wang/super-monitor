//! JWT 鉴权中间件 + 项目级权限检查。

use async_trait::async_trait;
use axum::{
    extract::{FromRequestParts, Request, State},
    http::request::Parts,
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use sea_orm::{DatabaseConnection, EntityTrait};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::models;
use crate::router::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentUser {
    pub id: i32,
    pub username: String,
    pub role: String,
}

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for CurrentUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<CurrentUser>()
            .cloned()
            .ok_or(AppError::Unauthorized)
    }
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

    let mut validation = Validation::new(Algorithm::HS256);
    validation.leeway = 60;
    let claims = decode::<Claims>(
        token,
        &DecodingKey::from_secret(state.config.jwt_secret.as_bytes()),
        &validation,
    )
    .map_err(|_| AppError::Unauthorized)?
    .claims;

    if claims.token_type != "access" {
        return Err(AppError::Unauthorized);
    }

    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::Internal("database not connected".into()))?;
    let user = models::User::find_by_id(claims.sub)
        .one(db)
        .await?
        .ok_or(AppError::Unauthorized)?;

    req.extensions_mut().insert(CurrentUser {
        id: user.id,
        username: user.username,
        role: user.role,
    });

    Ok(next.run(req).await)
}

/// 检查当前用户是否有权访问指定 project_id 的项目。
/// - super_admin 可访问所有项目
/// - 其他用户只能访问自己为 owner 或同 group 的项目
pub async fn check_project_access(
    db: &DatabaseConnection,
    current_user: &CurrentUser,
    project_id: i32,
) -> AppResult<()> {
    // super_admin 可访问所有项目
    if current_user.role == "super_admin" {
        return Ok(());
    }

    let project = models::Project::find_by_id(project_id)
        .one(db)
        .await?
        .ok_or(AppError::NotFound)?;

    // 是项目 owner
    if project.owner_id == current_user.id {
        return Ok(());
    }

    // 同 group
    let user = models::User::find_by_id(current_user.id)
        .one(db)
        .await?
        .ok_or(AppError::Unauthorized)?;

    if let Some(user_gid) = user.group_id {
        if user_gid == project.group_id {
            return Ok(());
        }
    }

    Err(AppError::Forbidden)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_user_serialization() {
        let user = CurrentUser { id: 1, username: "admin".into(), role: "super_admin".into() };
        let json = serde_json::to_string(&user).unwrap();
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"username\":\"admin\""));
        assert!(json.contains("\"role\":\"super_admin\""));
    }

    #[test]
    fn test_current_user_deserialization() {
        let json = r#"{"id":42,"username":"test","role":"member"}"#;
        let user: CurrentUser = serde_json::from_str(json).unwrap();
        assert_eq!(user.id, 42);
        assert_eq!(user.username, "test");
        assert_eq!(user.role, "member");
    }

    #[test]
    fn test_claims_serialization_roundtrip() {
        let claims = Claims {
            sub: 1,
            username: "admin".into(),
            role: "super_admin".into(),
            token_type: "access".into(),
            exp: 9999999999,
        };
        let json = serde_json::to_string(&claims).unwrap();
        let decoded: Claims = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.sub, 1);
        assert_eq!(decoded.token_type, "access");
    }

    #[test]
    fn test_jwt_encode_decode() {
        use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};

        let claims = Claims {
            sub: 123,
            username: "user1".into(),
            role: "member".into(),
            token_type: "access".into(),
            exp: 9999999999,
        };

        let secret = "test_secret_key";
        let token =
            encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
                .unwrap();
        assert!(!token.is_empty());

        let decoded = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::new(Algorithm::HS256),
        )
        .unwrap();
        assert_eq!(decoded.claims.sub, 123);
        assert_eq!(decoded.claims.username, "user1");
    }

    #[test]
    fn test_jwt_wrong_secret_fails() {
        use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};

        let claims = Claims {
            sub: 1,
            username: "user".into(),
            role: "member".into(),
            token_type: "access".into(),
            exp: 9999999999,
        };

        let token =
            encode(&Header::default(), &claims, &EncodingKey::from_secret("secret1".as_bytes()))
                .unwrap();
        let result = decode::<Claims>(
            &token,
            &DecodingKey::from_secret("secret2".as_bytes()),
            &Validation::new(Algorithm::HS256),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_jwt_refresh_token_type() {
        use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};

        let claims = Claims {
            sub: 1,
            username: "user".into(),
            role: "member".into(),
            token_type: "refresh".into(),
            exp: 9999999999,
        };

        let secret = "test_secret";
        let token =
            encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
                .unwrap();
        let decoded = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::new(Algorithm::HS256),
        )
        .unwrap();
        assert_eq!(decoded.claims.token_type, "refresh");
    }
}
