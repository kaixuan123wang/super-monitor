//! 登录 / 注册 / Token 刷新。

use axum::{extract::State, Json};
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{Duration, FixedOffset, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, PaginatorTrait,
    QueryFilter, Set,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::{AppError, AppResult};
use crate::models;
use crate::router::AppState;

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

fn now_fixed() -> chrono::DateTime<FixedOffset> {
    Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap())
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
    if username.is_empty() || email.is_empty() || body.password.len() < 6 {
        return Err(AppError::BadRequest(
            "username/email are required and password must be at least 6 chars".into(),
        ));
    }

    let is_first_user = models::User::find().count(db).await? == 0;
    let password_hash = hash(&body.password, DEFAULT_COST)
        .map_err(|e| AppError::Internal(format!("password hash failed: {e}")))?;

    let user = models::user::ActiveModel {
        id: sea_orm::NotSet,
        username: Set(username.to_string()),
        email: Set(email.to_string()),
        password_hash: Set(password_hash),
        role: Set(if is_first_user { "super_admin" } else { "member" }.into()),
        group_id: Set(None),
        avatar: Set(None),
        last_login_at: Set(Some(now_fixed())),
        created_at: Set(now_fixed()),
        updated_at: Set(now_fixed()),
    }
    .insert(db)
    .await?;

    let mut user = user;
    if is_first_user || body.group_name.as_ref().is_some_and(|s| !s.trim().is_empty()) {
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
        .insert(db)
        .await?;
        let mut am: models::user::ActiveModel = user.into();
        am.group_id = Set(Some(group.id));
        am.updated_at = Set(now_fixed());
        user = am.update(db).await?;
    }

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
