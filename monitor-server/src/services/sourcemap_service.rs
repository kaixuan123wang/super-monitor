//! Source Map 文件存储与堆栈解析服务。

use std::path::{Path, PathBuf};

use anyhow::Context;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::error::{AppError, AppResult};
use crate::models;

/// 构造存储路径：`{base_dir}/{project_id}/{release}/{filename}`
/// 对 filename 做路径遍历防护：仅保留文件名部分，拒绝包含 `..` 的路径。
pub fn storage_path(base_dir: &str, project_id: i32, release: &str, filename: &str) -> PathBuf {
    let base = match Path::new(base_dir).canonicalize() {
        Ok(p) => p,
        Err(_) => {
            return Path::new(base_dir)
                .join(project_id.to_string())
                .join("_invalid")
                .join("rejected");
        }
    };

    // 取最后一段文件名，防止 ../../../etc/passwd 之类的路径遍历
    let safe_filename = Path::new(filename)
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".into());

    // 额外检查：拒绝包含 .. 的组件
    if safe_filename.contains("..")
        || safe_filename.is_empty()
        || release.contains("..")
        || release.contains('/')
        || release.contains('\\')
        || release.is_empty()
    {
        return base
            .join(project_id.to_string())
            .join("_invalid")
            .join("rejected");
    }

    let path = base
        .join(project_id.to_string())
        .join(release)
        .join(&safe_filename);

    // 最终校验：确保构造出的路径在 base_dir 下。
    if !path.starts_with(&base) {
        return base
            .join(project_id.to_string())
            .join("_invalid")
            .join("rejected");
    }

    path
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
/// 会校验 `path` 必须在 `base_dir` 之下，防止路径遍历删除任意文件。
pub async fn delete_file(base_dir: &str, path: &Path) -> AppResult<()> {
    if !path.exists() {
        return Ok(());
    }
    let base = Path::new(base_dir)
        .canonicalize()
        .map_err(|_| AppError::Internal("invalid sourcemap base dir".into()))?;
    let resolved = path
        .canonicalize()
        .map_err(|_| AppError::BadRequest("invalid storage path".into()))?;
    if !resolved.starts_with(&base) {
        return Err(AppError::Forbidden);
    }
    tokio::fs::remove_file(path)
        .await
        .context("delete sourcemap file")
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(())
}

/// 计算数据的 content hash（FNV-1a 64bit，用于去重）。
pub fn content_hash(data: &[u8]) -> String {
    let mut h: u64 = 14695981039346656037;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    format!("{:016x}", h)
}

/// 计算数据的 MD5 hex 字符串（用于去重）。
/// 注意：当前使用 FNV-1a 哈希作为内容指纹，碰撞率高于真正的 MD5。
pub fn md5_hex(data: &[u8]) -> String {
    content_hash(data)
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
                        "    at {}:{}:{} [原始: {}:{}:{}]",
                        src, src_line, src_col, file_hint, row, col
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
    let map_name = normalize_asset_name(map_filename);
    let map_base = map_name
        .strip_suffix(".map")
        .unwrap_or(&map_name)
        .to_string();

    source_file.contains(map_filename)
        || map_filename.contains(source_file)
        || source_name == map_base
        || source_name.ends_with(&map_base)
        || map_base.ends_with(&source_name)
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
    // 格式: file.js:123:45 或 https://example.com/file.js:123:45
    // 从末尾解析 col:row:file，col 和 row 必须是纯数字
    let last_colon = loc.rfind(':')?;
    let col_str = loc[last_colon + 1..].trim();
    let col: u32 = col_str.parse().ok()?;
    let rest = &loc[..last_colon];

    let second_colon = rest.rfind(':')?;
    let row_str = rest[second_colon + 1..].trim();
    let row: u32 = row_str.parse().ok()?;
    let file = rest[..second_colon].to_string();

    if file.is_empty() {
        return None;
    }
    Some((file, row, col))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_path_normal() {
        let base = std::env::current_dir().unwrap();
        let base_str = base.to_string_lossy();
        let path = storage_path(&base_str, 1, "v1.0", "app.js.map");
        assert!(path.to_string_lossy().contains("1"));
        assert!(path.to_string_lossy().contains("v1.0"));
        assert!(path.to_string_lossy().contains("app.js.map"));
    }

    #[test]
    fn test_storage_path_traversal_attack() {
        let path = storage_path("/data/maps", 1, "v1.0", "../../../etc/passwd");
        // Should reject path traversal
        assert!(
            path.to_string_lossy().contains("_invalid") || !path.to_string_lossy().contains("etc")
        );
    }

    #[test]
    fn test_storage_path_dotdot_in_release() {
        let path = storage_path("/data/maps", 1, "../etc", "app.js.map");
        assert!(path.to_string_lossy().contains("_invalid"));
    }

    #[test]
    fn test_content_hash_deterministic() {
        let data = b"hello world";
        let h1 = content_hash(data);
        let h2 = content_hash(data);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_content_hash_different_data() {
        let h1 = content_hash(b"hello");
        let h2 = content_hash(b"world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_content_hash_format() {
        let h = content_hash(b"test");
        assert_eq!(h.len(), 16);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_md5_hex_delegates_to_content_hash() {
        let data = b"test data";
        assert_eq!(md5_hex(data), content_hash(data));
    }

    #[test]
    fn test_filename_matches_source_exact() {
        assert!(filename_matches_source("app.js.map", "app.js"));
    }

    #[test]
    fn test_filename_matches_source_with_path() {
        assert!(filename_matches_source("app.js.map", "https://cdn.example.com/static/app.js"));
    }

    #[test]
    fn test_filename_matches_source_no_match() {
        assert!(!filename_matches_source("vendor.js.map", "app.js"));
    }

    #[test]
    fn test_filename_matches_source_empty() {
        assert!(!filename_matches_source("app.js.map", ""));
    }

    #[test]
    fn test_normalize_asset_name_strips_query() {
        assert_eq!(normalize_asset_name("app.js?v=123"), "app.js");
    }

    #[test]
    fn test_normalize_asset_name_strips_hash() {
        assert_eq!(normalize_asset_name("app.js#section"), "app.js");
    }

    #[test]
    fn test_normalize_asset_name_strips_path() {
        assert_eq!(normalize_asset_name("/static/js/app.js"), "app.js");
    }

    #[test]
    fn test_parse_stack_line_paren_format() {
        let result = parse_stack_line("    at Object.onClick (https://cdn.com/app.js:123:45)");
        assert!(result.is_some());
        let (file, row, col) = result.unwrap();
        assert_eq!(file, "https://cdn.com/app.js");
        assert_eq!(row, 123);
        assert_eq!(col, 45);
    }

    #[test]
    fn test_parse_stack_line_direct_format() {
        let result = parse_stack_line("app.js:10:20");
        assert!(result.is_some());
        let (file, row, col) = result.unwrap();
        assert_eq!(file, "app.js");
        assert_eq!(row, 10);
        assert_eq!(col, 20);
    }

    #[test]
    fn test_parse_stack_line_no_colon() {
        assert!(parse_stack_line("no location here").is_none());
    }

    #[test]
    fn test_parse_stack_line_non_numeric() {
        assert!(parse_stack_line("app.js:abc:def").is_none());
    }

    #[test]
    fn test_extract_location_in_parens_found() {
        let loc = extract_location_in_parens("at func (file.js:1:2)");
        assert_eq!(loc, Some("file.js:1:2".into()));
    }

    #[test]
    fn test_extract_location_in_parens_not_found() {
        assert!(extract_location_in_parens("no parens here").is_none());
    }

    #[test]
    fn test_parse_location_standard() {
        let result = parse_location("app.js:100:50");
        assert_eq!(result, Some(("app.js".into(), 100, 50)));
    }

    #[test]
    fn test_parse_location_url() {
        let result = parse_location("https://cdn.com/bundle.js:10:20");
        assert_eq!(result, Some(("https://cdn.com/bundle.js".into(), 10, 20)));
    }

    #[test]
    fn test_parse_location_empty_file() {
        assert!(parse_location(":10:20").is_none());
    }

    #[test]
    fn test_delete_file_nonexistent() {
        // Should not error for non-existent file
        let rt = tokio::runtime::Runtime::new().unwrap();
        let base = std::env::current_dir().unwrap();
        let result = rt.block_on(delete_file(
            base.to_str().unwrap(),
            std::path::Path::new("/nonexistent/file.map"),
        ));
        assert!(result.is_ok());
    }

    #[test]
    fn test_map_stacktrace_invalid_sourcemap() {
        let result = map_stacktrace("at func (app.js:1:2)", b"not a sourcemap");
        assert!(result.is_none());
    }
}
