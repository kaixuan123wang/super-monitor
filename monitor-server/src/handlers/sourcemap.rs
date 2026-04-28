//! Source Map 上传 / 列表 / 删除接口。

use axum::{
    body::Bytes,
    extract::{Extension, Multipart, Path, Query, State},
    Json,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::{AppError, AppResult};
use crate::middleware::auth::{check_project_access, CurrentUser};
use crate::models;
use crate::router::AppState;
use crate::services::sourcemap_service;
use crate::utils::{get_db, now_fixed};

// ── 上传 ────────────────────────────────────────────────────────────────────

pub async fn upload(
    State(state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
    mut multipart: Multipart,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    let base_dir = &state.config.sourcemap_dir;

    let mut project_id: Option<i32> = None;
    let mut release: Option<String> = None;
    let mut filename: Option<String> = None;
    let mut file_data: Option<Vec<u8>> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        let name = field.name().map(|s| s.to_string());
        match name.as_deref() {
            Some("project_id") => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::BadRequest(e.to_string()))?;
                project_id = text.parse::<i32>().ok();
            }
            Some("release") => {
                release = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| AppError::BadRequest(e.to_string()))?,
                );
            }
            Some("file") => {
                filename = field.file_name().map(|s| s.to_string());
                let bytes: Bytes = field
                    .bytes()
                    .await
                    .map_err(|e| AppError::BadRequest(e.to_string()))?;
                file_data = Some(bytes.to_vec());
            }
            _ => {}
        }
    }

    let project_id = project_id.ok_or_else(|| AppError::BadRequest("missing project_id".into()))?;
    check_project_access(db, &current_user, project_id).await?;
    let release = release.ok_or_else(|| AppError::BadRequest("missing release".into()))?;
    let filename = filename.ok_or_else(|| AppError::BadRequest("missing file".into()))?;
    let data = file_data.ok_or_else(|| AppError::BadRequest("missing file data".into()))?;

    // 输入校验：防路径穿越 + 长度限制
    if release.len() > 128
        || release.contains("..")
        || release.contains('/')
        || release.contains('\\')
    {
        return Err(AppError::BadRequest("invalid release format".into()));
    }
    if filename.len() > 255 {
        return Err(AppError::BadRequest("filename too long".into()));
    }
    // 限制文件大小（50MB 由 body limit 保证，此处额外校验 10MB）
    if data.len() > 10 * 1024 * 1024 {
        return Err(AppError::BadRequest("sourcemap file too large (max 10MB)".into()));
    }

    // 校验是否有效的 Source Map
    if sourcemap::SourceMap::from_reader(data.as_slice()).is_err() {
        return Err(AppError::BadRequest("invalid source map file".into()));
    }

    let content_hash = sourcemap_service::md5_hex(&data);
    let file_size = data.len() as i64;
    let path = sourcemap_service::storage_path(base_dir, project_id, &release, &filename);
    sourcemap_service::write_file(&path, &data).await?;

    let am = models::source_map::ActiveModel {
        id: sea_orm::NotSet,
        project_id: Set(project_id),
        release: Set(release.clone()),
        filename: Set(filename.clone()),
        file_size: Set(Some(file_size)),
        storage_path: Set(path.to_string_lossy().to_string()),
        content_hash: Set(Some(content_hash)),
        uploaded_at: Set(now_fixed()),
    };
    let saved = am.insert(db).await?;

    Ok(Json(json!({
        "code": 0, "message": "ok",
        "data": {
            "id": saved.id,
            "filename": saved.filename,
            "release": saved.release,
            "uploaded_at": saved.uploaded_at,
        }
    })))
}

// ── 列表 ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub project_id: i32,
    pub release: Option<String>,
    #[serde(default = "default_page")]
    pub page: u64,
    #[serde(default = "default_page_size")]
    pub page_size: u64,
}

fn default_page() -> u64 {
    1
}
fn default_page_size() -> u64 {
    20
}

pub async fn list(
    State(state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    check_project_access(db, &current_user, q.project_id).await?;

    let mut query =
        models::SourceMap::find().filter(models::source_map::Column::ProjectId.eq(q.project_id));
    if let Some(rel) = &q.release {
        query = query.filter(models::source_map::Column::Release.eq(rel.as_str()));
    }

    let page_size = q.page_size.clamp(1, 100);
    let total = query.clone().count(db).await?;
    let items = query
        .order_by_desc(models::source_map::Column::UploadedAt)
        .paginate(db, page_size)
        .fetch_page(q.page.saturating_sub(1))
        .await?;

    Ok(Json(json!({
        "code": 0, "message": "ok",
        "data": { "list": items, "total": total }
    })))
}

// ── 详情 ────────────────────────────────────────────────────────────────────

pub async fn detail(
    State(state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i32>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    let row = models::SourceMap::find_by_id(id)
        .one(db)
        .await?
        .ok_or(AppError::NotFound)?;
    check_project_access(db, &current_user, row.project_id).await?;

    Ok(Json(json!({ "code": 0, "message": "ok", "data": row })))
}

// ── 删除 ────────────────────────────────────────────────────────────────────

pub async fn remove(
    State(state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i32>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    let row = models::SourceMap::find_by_id(id)
        .one(db)
        .await?
        .ok_or(AppError::NotFound)?;
    check_project_access(db, &current_user, row.project_id).await?;

    sourcemap_service::delete_file(
        &state.config.sourcemap_dir,
        std::path::Path::new(&row.storage_path),
    )
    .await?;
    models::SourceMap::delete_by_id(id).exec(db).await?;

    Ok(Json(json!({ "code": 0, "message": "ok", "data": { "deleted": 1 } })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_query_defaults() {
        let q: ListQuery = serde_json::from_str(r#"{"project_id":1}"#).unwrap();
        assert_eq!(q.project_id, 1);
        assert!(q.release.is_none());
        assert_eq!(q.page, 1);
        assert_eq!(q.page_size, 20);
    }

    #[test]
    fn test_list_query_with_release() {
        let q: ListQuery = serde_json::from_str(r#"{"project_id":1,"release":"v1.0"}"#).unwrap();
        assert_eq!(q.release.as_deref(), Some("v1.0"));
    }

    #[test]
    fn test_now_fixed_is_utc() {
        let now = now_fixed();
        assert_eq!(now.offset().local_minus_utc(), 0);
    }
}
