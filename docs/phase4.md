# Phase 4：AI 与告警

## 目标

1. 告警规则配置
2. Webhook/邮件通知
3. AI 分析集成（调用 LLM API）
4. Source Map 上传与解析
5. **埋点分析**：漏斗分析（定义漏斗 + 可视化漏斗图）
6. **埋点分析**：留存分析（cohort 热力矩阵 + 曲线图）
7. **埋点调试**：实时事件流 Debug 页面（SSE 推送）

---

## 4.1 AI 分析模块

### 4.1.1 触发时机

| 触发方式 | 条件 | 优先级 |
|---------|------|--------|
| 手动触发 | 用户点击「AI 分析」按钮 | 高（立即执行） |
| 自动触发（新错误） | 新 fingerprint 首次出现 | 中（队列异步处理） |
| 自动触发（累积阈值） | 相同错误 1 小时内出现 > 50 次 | 中（队列异步处理） |
| 批量触发 | 用户选择多个错误批量分析 | 低（队列异步处理） |

### 4.1.2 AI Prompt 设计

```
你是一位资深前端错误分析专家。请分析以下报错信息并给出专业诊断：

【错误信息】
{error_message}

【错误类型】
{error_type}

【错误堆栈】
{stack_trace}

【Source Map 解析后的堆栈】（如有）
{mapped_stack}

【相关代码片段】（如有）
{code_snippet}

【页面信息】
URL: {url}
浏览器: {browser} {browser_version}
操作系统: {os} {os_version}
设备: {device}

【上下文】
SDK 版本: {sdk_version}
代码版本: {release}
环境: {environment}

请按以下 JSON 格式输出分析结果：
{
  "root_cause": "错误原因分析（2-3句话，技术性强）",
  "fix_suggestion": "修复方案（含具体代码示例）",
  "severity": 1-5,
  "probable_file": "可能出错的文件路径",
  "probable_line": 行号,
  "confidence": 0.0-1.0,
  "tags": ["标签1", "标签2"]
}
```

### 4.1.3 AI 服务实现

```rust
pub async fn analyze_error(error: &JsError, source_map: Option<&SourceMap>) -> Result<AiAnalysis> {
    // 1. 检查缓存
    let cache_key = generate_cache_key(error);
    if let Some(cached) = get_cached_analysis(&cache_key).await? {
        return Ok(cached);
    }

    // 2. 解析 Source Map
    let mapped_stack = if let Some(sm) = source_map {
        sourcemap_service::map_stacktrace(&error.stack, sm).await?
    } else {
        None
    };

    // 3. 构建 Prompt
    let prompt = build_prompt(error, mapped_stack.as_ref());

    // 4. 调用 LLM API
    let start = Instant::now();
    let response = call_llm_api(&prompt).await?;
    let cost_ms = start.elapsed().as_millis() as i32;

    // 5. 解析响应
    let analysis = parse_ai_response(&response)?;

    // 6. 保存结果（含缓存标记）
    save_analysis(&analysis, &cache_key, cost_ms).await?;

    Ok(analysis)
}
```

### 4.1.4 AI 调用配置

```toml
[ai]
provider = "deepseek"
api_key = "${AI_API_KEY}"
api_base = "https://api.deepseek.com/v1"
model = "deepseek-chat"
max_tokens = 2048
temperature = 0.3
request_timeout = 60

# 限流配置
rate_limit_per_minute = 60
rate_limit_per_project = 20

# 缓存配置
cache_ttl_days = 7
```

### 4.1.5 降级策略

| 场景 | 处理方式 |
|------|---------|
| AI 服务超时（>60s） | 返回「分析超时，请稍后重试」，任务入队列重试 |
| AI 服务返回格式错误 | 记录日志，返回「AI 分析失败，请手动分析」 |
| AI API 限流（429） | 指数退避重试，最大重试 3 次 |
| AI 服务完全不可用 | 关闭自动触发，仅保留手动触发入口 |

### 4.1.6 AI 分析 API

```
POST /api/ai/analyze/:error_id    # 触发 AI 分析（异步）
  Response: { task_id, status: 'queued' }

GET  /api/ai/analysis/:error_id   # 获取 AI 分析结果
  Response: { id, error_id, ai_suggestion, ai_confidence, severity_score, ... }
  或 { status: 'pending' } 如果分析中

GET  /api/ai/analyses             # AI 分析历史列表
  Query: project_id, page, model_used, has_suggestion

POST /api/ai/analyze-batch        # 批量触发 AI 分析
  Body: { fingerprint, project_id }
```

**AI 限流策略**：
- 单个项目每分钟最多 20 次 AI 分析请求
- 相同 fingerprint 的分析结果缓存 7 天
- 超出限流返回 429，前端提示「分析队列已满，请稍后重试」

---

## 4.2 Source Map 支持

### 4.2.1 上传流程

