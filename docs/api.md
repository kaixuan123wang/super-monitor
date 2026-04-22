# 附录：API 设计

## SDK 公开接口

```
POST /api/v1/collect              # SDK 数据上报（按 app_id + app_key 校验）
  Headers: X-App-Id, X-App-Key
  Body: { type: 'error'|'network'|'performance'|'breadcrumb'|'track'|'track_batch'|'profile'|'track_signup', data: {...} }
  Response: { code: 0, message: 'ok' }

GET  /api/v1/collect/health       # SDK 健康检查
  Response: { status: 'ok', version: '1.0.0' }
```

---

## 认证接口

```
POST /api/auth/login              # 登录
  Body: { email, password }
  Response: { access_token, refresh_token, expires_in, user }

POST /api/auth/register           # 注册（需分组邀请码或创建新分组）
  Body: { username, email, password, group_invite_code? }
  Response: { access_token, refresh_token, user }

POST /api/auth/refresh            # 刷新 Token
  Headers: Authorization: Bearer {refresh_token}
  Response: { access_token, expires_in }

GET  /api/auth/me                 # 当前用户信息
  Response: { id, username, email, role, group_id, permissions }

POST /api/auth/logout             # 登出（使 refresh_token 失效）
```

---

## 项目管理

```
GET    /api/projects              # 项目列表（当前用户有权限的）
  Query: page, page_size, group_id, keyword
  Response: { total, list: [{ id, name, app_id, group_id, owner_id, ... }] }

POST   /api/projects              # 创建项目（需 admin/owner 角色）
  Body: { name, group_id, description, alert_threshold, data_retention_days }
  Response: { id, name, app_id, app_key, ... }

GET    /api/projects/:id          # 项目详情
PUT    /api/projects/:id          # 更新项目（owner 或 admin 可修改）
DELETE /api/projects/:id          # 删除项目（owner 或 admin）

GET    /api/projects/:id/members  # 项目成员列表
POST   /api/projects/:id/members  # 添加成员（owner/admin）
DELETE /api/projects/:id/members/:user_id  # 移除成员
PUT    /api/projects/:id/members/:user_id  # 修改成员角色
```

---

## 错误管理

```
GET /api/errors                   # 错误列表
  Query:
    - project_id (required)
    - page, page_size
    - start_time, end_time       # ISO 8601 格式
    - error_type                 # 错误类型筛选
    - browser, os, device        # 设备信息筛选
    - source_url                 # 来源 URL 筛选
    - release                    # 版本筛选
    - environment                # 环境筛选
    - fingerprint                # 相同错误聚合
    - has_ai_analysis            # 是否已 AI 分析
    - sort_by: 'time'|'count'|'latest'
  Response: { total, list: [...], aggregations: { error_types, browsers, ... } }

GET /api/errors/:id               # 错误详情（含 AI 分析结果）
GET /api/errors/stats             # 错误统计（按时间聚合）
  Query: project_id, start_time, end_time, interval: '1h'|'1d'|'1w'
  Response: { intervals: [{ time, count, unique_errors }] }

GET /api/errors/trends            # 错误趋势（用于图表）
  Query: project_id, days: 7|30|90
  Response: { daily: [{ date, count, resolved }], top_errors: [...] }

GET /api/errors/:id/similar       # 相似错误列表（相同 fingerprint）
```

---

## 接口监控

```
GET /api/network                  # 接口报错列表
  Query: project_id, page, page_size, url, method, status, start_time, end_time
GET /api/network/stats            # 接口报错统计
  Query: project_id, days
  Response: { top_urls, status_distribution, avg_duration }
GET /api/network/:id              # 接口报错详情
```

---

## Source Map

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

---

## AI 分析

```
POST /api/ai/analyze/:error_id    # 触发 AI 分析（异步）
  Response: { task_id, status: 'queued' }

GET  /api/ai/analysis/:error_id   # 获取 AI 分析结果
  Response: { id, error_id, ai_suggestion, ai_confidence, severity_score, ... }
  或 { status: 'pending' } 如果分析中

GET  /api/ai/analyses             # AI 分析历史列表
  Query: project_id, page, model_used, has_suggestion

POST /api/ai/analyze-batch        # 批量触发 AI 分析（相同 fingerprint 的错误）
  Body: { fingerprint, project_id }
```

---

## 仪表盘

