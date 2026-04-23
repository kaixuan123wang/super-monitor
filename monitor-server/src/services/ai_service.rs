//! AI 错误分析服务：调用 LLM API，写入 ai_analyses 表。

use chrono::{FixedOffset, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde_json::{json, Value};
use std::time::Instant;

use crate::config::Config;
use crate::error::AppResult;
use crate::models;
use crate::services::sourcemap_service;

fn now_fixed() -> chrono::DateTime<FixedOffset> {
    Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap())
}

/// 触发对单条错误的 AI 分析（异步，调用方用 tokio::spawn）。
pub async fn analyze_error(
    db: &DatabaseConnection,
    cfg: &Config,
    error: &models::js_error::Model,
) -> AppResult<models::ai_analysis::Model> {
    // 1. 检查是否已有缓存结果（同 fingerprint，7 天内）
    if let Some(fp) = &error.fingerprint {
        let cache_key = format!("{}:{}", error.project_id, fp);
        let cutoff = (Utc::now() - chrono::Duration::days(7))
            .with_timezone(&FixedOffset::east_opt(0).unwrap());
        let cached = models::AiAnalysis::find()
            .filter(models::ai_analysis::Column::CacheKey.eq(&cache_key))
            .filter(models::ai_analysis::Column::Status.eq("success"))
            .filter(models::ai_analysis::Column::CreatedAt.gte(cutoff))
            .one(db)
            .await?;
        if let Some(row) = cached {
            // 为当前 error_id 创建一条 is_cached=true 的记录
            let am = models::ai_analysis::ActiveModel {
                id: sea_orm::NotSet,
                error_id: Set(error.id),
                fingerprint: Set(error.fingerprint.clone()),
                project_id: Set(error.project_id),
                model_used: Set(row.model_used.clone()),
                prompt_tokens: Set(None),
                completion_tokens: Set(None),
                cost_ms: Set(None),
                status: Set("success".into()),
                ai_suggestion: Set(row.ai_suggestion.clone()),
                severity_score: Set(row.severity_score),
                confidence: Set(row.confidence),
                probable_file: Set(row.probable_file.clone()),
                probable_line: Set(row.probable_line),
                tags: Set(row.tags.clone()),
                analyzed_stack: Set(row.analyzed_stack.clone()),
                is_cached: Set(true),
                cache_key: Set(Some(cache_key)),
                created_at: Set(now_fixed()),
                updated_at: Set(now_fixed()),
            };
            let saved = am.insert(db).await?;
            return Ok(saved);
        }
    }

    // 2. 创建 pending 记录
    let fp = error.fingerprint.clone();
    let cache_key = fp.as_ref().map(|f| format!("{}:{}", error.project_id, f));
    let pending = models::ai_analysis::ActiveModel {
        id: sea_orm::NotSet,
        error_id: Set(error.id),
        fingerprint: Set(fp.clone()),
        project_id: Set(error.project_id),
        model_used: Set(Some(cfg.ai_model.clone())),
        prompt_tokens: Set(None),
        completion_tokens: Set(None),
        cost_ms: Set(None),
        status: Set("pending".into()),
        ai_suggestion: Set(None),
        severity_score: Set(None),
        confidence: Set(None),
        probable_file: Set(None),
        probable_line: Set(None),
        tags: Set(None),
        analyzed_stack: Set(None),
        is_cached: Set(false),
        cache_key: Set(cache_key.clone()),
        created_at: Set(now_fixed()),
        updated_at: Set(now_fixed()),
    };
    let mut record = pending.insert(db).await?;

    // 3. 尝试解析 Source Map 还原堆栈
    let analyzed_stack = if let (Some(stack), Some(release)) = (&error.stack, &error.release) {
        let source_file = error.source_url.as_deref().unwrap_or("");
        match sourcemap_service::load_source_map_for_error(
            db,
            error.project_id,
            release,
            source_file,
        )
        .await
        {
            Ok(Some(sm_data)) => sourcemap_service::map_stacktrace(stack, &sm_data),
            _ => None,
        }
    } else {
        None
    };

    // 4. 构建 prompt
    let prompt = build_prompt(error, analyzed_stack.as_deref());

    // 5. 调用 LLM（如未配置 API Key，跳过）
    if cfg.ai_api_key.is_empty() || !cfg.ai_enabled {
        let mut am: models::ai_analysis::ActiveModel = record.clone().into();
        am.status = Set("failed".into());
        am.ai_suggestion = Set(Some("AI 分析未启用（未配置 AI_API_KEY）".into()));
        am.analyzed_stack = Set(analyzed_stack);
        am.updated_at = Set(now_fixed());
        record = am.update(db).await?;
        return Ok(record);
    }

    let start = Instant::now();
    let llm_result = call_llm(&cfg.ai_api_base, &cfg.ai_api_key, &cfg.ai_model, &prompt).await;
    let cost_ms = start.elapsed().as_millis() as i32;

    // 6. 解析并写库
    let mut am: models::ai_analysis::ActiveModel = record.clone().into();
    am.cost_ms = Set(Some(cost_ms));
    am.analyzed_stack = Set(analyzed_stack);
    am.updated_at = Set(now_fixed());

    match llm_result {
        Ok((content, prompt_tokens, completion_tokens)) => {
            am.prompt_tokens = Set(prompt_tokens);
            am.completion_tokens = Set(completion_tokens);
            match parse_ai_response(&content) {
                Ok(parsed) => {
                    am.status = Set("success".into());
                    am.ai_suggestion = Set(Some(build_suggestion_text(&parsed)));
                    am.severity_score = Set(parsed
                        .get("severity")
                        .and_then(|v| v.as_i64())
                        .map(|v| v as i16));
                    am.confidence = Set(parsed
                        .get("confidence")
                        .and_then(|v| v.as_f64())
                        .map(|v| v as f32));
                    am.probable_file = Set(parsed
                        .get("probable_file")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()));
                    am.probable_line = Set(parsed
                        .get("probable_line")
                        .and_then(|v| v.as_i64())
                        .map(|v| v as i32));
                    am.tags = Set(parsed.get("tags").cloned());
                }
                Err(_) => {
                    // JSON 解析失败，保存原始文本
                    am.status = Set("success".into());
                    am.ai_suggestion = Set(Some(content));
                }
            }
        }
        Err(e) => {
            am.status = Set("failed".into());
            am.ai_suggestion = Set(Some(format!("AI 分析失败: {}", e)));
        }
    }

    record = am.update(db).await?;
    Ok(record)
}