1. 用户 CI/CD 构建后，调用 API 上传 `.map` 文件
2. 后端校验文件格式（必须是有效的 Source Map）
3. 文件存储至本地磁盘或对象存储（S3/MinIO）
4. 数据库记录文件元信息

### 4.2.2 上传 API

```
POST /api/sourcemaps              # 上传 Source Map（multipart/form-data）
  Headers: Authorization
  Body: { project_id, version, file: .map文件 }
  Response: { id, filename, uploaded_at }

GET  /api/sourcemaps              # Source Map 列表
  Query: project_id, version, page
GET  /api/sourcemaps/:id          # Source Map 详情
DELETE /api/sourcemaps/:id        # 删除 Source Map
```

### 4.2.3 解析流程

```rust
pub async fn resolve_stacktrace(
    project_id: i32,
    release: &str,
    stacktrace: &str,
) -> Result<Option<String>> {
    // 1. 查找匹配的 Source Map
    let source_maps = find_source_maps(project_id, release).await?;

    // 2. 解析每一行堆栈
    let mut resolved = Vec::new();
    for line in stacktrace.lines() {
        if let Some((file, line_no, col)) = parse_stack_line(line) {
            if let Some(sm) = source_maps.iter().find(|s| s.filename.contains(&file)) {
                let mapping = sourcemap_service::map_position(sm, line_no, col).await?;
                resolved.push(format!(
                    "    at {} ({}:{}:{}) [原始: {}:{}:{}]",
                    mapping.name.unwrap_or_default(),
                    mapping.source,
                    mapping.line,
                    mapping.column,
                    file, line_no, col
                ));
            }
        }
    }

    Ok(Some(resolved.join("\n")))
}
```

### 4.2.4 自动关联

- SDK 上报时携带 `release` 字段（如 git commit hash 或版本号）
- 后端收到错误后，自动查找 `project_id + release` 匹配的 Source Map
- 解析后的原始堆栈存入 `ai_analyses.analyzed_stack`

---

## 4.3 实时告警设计

### 4.3.1 告警规则类型

| 规则类型 | 触发条件 | 参数 |
|---------|---------|------|
| error_spike | 1 分钟内错误数 > N | threshold: 错误数 |
| failure_rate | 接口失败率 > X% | threshold: 百分比 (1-100) |
| new_error | 出现新的 fingerprint | 无阈值 |
| p0_error | 出现 P0 级错误 | 无阈值 |
| error_trend | 错误数较上小时增长 X% | threshold: 百分比 |

### 4.3.2 告警处理流程

```
SDK 上报错误 → 后端接收 → Redis Stream 入队
                                    ↓
告警处理器消费 → 匹配项目告警规则 → 判断是否触发
                                    ↓
                    是 → 检查去重（10分钟窗口）
                                    ↓
                    未告警过 → 生成告警内容 → SSE 推送
                                    ↓
                    有 Webhook → 调用 Webhook
                    有邮箱 → 发送邮件
```

### 4.3.3 告警去重与升级

```rust
// 去重逻辑
const DEDUP_WINDOW_MINUTES: i64 = 10;

async fn should_alert(rule_id: i32, fingerprint: &str) -> bool {
    let key = format!("alert:{}:{}", rule_id, fingerprint);
    let is_new: bool = redis::cmd("SET")
        .arg(&key)
        .arg("1")
        .arg("NX")
        .arg("EX")
        .arg(DEDUP_WINDOW_MINUTES * 60)
        .query_async(&mut conn)
        .await?;
    Ok(is_new)
}

// 告警升级
async fn check_alert_escalation(rule_id: i32, project_id: i32) {
    let recent_alerts = count_alerts_in_window(rule_id, 30).await?;
    if recent_alerts >= 3 {
        send_escalated_alert(rule_id, project_id).await?;
    }
}
```

### 4.3.4 通知渠道

| 渠道 | Phase 1 | Phase 2 |
|------|---------|---------|
| SSE 实时推送 | ✅ 实现 | ✅ |
| Webhook（飞书/钉钉/企微/Slack） | ⬜ 预留接口 | ✅ 实现 |
| 邮件通知 | ⬜ 预留接口 | ✅ 实现 |
| 短信通知 | ⬜ 不实现 | ⬜ 不实现 |

**SSE 推送范围**：用户登录后，推送其所有有权限项目的告警。

### 4.3.5 告警消息格式

```json
{
  "type": "alert",
  "data": {
    "rule_id": 1,
    "rule_name": "生产环境错误激增",
    "project_id": 1,
    "project_name": "官网",
    "alert_type": "error_spike",
    "severity": "warning",
    "content": "1分钟内检测到 25 个错误，超过阈值 10",
    "error_count": 25,
    "sample_errors": [
      { "id": 123, "message": "TypeError: Cannot read...", "url": "..." }
    ],
    "timestamp": "2024-01-15T10:30:00Z"
  }
}
```

