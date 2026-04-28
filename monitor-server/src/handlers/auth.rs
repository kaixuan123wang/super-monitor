//! 登录 / 注册 / Token 刷新。

use axum::{extract::State, Json};
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, PaginatorTrait,
    QueryFilter, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::{AppError, AppResult};
use crate::models;
use crate::router::AppState;
use crate::utils::now_fixed;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Claims {
    sub: i32,
    username: String,
    role: String,
    token_type: String,
    exp: usize,
}

#[derive(Debug, Deserialize)]
pub struct RegisterBody {
    pub username: String,
    pub email: String,
    pub password: String,
    pub group_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginBody {
    pub account: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct RefreshBody {
    pub refresh_token: String,
}

fn get_db(state: &AppState) -> AppResult<&DatabaseConnection> {
    state
        .db
        .as_ref()
        .ok_or_else(|| AppError::Internal("database not connected".into()))
}

pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterBody>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    let username = body.username.trim();
    let email = body.email.trim();
    if username.is_empty() || email.is_empty() || body.password.len() < 12 {
        return Err(AppError::BadRequest(
            "username/email are required and password must be at least 12 chars".into(),
        ));
    }
    if username.len() > 64 {
        return Err(AppError::BadRequest("username must be at most 64 characters".into()));
    }
    if email.len() > 254 {
        return Err(AppError::BadRequest("email must be at most 254 characters".into()));
    }

    // 检查用户名/邮箱是否已存在
    let existing = models::User::find()
        .filter(
            Condition::any()
                .add(models::user::Column::Username.eq(username))
                .add(models::user::Column::Email.eq(email)),
        )
        .one(db)
        .await?;
    if existing.is_some() {
        return Err(AppError::BadRequest("username or email already exists".into()));
    }

    let password_hash = hash(&body.password, DEFAULT_COST)
        .map_err(|e| AppError::Internal(format!("password hash failed: {e}")))?;

    // 使用事务 + advisory lock 防止并发注册竞态（避免多个 super_admin）
    let txn = db.begin().await?;

    // 使用 PostgreSQL advisory lock 串行化注册流程
    use sea_orm::ConnectionTrait;
    txn.execute(sea_orm::Statement::from_string(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT pg_advisory_xact_lock(8432910)".to_string(),
    ))
    .await?;

    // 在锁保护下检查是否为首个用户
    let user_count = models::User::find().count(&txn).await?;
    let is_first_user = user_count == 0;

    let user = models::user::ActiveModel {
        id: sea_orm::NotSet,
        username: Set(username.to_string()),
        email: Set(email.to_string()),
        password_hash: Set(password_hash),
        role: Set(if is_first_user {
            "super_admin"
        } else {
            "member"
        }
        .into()),
        group_id: Set(None),
        avatar: Set(None),
        last_login_at: Set(Some(now_fixed())),
        created_at: Set(now_fixed()),
        updated_at: Set(now_fixed()),
    }
    .insert(&txn)
    .await
    .map_err(|e| {
        if e.to_string().contains("duplicate") || e.to_string().contains("unique") {
            AppError::BadRequest("username or email already exists".into())
        } else {
            AppError::Database(e)
        }
    })?;

    let mut user = user;
    if is_first_user
        || body
            .group_name
            .as_ref()
            .is_some_and(|s| !s.trim().is_empty())
    {
        let group = models::group::ActiveModel {
            id: sea_orm::NotSet,
            name: Set(body
                .group_name
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "Default".into())),
            description: Set(None),
            owner_id: Set(user.id),
            created_at: Set(now_fixed()),
            updated_at: Set(now_fixed()),
        }
        .insert(&txn)
        .await?;
        let mut am: models::user::ActiveModel = user.into();
        am.group_id = Set(Some(group.id));
        am.updated_at = Set(now_fixed());
        user = am.update(&txn).await?;
    }

    txn.commit().await?;

    Ok(Json(json!({
        "code": 0,
        "message": "ok",
        "data": build_auth_response(&state, &user)?
    })))
}

pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginBody>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    let account = body.account.trim();
    let user = models::User::find()
        .filter(
            Condition::any()
                .add(models::user::Column::Username.eq(account))
                .add(models::user::Column::Email.eq(account)),
        )
        .one(db)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let ok = verify(&body.password, &user.password_hash)
        .map_err(|e| AppError::Internal(format!("password verify failed: {e}")))?;
    if !ok {
        return Err(AppError::Unauthorized);
    }

    let mut am: models::user::ActiveModel = user.into();
    am.last_login_at = Set(Some(now_fixed()));
    am.updated_at = Set(now_fixed());
    let user = am.update(db).await?;

    Ok(Json(json!({
        "code": 0,
        "message": "ok",
        "data": build_auth_response(&state, &user)?
    })))
}