fn build_prompt(error: &models::js_error::Model, mapped_stack: Option<&str>) -> String {
    format!(
        r#"你是一位资深前端错误分析专家。请分析以下报错信息并给出专业诊断：

【错误信息】
{}

【错误类型】
{}

【错误堆栈】
{}

【Source Map 解析后的堆栈】
{}

【页面信息】
URL: {}
浏览器: {} {}
操作系统: {} {}

【上下文】
SDK 版本: {}
代码版本: {}
环境: {}

请按以下 JSON 格式输出分析结果（只输出 JSON，不要其他文字）：
{{
  "root_cause": "错误原因分析（2-3句话，技术性强）",
  "fix_suggestion": "修复方案（含具体代码示例）",
  "severity": 1,
  "probable_file": "可能出错的文件路径",
  "probable_line": 0,
  "confidence": 0.8,
  "tags": ["标签1", "标签2"]
}}"#,
        error.message,
        error.error_type,
        error.stack.as_deref().unwrap_or("(无堆栈)"),
        mapped_stack.unwrap_or("(无 Source Map)"),
        error.url.as_deref().unwrap_or(""),
        error.browser.as_deref().unwrap_or(""),
        error.browser_version.as_deref().unwrap_or(""),
        error.os.as_deref().unwrap_or(""),
        error.os_version.as_deref().unwrap_or(""),
        error.sdk_version.as_deref().unwrap_or(""),
        error.release.as_deref().unwrap_or(""),
        error.environment.as_deref().unwrap_or(""),
    )
}

fn build_suggestion_text(parsed: &Value) -> String {
    let root_cause = parsed
        .get("root_cause")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let fix = parsed
        .get("fix_suggestion")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    format!("【根因分析】\n{}\n\n【修复建议】\n{}", root_cause, fix)
}

fn parse_ai_response(content: &str) -> Result<Value, serde_json::Error> {
    // 尝试提取 JSON block
    let json_str = if let (Some(start), Some(end)) = (content.find('{'), content.rfind('}')) {
        &content[start..=end]
    } else {
        content
    };
    serde_json::from_str(json_str)
}

/// 调用 OpenAI 兼容接口，返回 (content, prompt_tokens, completion_tokens)。
async fn call_llm(
    api_base: &str,
    api_key: &str,
    model: &str,
    prompt: &str,
) -> Result<(String, Option<i32>, Option<i32>), String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;

    let body = json!({
        "model": model,
        "messages": [
            {"role": "user", "content": prompt}
        ],
        "max_tokens": 2048,
        "temperature": 0.3
    });

    let url = format!("{}/chat/completions", api_base.trim_end_matches('/'));
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("LLM API error {}: {}", status, text));
    }

    let data: Value = resp.json().await.map_err(|e| e.to_string())?;
    let content = data["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let prompt_tokens = data["usage"]["prompt_tokens"].as_i64().map(|v| v as i32);
    let completion_tokens = data["usage"]["completion_tokens"]
        .as_i64()
        .map(|v| v as i32);

    Ok((content, prompt_tokens, completion_tokens))
}