### 4.3.6 告警管理 API

```
GET  /api/alerts/rules            # 告警规则列表
  Query: project_id
POST /api/alerts/rules            # 创建告警规则
  Body: { project_id, name, rule_type, threshold, interval_minutes, webhook_url, email }
PUT  /api/alerts/rules/:id        # 更新规则
DELETE /api/alerts/rules/:id      # 删除规则

GET  /api/alerts/logs             # 告警历史
  Query: project_id, page, status, start_time, end_time
GET  /api/alerts/logs/:id         # 告警详情
```

---

## 4.4 埋点漏斗分析

### 4.4.1 漏斗定义

```sql
CREATE TABLE track_funnels (
    id              SERIAL PRIMARY KEY,
    project_id      INTEGER NOT NULL REFERENCES projects(id),
    name            VARCHAR(200) NOT NULL,
    description     TEXT,
    steps           JSONB NOT NULL,
    window_minutes  INTEGER DEFAULT 1440,
    created_by      INTEGER REFERENCES users(id),
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    updated_at      TIMESTAMPTZ DEFAULT NOW()
);
```

steps 格式：`[{"event": "view_product", "filters": {...}}, ...]`

### 4.4.2 漏斗分析 API

```
GET    /api/tracking/funnels                 # 漏斗列表
POST   /api/tracking/funnels                 # 创建漏斗定义
GET    /api/tracking/funnels/:id             # 漏斗详情
PUT    /api/tracking/funnels/:id             # 更新漏斗
DELETE /api/tracking/funnels/:id             # 删除漏斗
POST   /api/tracking/funnels/:id/analyze     # 执行漏斗分析
  Body: { time_range: { start, end }, group_by }
  Response: {
    steps: [
      { event, display_name, user_count, conversion_rate, avg_time_to_next },
      ...
    ],
    overall_conversion: 0.23,
    breakdown: { }
  }
```

### 4.4.3 前端漏斗分析页

交互设计（参考神策漏斗）：
- 拖拽排序的步骤配置区
- 可视化漏斗图（各步骤转化率 + 流失率）
- 下方明细：各步骤转化人数、平均转化时长
- 支持分组对比（如 A/B 实验对比不同渠道的转化）

---

## 4.5 埋点留存分析

### 4.5.1 留存配置

```sql
CREATE TABLE track_retention_configs (
    id              SERIAL PRIMARY KEY,
    project_id      INTEGER NOT NULL REFERENCES projects(id),
    name            VARCHAR(200) NOT NULL,
    initial_event   VARCHAR(128) NOT NULL,
    return_event    VARCHAR(128) NOT NULL,
    initial_filters JSONB,
    return_filters  JSONB,
    retention_days  INTEGER DEFAULT 7,
    created_by      INTEGER REFERENCES users(id),
    created_at      TIMESTAMPTZ DEFAULT NOW()
);
```

### 4.5.2 留存分析 API

```
GET    /api/tracking/retentions              # 留存配置列表
POST   /api/tracking/retentions              # 创建留存配置
POST   /api/tracking/retentions/:id/analyze  # 执行留存分析
  Body: { time_range, retention_type: 'day' | 'week' }
  Response: {
    retention_table: [
      { cohort_date, cohort_size, day_1: 0.45, day_2: 0.32, ..., day_7: 0.18 },
      ...
    ],
    avg_retention: [0.45, 0.32, 0.28, 0.22, 0.20, 0.19, 0.18]
  }
```

### 4.5.3 前端留存分析页

交互设计（参考 Amplitude Retention）：
- 上方：初始事件 + 回访事件 + 时间范围配置
- 中间：热力矩阵（cohort 表格，颜色深浅表示留存率高低）
- 下方：平均留存曲线图

---

## 4.6 埋点实时事件流 Debug

### 4.6.1 SSE 实时事件流

```
GET    /api/tracking/live-events             # SSE 实时事件流
  Query: project_id, distinct_id（可选，过滤特定用户）
  Events:
    - track: { event, distinct_id, properties, created_at }
    - profile: { distinct_id, properties }
```

### 4.6.2 前端 Debug 页面

功能：
- 实时显示 incoming 的埋点事件
- 支持按事件名、用户 ID 过滤
- 事件 JSON 格式化展示
- 暂停/继续接收

---

## 4.7 本阶段验收标准

- [ ] AI 分析能正确调用 LLM API 并返回分析结果
- [ ] Source Map 能上传、解析并正确还原堆栈
- [ ] 告警规则能正确触发并推送 SSE 通知
- [ ] 告警去重和升级机制正常工作
- [ ] 漏斗分析能计算各步骤转化率
- [ ] 留存分析能生成 cohort 热力矩阵
- [ ] 实时事件流 Debug 页面能正常显示事件