pub async fn refresh(
    State(state): State<AppState>,
    Json(body): Json<RefreshBody>,
) -> AppResult<Json<Value>> {
    let token = decode::<Claims>(
        &body.refresh_token,
        &DecodingKey::from_secret(state.config.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| AppError::Unauthorized)?;
    if token.claims.token_type != "refresh" {
        return Err(AppError::Unauthorized);
    }

    let db = get_db(&state)?;
    let user = models::User::find_by_id(token.claims.sub)
        .one(db)
        .await?
        .ok_or(AppError::Unauthorized)?;

    Ok(Json(json!({
        "code": 0,
        "message": "ok",
        "data": build_auth_response(&state, &user)?
    })))
}

/// 生成短生命周期的 SSE token（60s），避免将 access_token 泄漏到 URL。
/// POST /api/auth/sse-token
pub async fn sse_token(
    State(state): State<AppState>,
    req: axum::extract::Request,
) -> AppResult<Json<Value>> {
    // 从 Authorization header 提取并验证 access_token
    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(AppError::Unauthorized)?;

    let claims = decode::<Claims>(
        auth_header,
        &DecodingKey::from_secret(state.config.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| AppError::Unauthorized)?;

    if claims.claims.token_type != "access" {
        return Err(AppError::Unauthorized);
    }

    // 生成短生命周期 token
    let sse_token = uuid::Uuid::new_v4().simple().to_string();

    // 存入 Redis（60s TTL）。Redis 不可用时不能签发可用的 SSE token。
    let redis = state
        .redis
        .as_ref()
        .ok_or_else(|| AppError::Internal("SSE authentication service unavailable".into()))?;
    let key = format!("sse_token:{}", sse_token);
    let value = claims.claims.sub.to_string();
    let mut conn = redis.clone();
    redis::cmd("SETEX")
        .arg(&key)
        .arg(60)
        .arg(&value)
        .query_async::<_, ()>(&mut conn)
        .await
        .map_err(|e| AppError::Internal(format!("SSE token store failed: {e}")))?;

    Ok(Json(json!({
        "code": 0,
        "message": "ok",
        "data": { "token": sse_token, "expires_in": 60 }
    })))
}

fn build_auth_response(state: &AppState, user: &models::user::Model) -> AppResult<Value> {
    let access_token = sign_token(state, user, "access", Duration::hours(2))?;
    let refresh_token = sign_token(state, user, "refresh", Duration::days(7))?;
    Ok(json!({
        "access_token": access_token,
        "refresh_token": refresh_token,
        "token_type": "Bearer",
        "expires_in": 7200,
        "user": user,
    }))
}

fn sign_token(
    state: &AppState,
    user: &models::user::Model,
    token_type: &str,
    ttl: Duration,
) -> AppResult<String> {
    let claims = Claims {
        sub: user.id,
        username: user.username.clone(),
        role: user.role.clone(),
        token_type: token_type.into(),
        exp: (Utc::now() + ttl).timestamp() as usize,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.config.jwt_secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(format!("jwt encode failed: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_body_deserialize() {
        let json = r#"{"username":"test","email":"test@test.com","password":"123456"}"#;
        let body: RegisterBody = serde_json::from_str(json).unwrap();
        assert_eq!(body.username, "test");
        assert_eq!(body.email, "test@test.com");
        assert_eq!(body.password, "123456");
        assert!(body.group_name.is_none());
    }

    #[test]
    fn test_register_body_with_group() {
        let json =
            r#"{"username":"test","email":"t@t.com","password":"123456","group_name":"MyGroup"}"#;
        let body: RegisterBody = serde_json::from_str(json).unwrap();
        assert_eq!(body.group_name.as_deref(), Some("MyGroup"));
    }

    #[test]
    fn test_login_body_deserialize() {
        let json = r#"{"account":"admin","password":"secret"}"#;
        let body: LoginBody = serde_json::from_str(json).unwrap();
        assert_eq!(body.account, "admin");
        assert_eq!(body.password, "secret");
    }

    #[test]
    fn test_login_body_with_email() {
        let json = r#"{"account":"admin@test.com","password":"secret"}"#;
        let body: LoginBody = serde_json::from_str(json).unwrap();
        assert_eq!(body.account, "admin@test.com");
    }

    #[test]
    fn test_refresh_body_deserialize() {
        let json = r#"{"refresh_token":"eyJhbGciOiJIUzI1NiJ9.test"}"#;
        let body: RefreshBody = serde_json::from_str(json).unwrap();
        assert_eq!(body.refresh_token, "eyJhbGciOiJIUzI1NiJ9.test");
    }

    #[test]
    fn test_now_fixed_is_utc() {
        let now = now_fixed();
        assert_eq!(now.offset().local_minus_utc(), 0);
    }

    #[test]
    fn test_claims_serialization() {
        let claims = Claims {
            sub: 1,
            username: "admin".into(),
            role: "super_admin".into(),
            token_type: "access".into(),
            exp: 9999999999,
        };
        let json = serde_json::to_string(&claims).unwrap();
        assert!(json.contains("\"sub\":1"));
        assert!(json.contains("\"token_type\":\"access\""));
    }

    #[test]
    fn test_jwt_encode_decode_roundtrip() {
        let claims = Claims {
            sub: 42,
            username: "user1".into(),
            role: "member".into(),
            token_type: "access".into(),
            exp: 9999999999,
        };
        let secret = "test_secret";
        let token =
            encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
                .unwrap();
        let decoded = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        )
        .unwrap();
        assert_eq!(decoded.claims.sub, 42);
        assert_eq!(decoded.claims.username, "user1");
        assert_eq!(decoded.claims.token_type, "access");
    }

    #[test]
    fn test_jwt_refresh_token_type() {
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
            &Validation::default(),
        )
        .unwrap();
        assert_eq!(decoded.claims.token_type, "refresh");
    }
}
