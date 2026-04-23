//! Source Map 文件存储与堆栈解析服务。

use std::path::{Path, PathBuf};

use anyhow::Context;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::error::{AppError, AppResult};
use crate::models;

/// 构造存储路径：`{base_dir}/{project_id}/{release}/{filename}`
pub fn storage_path(base_dir: &str, project_id: i32, release: &str, filename: &str) -> PathBuf {
    Path::new(base_dir)
        .join(project_id.to_string())
        .join(release)
        .join(filename)
}

/// 将文件内容写入磁盘，自动创建父目录。
pub async fn write_file(path: &Path, data: &[u8]) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .context("create dir")
            .map_err(|e| AppError::Internal(e.to_string()))?;
    }
    tokio::fs::write(path, data)
        .await
        .context("write sourcemap file")
        .map_err(|e| AppError::Internal(e.to_string()))
}

/// 删除磁盘上的 Source Map 文件。
pub async fn delete_file(path: &Path) -> AppResult<()> {
    if path.exists() {
        tokio::fs::remove_file(path)
            .await
            .context("delete sourcemap file")
            .map_err(|e| AppError::Internal(e.to_string()))?;
    }
    Ok(())
}

/// 计算数据的 MD5 hex 字符串（用于去重）。
pub fn md5_hex(data: &[u8]) -> String {
    let digest = md5_simple(data);
    digest
}

fn md5_simple(data: &[u8]) -> String {
    // 使用标准库无法直接计算 MD5，这里用简单的 hash 替代（实际项目可依赖 md5 crate）
    // 为保持零额外依赖，用 fnv 风格 hash 作为 content key
    let mut h: u64 = 14695981039346656037;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    format!("{:016x}", h)
}

/// 使用 `sourcemap` crate 解析堆栈（每行尝试映射到原始位置）。
pub fn map_stacktrace(stack: &str, source_map_data: &[u8]) -> Option<String> {
    let sm = sourcemap::SourceMap::from_reader(source_map_data).ok()?;
    let mut lines = Vec::new();

    for line in stack.lines() {
        // 尝试匹配形如 "at ... (file.js:123:45)" 或 "file.js:123:45"
        if let Some((file_hint, row, col)) = parse_stack_line(line) {
            if let Some(token) = sm.lookup_token(row.saturating_sub(1), col.saturating_sub(1)) {
                let src = token.get_source().unwrap_or(&file_hint);
                let src_line = token.get_src_line() + 1;
                let src_col = token.get_src_col() + 1;
                let name = token.get_name().unwrap_or("");
                let mapped = if name.is_empty() {
                    format!(
                        "    at {} ({}:{}:{}) [原始: {}:{}:{}]",
                        name, src, src_line, src_col, file_hint, row, col
                    )
                } else {
                    format!("    at {} ({}:{}:{})", name, src, src_line, src_col)
                };
                lines.push(mapped);
                continue;
            }
        }
        lines.push(line.to_string());
    }
    Some(lines.join("\n"))
}

/// 从 DB 查找项目 + release 对应的 Source Map 文件内容。
pub async fn load_source_map_for_error(
    db: &DatabaseConnection,
    project_id: i32,
    release: &str,
    source_file: &str,
) -> AppResult<Option<Vec<u8>>> {
    let maps = models::SourceMap::find()
        .filter(models::source_map::Column::ProjectId.eq(project_id))
        .filter(models::source_map::Column::Release.eq(release))
        .all(db)
        .await?;

    // 找最匹配的文件名。上报栈里通常是 `app.js`，上传文件常是 `app.js.map`。
    let matched = maps
        .iter()
        .find(|m| filename_matches_source(&m.filename, source_file))
        .or_else(|| maps.first());

    if let Some(sm) = matched {
        let data = tokio::fs::read(&sm.storage_path).await.ok();
        return Ok(data);
    }
    Ok(None)
}

fn filename_matches_source(map_filename: &str, source_file: &str) -> bool {
    if source_file.is_empty() {
        return false;
    }

    let source_name = normalize_asset_name(source_file);
    let map_name = normalize_asset_name(map_filename)
        .trim_end_matches(".map")
        .to_string();

    source_file.contains(map_filename)
        || map_filename.contains(source_file)
        || source_name == map_name
        || source_name.ends_with(&map_name)
        || map_name.ends_with(&source_name)
}

fn normalize_asset_name(value: &str) -> String {
    value
        .split(['?', '#'])
        .next()
        .unwrap_or(value)
        .rsplit('/')
        .next()
        .unwrap_or(value)
        .to_string()
}

/// 解析堆栈行，返回 (文件名, 行号, 列号)。
fn parse_stack_line(line: &str) -> Option<(String, u32, u32)> {
    // 匹配 "...(filename.js:LINE:COL)" 或 "filename.js:LINE:COL"
    let re_paren = extract_location_in_parens(line);
    if let Some(loc) = re_paren {
        return parse_location(&loc);
    }
    // 尝试直接解析整行
    parse_location(line.trim())
}

fn extract_location_in_parens(line: &str) -> Option<String> {
    let start = line.rfind('(')?;
    let end = line.rfind(')')?;
    if end > start {
        Some(line[start + 1..end].to_string())
    } else {
        None
    }
}

fn parse_location(loc: &str) -> Option<(String, u32, u32)> {
    // 格式: file.js:123:45
    let parts: Vec<&str> = loc.rsplitn(3, ':').collect();
    if parts.len() == 3 {
        let col: u32 = parts[0].trim().parse().ok()?;
        let row: u32 = parts[1].trim().parse().ok()?;
        let file = parts[2].to_string();
        Some((file, row, col))
    } else {
        None
    }
}