```
GET  /api/dashboard/overview      # 仪表盘概览数据
  Query: project_id, days: 7|30
  Response: {
    total_errors, total_network_errors,
    error_trend: [...],
    browser_distribution: [...],
    os_distribution: [...],
    top_errors: [...],
    avg_performance: { fp, fcp, lcp, cls }
  }

GET  /api/dashboard/realtime      # 实时数据（SSE 连接）
  Headers: Accept: text/event-stream
  Events:
    - init: { project_ids, connection_id }
    - error: { id, message, error_type, created_at, project_id }
    - alert: { rule_id, alert_content, severity }
    - heartbeat: { timestamp }
```

---

## 告警管理

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

## 用户与分组

```
GET  /api/users                   # 用户列表（super_admin / admin 可查看全部）
  Query: page, page_size, group_id, role, keyword
POST /api/users                   # 创建用户（admin 以上）
PUT  /api/users/:id               # 更新用户信息/角色
DELETE /api/users/:id             # 删除用户（super_admin）

GET  /api/groups                  # 分组列表
POST /api/groups                  # 创建分组
GET  /api/groups/:id              # 分组详情（含成员列表）
PUT  /api/groups/:id              # 更新分组
DELETE /api/groups/:id            # 删除分组（需无项目）
GET  /api/groups/:id/invite-code  # 获取邀请码
POST /api/groups/:id/join         # 通过邀请码加入分组
```

---

## 埋点管理平台

### 事件定义管理

```
GET    /api/tracking/events                  # 事件定义列表
  Query: project_id, category, status, keyword
POST   /api/tracking/events                  # 创建事件定义
  Body: { event_name, display_name, description, category, properties: [...] }
GET    /api/tracking/events/:event_name      # 事件详情（含属性列表、近7天数据量）
PUT    /api/tracking/events/:event_name      # 更新事件定义
DELETE /api/tracking/events/:event_name      # 删除/弃用事件定义
```

### 事件属性管理

```
GET    /api/tracking/properties              # 属性定义列表
  Query: project_id, event_name（不传则返回通用属性）
POST   /api/tracking/properties              # 创建属性定义
```

### 用户属性管理

```
GET    /api/tracking/user-properties         # 用户属性定义列表
POST   /api/tracking/user-properties         # 创建用户属性定义
```

### 实时事件流（Debug 模式）

```
GET    /api/tracking/live-events             # SSE 实时事件流
  Query: project_id, distinct_id（可选，过滤特定用户）
  Events:
    - track: { event, distinct_id, properties, created_at }
    - profile: { distinct_id, properties }
```

### 事件分析查询

```
POST   /api/tracking/analysis/events         # 事件分析
  Body: {
    project_id,
    event,
    filters: [{ property, operator, value }],
    group_by: ['browser', 'city'],
    metric: 'count' | 'uv' | 'sum' | 'avg',
    metric_property: 'price',
    time_range: { start, end },
    interval: '1h' | '1d' | '1w'
  }
  Response: {
    total: number,
    data: [{ time, value, breakdown: {...} }]
  }
```

### 漏斗分析

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

### 留存分析

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

### 用户画像查询

```
GET    /api/tracking/users                   # 用户列表（含属性筛选）
  Query: project_id, page, filters（JSON 序列化的筛选条件）
GET    /api/tracking/users/:distinct_id      # 用户详情（属性 + 最近事件流水）
GET    /api/tracking/users/:distinct_id/events  # 用户事件时间线
  Query: page, event_name, start_time, end_time
```

---

## 统一响应格式

```typescript
interface ApiResponse<T> {
  code: number;           // 0 = 成功，非0 = 业务错误码
  message: string;        // 错误描述
  data: T;               // 业务数据
  pagination?: {
    page: number;
    page_size: number;
    total: number;
    total_pages: number;
  };
}

// 错误码定义
enum ErrorCode {
  SUCCESS = 0,
  BAD_REQUEST = 400,
  UNAUTHORIZED = 401,
  FORBIDDEN = 403,
  NOT_FOUND = 404,
  RATE_LIMITED = 429,
  INTERNAL_ERROR = 500,
  // 业务错误
  INVALID_APP_KEY = 1001,
  PROJECT_NOT_FOUND = 1002,
  AI_RATE_LIMITED = 2001,
  AI_ANALYSIS_FAILED = 2002,
}
```
