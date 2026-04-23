# JS 监控平台 — 技术设计方案

## 项目概述

构建一个完整的 JS 监控平台，包含 **脚本端 SDK**（可注入任意网站）和 **监控端管理系统**（Vue3 + Vite + Rust），实现前端错误监控、接口监控、AI 智能分析、实时告警和前端埋点信息采集等功能。

---

## 一、项目目录结构

```
js-monitor-platform/
├── sdk/                          # 脚本端 SDK（原生 JS，零依赖）
│   ├── src/
│   │   ├── core/                 # 核心模块
│   │   │   ├── monitor.ts        # 主入口，初始化配置
│   │   │   ├── reporter.ts       # 数据上报模块
│   │   │   ├── store.ts          # 本地缓存/队列
│   │   │   └── utils.ts          # 工具函数
│   │   ├── plugins/              # 监控插件
│   │   │   ├── error.ts          # JS 错误、Promise 错误、资源加载错误
│   │   │   ├── console.ts        # 控制台日志劫持
│   │   │   ├── network.ts        # fetch / XHR 接口监控
│   │   │   ├── performance.ts    # 性能指标（FP/FCP/LCP/CLS/TTFB）
│   │   │   ├── behavior.ts       # 用户行为追踪（代码埋点：track/identify/setUserProperties）
│   │   │   ├── auto-track.ts     # 全埋点：自动采集 $page_view / $element_click / $page_leave
│   │   │   ├── exposure.ts       # 曝光追踪：IntersectionObserver 半自动曝光（data-track-imp）
│   │   │   ├── profile.ts        # 用户画像操作（Profile set/set_once/append/unset）
│   │   │   └── breadcrumb.ts     # 面包屑（用户操作路径）
│   │   ├── core/                 # 核心模块
│   │   │   ├── monitor.ts        # 主入口，初始化配置
│   │   │   ├── reporter.ts       # 数据上报模块
│   │   │   ├── store.ts          # 本地缓存/队列
│   │   │   ├── identity.ts       # 用户身份管理（匿名 ID / 登录 ID / ID 关联）
│   │   │   ├── super-props.ts    # 超级属性管理（register / unregister）
│   │   │   ├── timer.ts          # 事件时长计时器（trackTimerStart / trackTimerEnd）
│   │   │   └── utils.ts          # 工具函数
│   │   ├── types/                # 类型定义
│   │   └── index.ts              # SDK 入口
│   ├── build/                    # 打包输出（UMD / ESM / IIFE）
│   └── package.json
│
├── monitor-web/                  # 监控端前端（Vue3 + Vite + TypeScript）
│   ├── src/
│   │   ├── api/                  # API 请求封装
│   │   ├── views/
│   │   │   ├── dashboard/        # 首页图表概览
│   │   │   ├── project/          # 项目管理（新建/编辑/删除）
│   │   │   ├── errors/           # 错误列表与详情
│   │   │   ├── network/          # 接口报错统计
│   │   │   ├── sourcemap/        # Source Map 上传管理
│   │   │   ├── ai-analysis/      # AI 报错分析结果
│   │   │   ├── tracking/         # 埋点功能模块
│   │   │   │   ├── events/       # 事件定义管理（列表 + 详情）
│   │   │   │   ├── analysis/     # 分析页（事件分析 / 漏斗分析 / 留存分析）
│   │   │   │   ├── users/        # 用户画像（列表 + 详情 + 事件时间线）
│   │   │   │   └── debug/        # 实时事件流调试页
│   │   │   ├── user/             # 用户管理
│   │   │   ├── group/            # 分组/团队管理
│   │   │   └── settings/         # 告警配置、通知设置
│   │   ├── components/
│   │   │   ├── charts/           # ECharts 图表组件
│   │   │   ├── error-list/       # 报错列表组件
│   │   │   ├── breadcrumb/       # 面包屑导航
│   │   │   └── layout/           # 布局组件
│   │   ├── stores/               # Pinia 状态管理
│   │   ├── router/               # Vue Router
│   │   ├── utils/
│   │   └── App.vue
│   ├── package.json
│   └── vite.config.ts
│
├── monitor-server/               # 监控端后端（Rust + Axum + SeaORM）
│   ├── src/
│   │   ├── main.rs               # 服务入口
│   │   ├── config.rs             # 配置管理
│   │   ├── router.rs             # 路由注册
│   │   ├── middleware/
│   │   │   ├── auth.rs           # JWT 鉴权中间件
│   │   │   ├── cors.rs           # 跨域中间件
│   │   │   └── rate_limit.rs     # 限流中间件
│   │   ├── handlers/
│   │   │   ├── sdk.rs            # SDK 数据上报接口（含埋点 track/profile/track_signup）
│   │   │   ├── project.rs        # 项目 CRUD
│   │   │   ├── error.rs          # 错误数据查询/统计
│   │   │   ├── network.rs        # 接口报错查询
│   │   │   ├── sourcemap.rs      # Source Map 上传/解析
│   │   │   ├── ai_analysis.rs    # AI 分析触发/结果获取
│   │   │   ├── auth.rs           # 登录/注册/Token 刷新
│   │   │   ├── user.rs           # 用户管理
│   │   │   ├── group.rs          # 分组管理
│   │   │   ├── alert.rs          # 告警规则/通知配置
│   │   │   ├── dashboard.rs      # 仪表盘统计数据
│   │   │   ├── tracking.rs       # 埋点事件定义/属性管理
│   │   │   ├── track_analysis.rs # 事件分析/漏斗分析/留存分析查询
│   │   │   └── track_users.rs    # 用户画像查询
│   │   ├── services/
│   │   │   ├── ai_service.rs     # AI 分析服务（调用 LLM）
│   │   │   ├── alert_service.rs  # 告警通知服务
│   │   │   ├── sourcemap_service.rs # Source Map 解析
│   │   │   ├── stats_service.rs  # 统计聚合服务
│   │   │   ├── track_service.rs  # 埋点数据处理（写入/预聚合）
│   │   │   └── identity_service.rs  # 用户 ID 关联合并服务
│   │   ├── models/               # SeaORM Entity
│   │   ├── db.rs                 # 数据库连接
│   │   └── lib.rs
│   ├── migrations/               # SeaORM 数据库迁移
│   ├── Cargo.toml
│   └── Dockerfile
│
├── docker-compose.yml            # 一键部署（Rust + PostgreSQL + Redis）
└── README.md
```

---

## 二、技术栈选型

### 脚本端 SDK
| 模块 | 技术 |
|------|------|
| 语言 | TypeScript |
| 构建 | Rollup（输出 UMD / ESM / IIFE 三种格式）|
| 体积 | 目标 < 15KB（gzip） |
| 兼容性 | IE11+（Promise polyfill 可选）|

### 监控端前端
| 模块 | 技术 |
|------|------|
| 框架 | Vue 3 + Composition API |
| 构建 | Vite 6 |
| 状态管理 | Pinia |
| 路由 | Vue Router 4 |
| UI 框架 | Element Plus |
| 图表 | ECharts 5 + vue-echarts |
| HTTP 客户端 | Axios |
| 实时通信 | SSE（Server-Sent Events） |

### 监控端后端
| 模块 | 技术 |
|------|------|
| 框架 | Axum（Rust） |
| ORM | SeaORM |
| 数据库 | PostgreSQL 15 |
| 缓存 | Redis（告警限流、Session、热点数据） |
| 消息队列 | Redis Stream（异步处理告警、AI 分析） |
| 异步运行时 | Tokio |
| Source Map 解析 | sourcemap crate |
| AI 调用 | reqwest → OpenAI/Claude API |

---

## 三、数据库设计

### 3.1 核心表结构

```sql
-- 用户表
CREATE TABLE users (
    id              SERIAL PRIMARY KEY,
    username        VARCHAR(50) NOT NULL UNIQUE,
    email           VARCHAR(100) NOT NULL UNIQUE,
    password_hash   VARCHAR(255) NOT NULL,
    role            VARCHAR(20) NOT NULL DEFAULT 'member',  -- super_admin / admin / owner / member / readonly
    group_id        INTEGER REFERENCES groups(id),
    avatar          VARCHAR(255),
    last_login_at   TIMESTAMPTZ,
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    updated_at      TIMESTAMPTZ DEFAULT NOW()
);

-- 分组/团队表
CREATE TABLE groups (
    id              SERIAL PRIMARY KEY,
    name            VARCHAR(100) NOT NULL,
    description     TEXT,
    owner_id        INTEGER NOT NULL REFERENCES users(id),
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    updated_at      TIMESTAMPTZ DEFAULT NOW()
);

-- 项目表（一个分组下多个项目）
CREATE TABLE projects (
    id              SERIAL PRIMARY KEY,
    name            VARCHAR(100) NOT NULL,
    app_id          VARCHAR(32) NOT NULL UNIQUE,  -- SDK 接入标识
    app_key         VARCHAR(64) NOT NULL UNIQUE,  -- SDK 上报鉴权密钥
    group_id        INTEGER NOT NULL REFERENCES groups(id),
    owner_id        INTEGER NOT NULL REFERENCES users(id),
    description     TEXT,
    alert_threshold INTEGER DEFAULT 10,           -- 1分钟错误数阈值
    alert_webhook   VARCHAR(500),
    data_retention_days INTEGER DEFAULT 30,       -- 数据保留天数（按项目配置）
    environment     VARCHAR(20) DEFAULT 'production',
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    updated_at      TIMESTAMPTZ DEFAULT NOW()
);

-- 项目成员关联表（支持跨分组协作）
CREATE TABLE project_members (
    id              SERIAL PRIMARY KEY,
    project_id      INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    user_id         INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role            VARCHAR(20) NOT NULL DEFAULT 'member',  -- owner / member / readonly
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(project_id, user_id)
);

-- JS 错误日志表（按时间范围分区）
CREATE TABLE js_errors (
    id              BIGSERIAL,
    project_id      INTEGER NOT NULL REFERENCES projects(id),
    app_id          VARCHAR(32) NOT NULL,
    message         TEXT NOT NULL,
    stack           TEXT,
    error_type      VARCHAR(50) NOT NULL,         -- TypeError / ReferenceError / SyntaxError 等
    source_url      VARCHAR(500),
    line            INTEGER,
    column          INTEGER,
    user_agent      VARCHAR(500),
    browser         VARCHAR(50),
    browser_version VARCHAR(30),
    os              VARCHAR(50),
    os_version      VARCHAR(30),
    device          VARCHAR(50),
    device_type     VARCHAR(20),                  -- desktop / mobile / tablet
    device_memory   VARCHAR(20),
    hardware_concurrency INTEGER,
    connection_type VARCHAR(20),                  -- 4g / wifi / unknown
    ip              INET,
    country         VARCHAR(50),
    city            VARCHAR(50),
    sdk_version     VARCHAR(20),
    release         VARCHAR(50),                  -- 代码版本
    environment     VARCHAR(20),                  -- production / staging / development
    url             VARCHAR(500),                 -- 页面 URL
    referrer        VARCHAR(500),
    viewport        VARCHAR(30),
    screen_resolution VARCHAR(30),
    language        VARCHAR(10),
    timezone        VARCHAR(50),
    breadcrumb      JSONB,                        -- 用户操作路径
    extra           JSONB,                        -- 扩展字段
    fingerprint     VARCHAR(64),                  -- 错误指纹（用于聚合相同错误）
    is_ai_analyzed  BOOLEAN DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, created_at)
) PARTITION BY RANGE (created_at);

-- 创建按月分区（示例：2024年1月）
CREATE TABLE js_errors_2024_01 PARTITION OF js_errors
    FOR VALUES FROM ('2024-01-01') TO ('2024-02-01');

-- 接口报错日志表
CREATE TABLE network_errors (
    id              BIGSERIAL PRIMARY KEY,
    project_id      INTEGER NOT NULL REFERENCES projects(id),
    app_id          VARCHAR(32) NOT NULL,
    url             VARCHAR(500) NOT NULL,
    method          VARCHAR(10) NOT NULL,
    status          INTEGER,
    request_headers JSONB,
    request_body    TEXT,                        -- 已脱敏
    response_headers JSONB,
    response_text   TEXT,                        -- 截断至 10KB
    duration        INTEGER,                     -- 请求耗时（ms）
    error_type      VARCHAR(50),                 -- timeout / error / abort / http_error
    user_agent      VARCHAR(500),
    ip              INET,
    browser         VARCHAR(50),
    os              VARCHAR(50),
    device          VARCHAR(50),
    sdk_version     VARCHAR(20),
    release         VARCHAR(50),
    environment     VARCHAR(20),
    page_url        VARCHAR(500),
    created_at      TIMESTAMPTZ DEFAULT NOW()
);

-- 性能数据表（采样上报）
CREATE TABLE performance_data (
    id              BIGSERIAL PRIMARY KEY,
    project_id      INTEGER NOT NULL REFERENCES projects(id),
    app_id          VARCHAR(32) NOT NULL,
    url             VARCHAR(500),
    fp              INTEGER,                     -- First Paint (ms)
    fcp             INTEGER,                     -- First Contentful Paint (ms)
    lcp             INTEGER,                     -- Largest Contentful Paint (ms)
    cls             NUMERIC(10, 4),              -- Cumulative Layout Shift
    ttfb            INTEGER,                     -- Time to First Byte (ms)
    tti             INTEGER,                     -- Time to Interactive (ms)
    load_time       INTEGER,                     -- 页面完全加载时间 (ms)
    dns_time        INTEGER,
    tcp_time        INTEGER,
    ssl_time        INTEGER,
    dom_parse_time  INTEGER,
    resource_count  INTEGER,
    resource_size   INTEGER,                     -- 总资源大小 (KB)
    user_agent      VARCHAR(500),
    browser         VARCHAR(50),
    device_type     VARCHAR(20),
    sdk_version     VARCHAR(20),
    release         VARCHAR(50),
    environment     VARCHAR(20),
    created_at      TIMESTAMPTZ DEFAULT NOW()
);

-- Source Map 文件表
CREATE TABLE source_maps (
    id              SERIAL PRIMARY KEY,
    project_id      INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    version         VARCHAR(50) NOT NULL,         -- release 版本号
    filename        VARCHAR(255) NOT NULL,        -- map 文件名
    original_filename VARCHAR(255),               -- 原始 JS 文件名
    file_path       VARCHAR(500) NOT NULL,        -- 服务器存储路径
    file_size       INTEGER,
    uploaded_by     INTEGER REFERENCES users(id),
    uploaded_at     TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(project_id, version, filename)
);

-- AI 分析结果表
CREATE TABLE ai_analyses (
    id              SERIAL PRIMARY KEY,
    error_id        BIGINT NOT NULL,
    project_id      INTEGER NOT NULL REFERENCES projects(id),
    error_type      VARCHAR(50),
    original_stack  TEXT,
    analyzed_stack  TEXT,                        -- Source Map 解析后的堆栈
    ai_suggestion   TEXT,                        -- AI 修复建议
    ai_confidence   NUMERIC(3, 2),               -- 置信度 0.00-1.00
    probable_file   VARCHAR(255),
    probable_line   INTEGER,
    probable_code   TEXT,                        -- 相关代码片段
    severity_score  INTEGER,                     -- 1-5 严重程度
    model_used      VARCHAR(50),                 -- deepseek / qwen / wenxin
    prompt_tokens   INTEGER,
    completion_tokens INTEGER,
    total_tokens    INTEGER,
    cost_ms         INTEGER,                     -- AI 调用耗时
    is_cached       BOOLEAN DEFAULT FALSE,       -- 是否命中缓存
    cache_key       VARCHAR(64),                 -- 缓存键（基于错误指纹）
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    updated_at      TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(error_id)
);

-- 告警规则表
CREATE TABLE alert_rules (
    id              SERIAL PRIMARY KEY,
    project_id      INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name            VARCHAR(100) NOT NULL,
    rule_type       VARCHAR(30) NOT NULL,        -- error_spike / failure_rate / new_error / p0_error
    threshold       INTEGER NOT NULL,            -- 阈值（根据类型不同含义不同）
    interval_minutes INTEGER DEFAULT 1,          -- 统计时间窗口
    webhook_url     VARCHAR(500),
    email           VARCHAR(100),
    enabled         BOOLEAN DEFAULT TRUE,
    created_by      INTEGER NOT NULL REFERENCES users(id),
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    updated_at      TIMESTAMPTZ DEFAULT NOW()
);

-- 告警记录表
CREATE TABLE alert_logs (
    id              SERIAL PRIMARY KEY,
    rule_id         INTEGER NOT NULL REFERENCES alert_rules(id),
    project_id      INTEGER NOT NULL REFERENCES projects(id),
    alert_content   TEXT NOT NULL,
    status          VARCHAR(20) DEFAULT 'pending', -- pending / sent / failed
    error_count     INTEGER,                     -- 触发时的错误数量
    sent_at         TIMESTAMPTZ,
    created_at      TIMESTAMPTZ DEFAULT NOW()
);

-- 错误聚合统计表（按小时预聚合，加速查询）
CREATE TABLE error_stats_hourly (
    id              SERIAL PRIMARY KEY,
    project_id      INTEGER NOT NULL REFERENCES projects(id),
    hour            TIMESTAMPTZ NOT NULL,
    error_type      VARCHAR(50),
    fingerprint     VARCHAR(64),
    message_pattern VARCHAR(255),                -- 错误消息模板
    count           INTEGER DEFAULT 0,
    affected_users  INTEGER DEFAULT 0,
    affected_pages  INTEGER DEFAULT 0,
    browser_breakdown JSONB,
    os_breakdown    JSONB,
    UNIQUE(project_id, hour, fingerprint)
);
```

### 3.2 索引设计

```sql
-- js_errors 核心查询索引
CREATE INDEX idx_js_errors_project_time ON js_errors(project_id, created_at DESC);
CREATE INDEX idx_js_errors_fingerprint ON js_errors(fingerprint, created_at DESC);
CREATE INDEX idx_js_errors_type ON js_errors(error_type, created_at DESC);
CREATE INDEX idx_js_errors_url ON js_errors(source_url) WHERE source_url IS NOT NULL;
CREATE INDEX idx_js_errors_browser ON js_errors(browser, created_at DESC);
CREATE INDEX idx_js_errors_release ON js_errors(release, created_at DESC);
CREATE INDEX idx_js_errors_is_ai_analyzed ON js_errors(is_ai_analyzed) WHERE is_ai_analyzed = FALSE;

-- network_errors 索引
CREATE INDEX idx_network_errors_project_time ON network_errors(project_id, created_at DESC);
CREATE INDEX idx_network_errors_url ON network_errors(url);
CREATE INDEX idx_network_errors_status ON network_errors(status);

-- performance_data 索引
CREATE INDEX idx_perf_project_time ON performance_data(project_id, created_at DESC);
CREATE INDEX idx_perf_url ON performance_data(url);

-- AI 分析索引
CREATE INDEX idx_ai_analysis_project ON ai_analyses(project_id, created_at DESC);
CREATE INDEX idx_ai_analysis_cache ON ai_analyses(cache_key) WHERE is_cached = TRUE;

-- 告警索引
CREATE INDEX idx_alert_logs_project_time ON alert_logs(project_id, created_at DESC);
CREATE INDEX idx_alert_logs_status ON alert_logs(status) WHERE status = 'pending';

-- 统计表索引
CREATE INDEX idx_stats_project_hour ON error_stats_hourly(project_id, hour DESC);
```

### 3.3 数据保留与清理策略

| 表名 | 保留策略 | 清理方式 |
|------|---------|---------|
| js_errors | 按项目配置（默认30天，可配置） | 定时任务删除旧分区 |
| network_errors | 同 js_errors | 定时任务 DELETE |
| performance_data | 90天 | 定时任务 DELETE |
| ai_analyses | 永久保留（数据量小） | 不清理 |
| alert_logs | 180天 | 定时任务 DELETE |
| error_stats_hourly | 365天 | 定时任务 DELETE |

**清理机制**：后端启动一个定时任务（每天凌晨3点），删除超期的 `js_errors` 分区和其他表的历史数据。

---

## 四、SDK 采集与上报设计

### 4.1 上报策略（混合策略）

| 错误级别 | 处理方式 | 说明 |
|---------|---------|------|
| P0（SyntaxError, ReferenceError） | 实时上报 | 立即发送，不等待批量 |
| P1（TypeError, 资源加载失败） | 批量上报 | 5秒或队列满10条时发送 |
| 性能数据 | 采样 + 批量 | 默认采样率 10%，批量上报 |
| 接口错误 | 实时上报 | 接口失败立即上报 |

### 4.2 本地存储与队列

```typescript
interface StoreConfig {
  maxQueueSize: 100;           // 最大队列长度
  flushInterval: 5000;         // 批量上报间隔（ms）
  retryMaxCount: 3;            // 最大重试次数
  retryInterval: 30000;        // 重试间隔（ms）
  storageType: 'indexedDB';    // 优先 IndexedDB，降级 localStorage
}
```

**队列管理**：
- 上报失败时存入 IndexedDB，网络恢复后按 FIFO 顺序补发
- 队列满时丢弃最旧的数据（优先保留 P0 错误）
- 页面 `beforeunload` 时强制 flush 队列

### 4.3 错误去重（客户端）

```typescript
// 错误指纹生成规则
fingerprint = hash(errorType + ':' + message + ':' + sourceUrl + ':' + line + ':' + column)

// 去重策略
interface DedupConfig {
  windowMs: 60000;             // 1分钟窗口
  maxCount: 10;                // 1分钟内相同指纹最多上报10次
  silentAfterMax: true;        // 超过阈值后静默丢弃
}
```

### 4.4 敏感数据脱敏

| 数据类型 | 脱敏规则 |
|---------|---------|
| 请求/响应 Body | 自动检测并替换：`password`, `token`, `secret`, `apiKey`, `authorization` 等字段值为 `[REDACTED]` |
| URL 查询参数 | 自动移除：`token`, `auth`, `key`, `secret` 等参数 |
| 用户输入 | 输入框 type=password 的内容替换为 `[REDACTED]` |
| Cookie | 默认不上报，如需上报则过滤敏感字段 |

### 4.5 采集数据详情

#### 页面信息
- `url` / `referrer` / `title`
- `viewport` 尺寸 / `screen` 分辨率
- `language` / `timezone`

#### 设备信息
- `userAgent`（解析出 browser、browserVersion、os、osVersion、device）
- `deviceMemory` / `hardwareConcurrency`
- `connection`（网络类型：4G/WiFi）

#### 错误信息
- 错误类型：`js` / `promise` / `resource` / `vue` / `react`
- 错误消息、完整堆栈
- 资源加载失败的 URL
- Vue/React 组件名（如检测到框架）

#### 接口信息
- 请求 URL、Method、Headers、Body（脱敏）
- 响应 Status、Headers、Body（截断至 10KB）
- 请求耗时

#### 面包屑（操作路径）
- 点击事件（目标元素 selector）
- 路由跳转
- 控制台日志
- 接口请求/响应
- 用户输入（脱敏）

#### 性能指标
- FP、FCP、LCP、CLS、TTFB、FID
- 页面加载时间、DNS 解析时间

---

## 五、API 设计

### 5.1 SDK 公开接口

```
POST /api/v1/collect              # SDK 数据上报（按 app_id + app_key 校验）
  Headers: X-App-Id, X-App-Key
  Body: { type: 'error'|'network'|'performance'|'breadcrumb', data: {...} }
  Response: { code: 0, message: 'ok' }

GET  /api/v1/collect/health       # SDK 健康检查
  Response: { status: 'ok', version: '1.0.0' }
```

### 5.2 认证接口

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

**Token 机制**：
- Access Token：JWT，有效期 2 小时，存内存
- Refresh Token：JWT，有效期 7 天，存 httpOnly cookie
- 前端 Axios 拦截器自动刷新 access_token

### 5.3 项目管理

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

### 5.4 错误管理

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

### 5.5 接口监控

```
GET /api/network                  # 接口报错列表
  Query: project_id, page, page_size, url, method, status, start_time, end_time
GET /api/network/stats            # 接口报错统计
  Query: project_id, days
  Response: { top_urls, status_distribution, avg_duration }
GET /api/network/:id              # 接口报错详情
```

### 5.6 Source Map

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

### 5.7 AI 分析

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

**AI 限流策略**：
- 单个项目每分钟最多 20 次 AI 分析请求
- 相同 fingerprint 的分析结果缓存 7 天
- 超出限流返回 429，前端提示「分析队列已满，请稍后重试」

### 5.8 仪表盘

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

**SSE 实现细节**：
- 连接建立时，后端记录用户 ID 与连接映射
- 用户登录后，推送其所有有权限项目的实时告警
- 心跳间隔：30 秒
- 断线重连：前端自动重连，指数退避（1s → 2s → 4s → 8s，最大 30s）
- 消息格式：`event: error\ndata: {...}\n\n`

### 5.9 告警管理

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

### 5.10 用户与分组

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

### 5.11 统一响应格式

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

---

## 六、AI 分析模块设计

### 6.1 触发时机

| 触发方式 | 条件 | 优先级 |
|---------|------|--------|
| 手动触发 | 用户点击「AI 分析」按钮 | 高（立即执行） |
| 自动触发（新错误） | 新 fingerprint 首次出现 | 中（队列异步处理） |
| 自动触发（累积阈值） | 相同错误 1 小时内出现 > 50 次 | 中（队列异步处理） |
| 批量触发 | 用户选择多个错误批量分析 | 低（队列异步处理） |

### 6.2 AI Prompt 设计

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

### 6.3 AI 服务实现

```rust
// AI 服务调用流程
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

### 6.4 AI 调用配置

```toml
[ai]
provider = "deepseek"           # deepseek / qwen / wenxin
api_key = "${AI_API_KEY}"
api_base = "https://api.deepseek.com/v1"
model = "deepseek-chat"
max_tokens = 2048
temperature = 0.3               # 低温度，输出更确定性
request_timeout = 60            # 秒

# 限流配置
rate_limit_per_minute = 60
rate_limit_per_project = 20

# 缓存配置
cache_ttl_days = 7
```

### 6.5 降级策略

| 场景 | 处理方式 |
|------|---------|
| AI 服务超时（>60s） | 返回「分析超时，请稍后重试」，任务入队列重试 |
| AI 服务返回格式错误 | 记录日志，返回「AI 分析失败，请手动分析」 |
| AI API 限流（429） | 指数退避重试，最大重试 3 次 |
| AI 服务完全不可用 | 关闭自动触发，仅保留手动触发入口 |

---

## 七、实时告警设计

### 7.1 告警规则类型

| 规则类型 | 触发条件 | 参数 |
|---------|---------|------|
| error_spike | 1 分钟内错误数 > N | threshold: 错误数 |
| failure_rate | 接口失败率 > X% | threshold: 百分比 (1-100) |
| new_error | 出现新的 fingerprint | 无阈值 |
| p0_error | 出现 P0 级错误 | 无阈值 |
| error_trend | 错误数较上小时增长 X% | threshold: 百分比 |

### 7.2 告警处理流程

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

### 7.3 告警去重与升级

```rust
// 去重逻辑
const DEDUP_WINDOW_MINUTES: i64 = 10;

async fn should_alert(rule_id: i32, fingerprint: &str) -> bool {
    let key = format!("alert:{}:{}", rule_id, fingerprint);
    // Redis SETNX，10 分钟过期
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
    let recent_alerts = count_alerts_in_window(rule_id, 30).await?; // 30 分钟内
    if recent_alerts >= 3 {
        // 升级：提升告警级别，增加通知渠道
        send_escalated_alert(rule_id, project_id).await?;
    }
}
```

### 7.4 通知渠道

| 渠道 | Phase 1 | Phase 2 |
|------|---------|---------|
| SSE 实时推送 | ✅ 实现 | ✅ |
| Webhook（飞书/钉钉/企微/Slack） | ⬜ 预留接口 | ✅ 实现 |
| 邮件通知 | ⬜ 预留接口 | ✅ 实现 |
| 短信通知 | ⬜ 不实现 | ⬜ 不实现 |

**SSE 推送范围**：用户登录后，推送其所有有权限项目的告警。

### 7.5 告警消息格式

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

---

## 八、Source Map 支持

### 8.1 上传流程

1. 用户 CI/CD 构建后，调用 API 上传 `.map` 文件
2. 后端校验文件格式（必须是有效的 Source Map）
3. 文件存储至本地磁盘或对象存储（S3/MinIO）
4. 数据库记录文件元信息

### 8.2 解析流程

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

### 8.3 自动关联

- SDK 上报时携带 `release` 字段（如 git commit hash 或版本号）
- 后端收到错误后，自动查找 `project_id + release` 匹配的 Source Map
- 解析后的原始堆栈存入 `ai_analyses.analyzed_stack`

---

## 九、权限模型

### 9.1 角色定义

| 角色 | 权限范围 | 说明 |
|------|---------|------|
| **super_admin** | 全系统管理 | 创建/删除分组，管理所有用户，系统配置 |
| **admin** | 分组级管理 | 创建项目，管理分组成员，查看分组所有项目 |
| **owner** | 项目级管理 | 项目 Owner，可修改项目设置、添加成员、配置告警 |
| **member** | 项目级读写 | 查看错误、触发 AI 分析、创建告警规则 |
| **readonly** | 项目级只读 | 仅查看错误和统计数据，不可修改任何配置 |

### 10.2 权限矩阵

| 操作 | super_admin | admin | owner | member | readonly |
|------|:-----------:|:-----:|:-----:|:------:|:--------:|
| 创建分组 | ✅ | ❌ | ❌ | ❌ | ❌ |
| 删除分组 | ✅ | ❌ | ❌ | ❌ | ❌ |
| 创建项目 | ✅ | ✅ | ❌ | ❌ | ❌ |
| 删除项目 | ✅ | ✅ | ✅ | ❌ | ❌ |
| 修改项目设置 | ✅ | ✅ | ✅ | ❌ | ❌ |
| 添加项目成员 | ✅ | ✅ | ✅ | ❌ | ❌ |
| 查看错误列表 | ✅ | ✅ | ✅ | ✅ | ✅ |
| 触发 AI 分析 | ✅ | ✅ | ✅ | ✅ | ❌ |
| 创建告警规则 | ✅ | ✅ | ✅ | ✅ | ❌ |
| 上传 Source Map | ✅ | ✅ | ✅ | ✅ | ❌ |
| 查看用户列表 | ✅ | ✅ | ❌ | ❌ | ❌ |
| 修改用户角色 | ✅ | ✅ | ❌ | ❌ | ❌ |

### 10.3 权限校验中间件

```rust
// Axum 中间件示例
async fn require_role(
    role: Role,
) -> impl Fn(Request, Next) -> Response {
    move |req, next| {
        let user = req.extensions().get::<CurrentUser>().unwrap();
        if user.role.level() < role.level() {
            return StatusCode::FORBIDDEN.into_response();
        }
        next.run(req).await
    }
}

// 路由使用
let project_routes = Router::new()
    .route("/", post(create_project))
    .layer(middleware::from_fn(require_role(Role::Admin)))
    .route("/:id", put(update_project))
    .layer(middleware::from_fn(require_project_owner));
```

---

## 十一、部署与运维

### 11.1 部署架构

```
                    ┌─────────────┐
                    │   Nginx     │  ← SSL 终止、反向代理、静态文件
                    │  (443/80)   │
                    └──────┬──────┘
                           │
           ┌───────────────┼───────────────┐
           │               │               │
    ┌──────▼──────┐ ┌──────▼──────┐ ┌──────▼──────┐
    │ monitor-web │ │monitor-server│ │  SSE 长连接  │
    │  (静态文件)  │ │   (API)     │ │  (独立端口)  │
    └─────────────┘ └──────┬──────┘ └─────────────┘
                           │
              ┌────────────┼────────────┐
              │            │            │
       ┌──────▼──────┐ ┌───▼───┐ ┌─────▼─────┐
       │  PostgreSQL │ │ Redis │ │  (S3/本地) │
       │    (15)     │ │  (7)  │ │ Source Map │
       └─────────────┘ └───────┘ └───────────┘
```

### 11.2 Docker Compose 配置

```yaml
version: '3.8'

services:
  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf
      - ./ssl:/etc/nginx/ssl
      - ./monitor-web/dist:/usr/share/nginx/html
    depends_on:
      - monitor-server

  postgres:
    image: postgres:15-alpine
    environment:
      POSTGRES_USER: monitor
      POSTGRES_PASSWORD: ${DB_PASSWORD}
      POSTGRES_DB: js_monitor
    volumes:
      - pgdata:/var/lib/postgresql/data
      - ./init.sql:/docker-entrypoint-initdb.d/init.sql
    ports:
      - "5432:5432"

  redis:
    image: redis:7-alpine
    volumes:
      - redisdata:/data
    command: redis-server --appendonly yes

  monitor-server:
    build: ./monitor-server
    environment:
      DATABASE_URL: postgres://monitor:${DB_PASSWORD}@postgres/js_monitor
      REDIS_URL: redis://redis:6379
      AI_API_KEY: ${AI_API_KEY}
      JWT_SECRET: ${JWT_SECRET}
      RUST_LOG: info
    ports:
      - "8080:8080"
      - "8081:8081"  # SSE 端口
    depends_on:
      - postgres
      - redis
    volumes:
      - ./uploads:/app/uploads

volumes:
  pgdata:
  redisdata:
```

### 11.3 Nginx 配置

```nginx
events {
    worker_connections 1024;
}

http {
    upstream api {
        server monitor-server:8080;
    }

    upstream sse {
        server monitor-server:8081;
    }

    server {
        listen 80;
        server_name monitor.example.com;
        return 301 https://$server_name$request_uri;
    }

    server {
        listen 443 ssl http2;
        server_name monitor.example.com;

        ssl_certificate /etc/nginx/ssl/cert.pem;
        ssl_certificate_key /etc/nginx/ssl/key.pem;

        # 静态文件（前端）
        location / {
            root /usr/share/nginx/html;
            try_files $uri $uri/ /index.html;
            expires 1d;
        }

        # API
        location /api/ {
            proxy_pass http://api;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_read_timeout 30s;
        }

        # SSE 长连接
        location /api/dashboard/realtime {
            proxy_pass http://sse;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_http_version 1.1;
            proxy_set_header Connection '';
            proxy_buffering off;
            proxy_cache off;
            proxy_read_timeout 3600s;
        }

        # SDK 上报（高并发，单独限流）
        location /api/v1/collect {
            proxy_pass http://api;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            limit_req zone=collect burst=100 nodelay;
        }
    }
}
```

### 11.4 日志与监控

| 组件 | 日志位置 | 轮转策略 |
|------|---------|---------|
| Nginx | /var/log/nginx/ | logrotate 每日 |
| Rust 后端 | stdout → docker logs | docker 日志驱动 |
| PostgreSQL | /var/lib/postgresql/data/log/ | 7 天自动清理 |

**自身监控**：
- 后端暴露 `/health` 和 `/metrics` 端点（Prometheus 格式）
- 关键指标：QPS、错误率、P99 延迟、AI 调用成功率、告警延迟

---

## 十二、AI 分析模块设计

### 12.1 触发时机

- 用户手动点击「AI 分析」按钮
- 新错误首次出现时自动触发（可配置）
- 相同错误累积超过阈值时触发

### 12.2 AI Prompt 设计

```
你是一位前端错误分析专家。请分析以下报错信息：

【错误信息】
{error_message}

【错误堆栈】
{stack_trace}

【Source Map 解析后的堆栈】（如有）
{mapped_stack}

【相关代码片段】（如有）
{code_snippet}

【页面信息】
URL: {url}
浏览器: {browser}

请给出：
1. 错误原因分析（2-3 句话）
2. 可能的修复方案（含代码示例）
3. 错误严重程度评估（1-5 分）
4. 受影响的文件和行号
```

### 12.3 扩展：自动定位代码

- 解析 Source Map 获取原始文件、行号、列号
- 读取项目上传的原始代码文件（如提供）
- 将代码片段一并送入 AI 分析

---

## 十三、实时告警设计

### 13.1 告警规则类型

- 错误数量激增：1 分钟内错误数 > N
- 接口失败率：失败率 > X%
- 新错误类型出现
- P0 级错误（如 SyntaxError、ReferenceError）

### 13.2 通知渠道

- WebSocket / SSE 推送至监控端（前端弹窗/声音提醒）
- Webhook（飞书/钉钉/企业微信/Slack）
- 邮件通知

### 13.3 告警去重

- 相同错误在 10 分钟内只告警一次
- 告警升级：10 分钟内持续报错则提升告警级别

---

## 十四、Source Map 支持

1. 用户在监控端上传 `.map` 文件，关联项目 + 版本
2. SDK 上报时携带 `release` 字段
3. 后端收到错误后，根据 `release` 匹配对应的 Source Map
4. 使用 `sourcemap` crate 解析堆栈，定位到原始源码位置
5. AI 分析时传入解析后的堆栈和源码片段

---

## 十六、部署方案

```yaml
# docker-compose.yml
services:
  postgres:
    image: postgres:15
    volumes:
      - pgdata:/var/lib/postgresql/data

  redis:
    image: redis:7-alpine

  monitor-server:
    build: ./monitor-server
    ports:
      - "8080:8080"
    depends_on:
      - postgres
      - redis

  monitor-web:
    build: ./monitor-web
    ports:
      - "80:80"
    depends_on:
      - monitor-server
```

---

## 二十、用户行为埋点模块设计

> 参考神策数据（代码埋点+全埋点+用户体系）、GrowingIO（无埋点/自动采集）、Mixpanel/Amplitude（事件驱动+用户画像）的最佳实践，在现有监控平台中集成完整的行为埋点能力。

### 20.1 埋点能力对比与选型

| 能力 | 神策数据 | GrowingIO | Mixpanel/Amplitude | 本平台实现 |
|------|---------|-----------|-------------------|-----------|
| 代码埋点 | ✅ | ✅ | ✅ | ✅ Phase 1 |
| 全埋点（无埋点） | ✅ | ✅（核心强项） | ✅ | ✅ Phase 2 |
| 用户识别（匿名→登录） | ✅ IDM 2.0/3.0 | ✅ | ✅ | ✅ Phase 1 |
| 用户属性（Profile） | ✅ | ✅ | ✅ People API | ✅ Phase 1 |
| 超级属性（全局附加） | ✅ | ✅ | ✅ Super Properties | ✅ Phase 1 |
| 事件时长统计 | ✅ | ✅ | ✅ | ✅ Phase 1 |
| 元素曝光追踪 | ✅ | ✅（半自动 imp） | ❌ | ✅ Phase 2 |
| 漏斗分析 | ✅ | ✅ | ✅ | ✅ Phase 2 |
| 留存分析 | ✅ | ✅ | ✅ | ✅ Phase 3 |
| 埋点管理平台 | ✅ | ✅ | ✅ | ✅ Phase 2 |
| 可视化圈选 | ✅ | ✅ | ❌ | ⬜ Phase 4（预留） |

---

### 20.2 SDK 埋点 API 设计

#### 20.2.1 初始化

```typescript
// SDK 初始化（新增埋点配置项）
interface TrackingConfig {
  // 代码埋点
  enableTracking: boolean;              // 是否开启埋点（默认 true）

  // 全埋点（无埋点自动采集）
  autoTrack: {
    pageView: boolean;                  // 自动采集页面浏览（$page_view）
    click: boolean;                     // 自动采集元素点击（$element_click）
    pageLeave: boolean;                 // 自动采集页面离开+停留时长（$page_leave）
    exposure: boolean;                  // 半自动曝光采集（需配合 data-track-imp）
  };

  // 用户身份
  anonymousIdPrefix: string;            // 匿名 ID 前缀（默认 'anon_'）

  // 上报配置（可与现有 reporter 共用）
  trackFlushInterval: number;           // 埋点批量上报间隔（默认 3000ms）
  trackMaxBatchSize: number;            // 埋点批量上报最大条数（默认 20）
}

// 初始化示例
Monitor.init({
  appId: 'your-app-id',
  appKey: 'your-app-key',
  server: 'https://monitor.example.com',
  tracking: {
    enableTracking: true,
    autoTrack: {
      pageView: true,
      click: true,
      pageLeave: true,
      exposure: false,
    },
  },
});
```

#### 20.2.2 代码埋点核心 API

```typescript
// ① 追踪自定义事件（对标 Mixpanel.track / 神策 track）
Monitor.track(eventName: string, properties?: Record<string, any>): void

// 示例
Monitor.track('purchase', {
  product_id: 'sku_001',
  product_name: '商品A',
  price: 99.9,
  quantity: 2,
  currency: 'CNY',
  payment_method: 'alipay',
});

// ② 用户识别：匿名用户 → 登录用户（对标 Mixpanel.identify / 神策 login）
Monitor.identify(userId: string): void

// 示例（用户登录时调用）
Monitor.identify('user_123456');

// ③ 设置用户属性（对标 Mixpanel.people.set / Amplitude setUserProperties）
Monitor.setUserProperties(properties: Record<string, any>): void

// 示例
Monitor.setUserProperties({
  $name: '张三',
  $email: 'zhangsan@example.com',
  membership: 'premium',
  signup_date: '2024-01-01',
  city: '北京',
});

// ④ 用户属性追加（列表类型，对标神策 profileAppend）
Monitor.appendUserProperties(properties: Record<string, string[]>): void

// 示例
Monitor.appendUserProperties({ tags: ['新用户', '活跃'] });

// ⑤ 设置全局超级属性（每次事件自动附加，对标 Mixpanel.register）
Monitor.registerSuperProperties(properties: Record<string, any>): void

// 示例
Monitor.registerSuperProperties({
  app_version: '2.1.0',
  channel: 'organic',
  experiment_group: 'A',
});

// ⑥ 清除超级属性
Monitor.unregisterSuperProperty(propertyName: string): void
Monitor.clearSuperProperties(): void

// ⑦ 事件时长统计（对标神策 trackTimerStart / trackTimerEnd）
Monitor.trackTimerStart(eventName: string): void
Monitor.trackTimerEnd(eventName: string, properties?: Record<string, any>): void

// 示例（统计视频播放时长）
Monitor.trackTimerStart('video_play');
// ... 用户播放视频 ...
Monitor.trackTimerEnd('video_play', { video_id: 'v_001', title: '教程1' });
// 自动附加 $event_duration 字段（单位：秒）

// ⑧ 用户注销（对标 Mixpanel.reset / 神策 logout）
Monitor.logout(): void

// ⑨ 设置匿名 ID（高级用法）
Monitor.identify_anonymous(anonymousId: string): void
```

#### 20.2.3 全埋点（无埋点）自动采集

```typescript
// SDK 内部自动采集的预置事件（无需手动调用）

// $page_view - 页面浏览事件（SPA 路由切换时自动发送）
{
  event: '$page_view',
  properties: {
    $page_url: 'https://example.com/product/123',
    $page_title: '商品详情页',
    $referrer: 'https://example.com/home',
    $referrer_title: '首页',
    $viewport_width: 1920,
    $viewport_height: 1080,
    $is_first_visit: false,           // 是否首次访问
    $is_first_day: false,             // 是否首日访问
  }
}

// $element_click - 元素点击事件（全量捕获 DOM 点击，可配置过滤）
{
  event: '$element_click',
  properties: {
    $element_id: 'btn-submit',
    $element_class: 'btn btn-primary',
    $element_type: 'button',
    $element_name: '立即购买',
    $element_content: '立即购买',     // 按钮文字
    $element_path: 'body > div > button#btn-submit',  // CSS selector
    $element_xpath: '/html/body/div/button',
    $page_url: 'https://example.com/product/123',
    $page_title: '商品详情页',
    $page_x: 120,                     // 点击坐标
    $page_y: 350,
  }
}

// $page_leave - 页面离开事件（含停留时长）
{
  event: '$page_leave',
  properties: {
    $page_url: 'https://example.com/product/123',
    $page_title: '商品详情页',
    $stay_duration: 45.2,            // 停留秒数
    $leave_reason: 'navigation',     // navigation / close / other
  }
}

// $app_start - 会话开始（用户首次进入或刷新页面）
// $form_submit - 表单提交
```

#### 20.2.4 元素曝光半自动采集（对标 GrowingIO imp）

```html
<!-- 标记需要追踪曝光的元素 -->
<div
  data-track-imp="true"
  data-track-event="product_exposure"
  data-track-attrs='{"product_id": "sku_001", "position": 3, "list": "首页推荐"}'
>
  商品卡片内容
</div>
```

```typescript
// SDK 内部使用 IntersectionObserver 监听元素可见性
// 当元素从不可见→可见时，自动发送 data-track-event 指定的埋点事件
// 支持 once 模式（只触发一次）和 always 模式（每次出现都触发）
```

---

### 20.3 用户身份体系设计

借鉴神策 IDM 2.0 的简易用户关联方案：

```
┌─────────────────────────────────────────────────────┐
│                  用户身份生命周期                      │
│                                                     │
│  访问网站                                            │
│    ↓                                                │
│  分配匿名 ID（UUID，存 localStorage）                 │
│    ↓ 用户登录                                        │
│  调用 identify(userId)                               │
│    ↓                                                │
│  后端关联：匿名 ID → 登录 ID（profile_merge）          │
│    ↓ 用户登出                                        │
│  调用 logout()，重置为新匿名 ID                       │
└─────────────────────────────────────────────────────┘
```

```typescript
// 匿名 ID 生成规则
function generateAnonymousId(): string {
  // 优先读取 localStorage 已有的 anonymous_id
  const stored = localStorage.getItem('__monitor_anon_id');
  if (stored) return stored;
  // 否则生成新的 UUID
  const id = 'anon_' + crypto.randomUUID();
  localStorage.setItem('__monitor_anon_id', id);
  return id;
}

// 事件上报时自动附加 distinct_id
interface TrackPayload {
  distinct_id: string;         // 当前标识：登录前=匿名ID，登录后=用户ID
  anonymous_id: string;        // 始终为设备匿名 ID
  is_login_id: boolean;        // distinct_id 是否为登录 ID
  event: string;
  properties: Record<string, any>;
  time: number;                // 事件发生时间戳（ms）
  // ... 其他公共属性
}
```

---

### 20.4 数据库设计（埋点相关）

#### 20.4.1 用户事件表（核心宽表）

```sql
-- 用户行为事件表（埋点数据，按月分区）
CREATE TABLE track_events (
    id              BIGSERIAL,
    project_id      INTEGER NOT NULL REFERENCES projects(id),
    app_id          VARCHAR(32) NOT NULL,

    -- 用户身份
    distinct_id     VARCHAR(128) NOT NULL,          -- 当前用户标识（登录ID或匿名ID）
    anonymous_id    VARCHAR(128),                   -- 设备匿名 ID
    user_id         VARCHAR(128),                   -- 登录用户 ID（登录后才有）
    is_login_id     BOOLEAN DEFAULT FALSE,          -- distinct_id 是否为登录 ID

    -- 事件核心字段
    event           VARCHAR(128) NOT NULL,          -- 事件名（如 purchase / $page_view）
    event_type      VARCHAR(20) DEFAULT 'custom',   -- custom / auto / page_view / click / exposure
    properties      JSONB,                          -- 事件属性（所有自定义属性）

    -- 超级属性（每次自动附加的全局属性，冗余到此列便于查询）
    super_properties JSONB,

    -- 会话信息
    session_id      VARCHAR(64),                    -- 会话 ID
    event_duration  NUMERIC(10, 3),                 -- 事件时长（trackTimer 系列，秒）

    -- 页面信息
    page_url        VARCHAR(1000),
    page_title      VARCHAR(255),
    referrer        VARCHAR(1000),

    -- 设备与环境（与 js_errors 保持一致）
    user_agent      VARCHAR(500),
    browser         VARCHAR(50),
    browser_version VARCHAR(30),
    os              VARCHAR(50),
    os_version      VARCHAR(30),
    device_type     VARCHAR(20),                    -- desktop / mobile / tablet
    screen_width    INTEGER,
    screen_height   INTEGER,
    viewport_width  INTEGER,
    viewport_height INTEGER,
    language        VARCHAR(10),
    timezone        VARCHAR(50),

    -- 地理信息
    ip              INET,
    country         VARCHAR(50),
    city            VARCHAR(50),

    -- SDK 信息
    sdk_version     VARCHAR(20),
    release         VARCHAR(50),                    -- 代码版本
    environment     VARCHAR(20),                    -- production / staging / development

    -- 时间
    client_time     TIMESTAMPTZ,                    -- 客户端事件时间（允许离线补报）
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),  -- 服务器接收时间

    PRIMARY KEY (id, created_at)
) PARTITION BY RANGE (created_at);

-- 按月分区（示例）
CREATE TABLE track_events_2024_01 PARTITION OF track_events
    FOR VALUES FROM ('2024-01-01') TO ('2024-02-01');
```

#### 20.4.2 用户画像表

```sql
-- 用户属性表（Profile，每个 distinct_id 一条记录）
CREATE TABLE track_user_profiles (
    id              BIGSERIAL PRIMARY KEY,
    project_id      INTEGER NOT NULL REFERENCES projects(id),
    distinct_id     VARCHAR(128) NOT NULL,
    anonymous_id    VARCHAR(128),
    user_id         VARCHAR(128),                   -- 关联业务用户 ID

    -- 预置用户属性
    name            VARCHAR(100),
    email           VARCHAR(200),
    phone           VARCHAR(20),

    -- 自定义用户属性（JSON，灵活扩展）
    properties      JSONB NOT NULL DEFAULT '{}',

    -- 统计字段（自动更新）
    first_visit_at  TIMESTAMPTZ,                    -- 首次访问时间
    last_visit_at   TIMESTAMPTZ,                    -- 最近访问时间
    total_events    INTEGER DEFAULT 0,              -- 累计事件数
    total_sessions  INTEGER DEFAULT 0,              -- 累计会话数

    created_at      TIMESTAMPTZ DEFAULT NOW(),
    updated_at      TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(project_id, distinct_id)
);

-- 用户 ID 关联表（匿名 ID 与登录 ID 的映射）
CREATE TABLE track_id_mapping (
    id              BIGSERIAL PRIMARY KEY,
    project_id      INTEGER NOT NULL REFERENCES projects(id),
    anonymous_id    VARCHAR(128) NOT NULL,
    login_id        VARCHAR(128) NOT NULL,
    merged_at       TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(project_id, anonymous_id)
);
```

#### 20.4.3 事件定义管理表

```sql
-- 事件定义表（埋点管理平台使用，类似神策「事件方案」）
CREATE TABLE track_event_definitions (
    id              SERIAL PRIMARY KEY,
    project_id      INTEGER NOT NULL REFERENCES projects(id),
    event_name      VARCHAR(128) NOT NULL,          -- 事件标识符（英文，如 purchase）
    display_name    VARCHAR(200),                   -- 展示名（中文，如 购买商品）
    description     TEXT,
    event_type      VARCHAR(20) DEFAULT 'custom',   -- custom / auto / virtual
    category        VARCHAR(50),                    -- 事件分类（如 交易 / 浏览 / 交互）
    status          VARCHAR(20) DEFAULT 'active',   -- active / deprecated / hidden
    created_by      INTEGER REFERENCES users(id),
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    updated_at      TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(project_id, event_name)
);

-- 事件属性定义表
CREATE TABLE track_event_properties (
    id              SERIAL PRIMARY KEY,
    project_id      INTEGER NOT NULL REFERENCES projects(id),
    event_name      VARCHAR(128),                   -- NULL 表示通用属性
    property_name   VARCHAR(128) NOT NULL,
    display_name    VARCHAR(200),
    property_type   VARCHAR(20) NOT NULL,           -- string / number / boolean / list / date
    description     TEXT,
    is_required     BOOLEAN DEFAULT FALSE,
    example_value   VARCHAR(255),
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(project_id, event_name, property_name)
);

-- 用户属性定义表
CREATE TABLE track_user_property_definitions (
    id              SERIAL PRIMARY KEY,
    project_id      INTEGER NOT NULL REFERENCES projects(id),
    property_name   VARCHAR(128) NOT NULL,
    display_name    VARCHAR(200),
    property_type   VARCHAR(20) NOT NULL,
    description     TEXT,
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(project_id, property_name)
);
```

#### 20.4.4 分析模型表

```sql
-- 漏斗定义表
CREATE TABLE track_funnels (
    id              SERIAL PRIMARY KEY,
    project_id      INTEGER NOT NULL REFERENCES projects(id),
    name            VARCHAR(200) NOT NULL,
    description     TEXT,
    steps           JSONB NOT NULL,                 -- 步骤列表（有序）
    -- steps 格式: [{"event": "view_product", "filters": {...}}, ...]
    window_minutes  INTEGER DEFAULT 1440,           -- 转化窗口期（分钟，默认24小时）
    created_by      INTEGER REFERENCES users(id),
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    updated_at      TIMESTAMPTZ DEFAULT NOW()
);

-- 留存分析定义表
CREATE TABLE track_retention_configs (
    id              SERIAL PRIMARY KEY,
    project_id      INTEGER NOT NULL REFERENCES projects(id),
    name            VARCHAR(200) NOT NULL,
    initial_event   VARCHAR(128) NOT NULL,          -- 初始事件
    return_event    VARCHAR(128) NOT NULL,           -- 回访事件
    initial_filters JSONB,                          -- 初始事件筛选条件
    return_filters  JSONB,                          -- 回访事件筛选条件
    retention_days  INTEGER DEFAULT 7,              -- 留存周期（天）
    created_by      INTEGER REFERENCES users(id),
    created_at      TIMESTAMPTZ DEFAULT NOW()
);
```

#### 20.4.5 埋点数据库索引

```sql
-- track_events 核心查询索引
CREATE INDEX idx_track_events_project_time ON track_events(project_id, created_at DESC);
CREATE INDEX idx_track_events_distinct_id ON track_events(distinct_id, created_at DESC);
CREATE INDEX idx_track_events_user_id ON track_events(user_id, created_at DESC) WHERE user_id IS NOT NULL;
CREATE INDEX idx_track_events_event_name ON track_events(project_id, event, created_at DESC);
CREATE INDEX idx_track_events_session ON track_events(session_id);
CREATE INDEX idx_track_events_properties ON track_events USING GIN(properties);   -- JSONB 查询加速

-- user_profiles 索引
CREATE INDEX idx_track_profiles_project ON track_user_profiles(project_id);
CREATE INDEX idx_track_profiles_user_id ON track_user_profiles(user_id) WHERE user_id IS NOT NULL;
CREATE INDEX idx_track_profiles_properties ON track_user_profiles USING GIN(properties);

-- id_mapping 索引
CREATE INDEX idx_id_mapping_login ON track_id_mapping(project_id, login_id);
```

---

### 20.5 SDK 数据上报协议（埋点扩展）

现有 `/api/v1/collect` 接口扩展支持埋点数据类型：

```typescript
// 上报类型扩展
type CollectType =
  | 'error'           // 原有：JS 错误
  | 'network'         // 原有：接口错误
  | 'performance'     // 原有：性能指标
  | 'breadcrumb'      // 原有：面包屑
  | 'track'           // 新增：自定义埋点事件
  | 'track_batch'     // 新增：批量埋点事件
  | 'profile'         // 新增：用户属性更新
  | 'track_signup';   // 新增：用户 ID 关联（匿名→登录）

// 单条埋点事件上报
POST /api/v1/collect
{
  "type": "track",
  "data": {
    "distinct_id": "anon_abc123",
    "anonymous_id": "anon_abc123",
    "is_login_id": false,
    "event": "purchase",
    "properties": {
      "product_id": "sku_001",
      "price": 99.9,
      "$page_url": "https://example.com/checkout",
      "$browser": "Chrome",
      // 超级属性自动附加
      "app_version": "2.1.0",
      "channel": "organic"
    },
    "client_time": 1700000000000
  }
}

// 批量上报（Beacon API 页面关闭时使用）
POST /api/v1/collect
{
  "type": "track_batch",
  "data": [
    { "event": "$page_view", "distinct_id": "...", "properties": {...} },
    { "event": "button_click", "distinct_id": "...", "properties": {...} }
  ]
}

// 用户属性更新
POST /api/v1/collect
{
  "type": "profile",
  "data": {
    "distinct_id": "user_123",
    "is_login_id": true,
    "operation": "set",       // set / set_once / append / unset
    "properties": {
      "$name": "张三",
      "membership": "premium"
    }
  }
}

// 用户 ID 关联（登录时调用）
POST /api/v1/collect
{
  "type": "track_signup",
  "data": {
    "distinct_id": "user_123",        // 新的登录 ID
    "original_id": "anon_abc123",     // 匿名 ID
    "is_login_id": true
  }
}
```

---

### 20.6 埋点管理平台 API

```
# 事件定义管理
GET    /api/tracking/events                  # 事件定义列表
  Query: project_id, category, status, keyword
POST   /api/tracking/events                  # 创建事件定义
  Body: { event_name, display_name, description, category, properties: [...] }
GET    /api/tracking/events/:event_name      # 事件详情（含属性列表、近7天数据量）
PUT    /api/tracking/events/:event_name      # 更新事件定义
DELETE /api/tracking/events/:event_name      # 删除/弃用事件定义

# 事件属性管理
GET    /api/tracking/properties              # 属性定义列表
  Query: project_id, event_name（不传则返回通用属性）
POST   /api/tracking/properties              # 创建属性定义

# 用户属性管理
GET    /api/tracking/user-properties         # 用户属性定义列表
POST   /api/tracking/user-properties         # 创建用户属性定义

# 实时事件流（Debug 模式）
GET    /api/tracking/live-events             # SSE 实时事件流
  Query: project_id, distinct_id（可选，过滤特定用户）
  Events:
    - track: { event, distinct_id, properties, created_at }
    - profile: { distinct_id, properties }

# 事件分析查询
POST   /api/tracking/analysis/events         # 事件分析
  Body: {
    project_id,
    event,
    filters: [{ property, operator, value }],
    group_by: ['browser', 'city'],           # 分组维度
    metric: 'count' | 'uv' | 'sum' | 'avg', # 指标
    metric_property: 'price',               # 聚合字段（sum/avg 时必填）
    time_range: { start, end },
    interval: '1h' | '1d' | '1w'
  }
  Response: {
    total: number,
    data: [{ time, value, breakdown: {...} }]
  }

# 漏斗分析
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
    breakdown: { ... }         # group_by 分组结果
  }

# 留存分析
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

# 用户画像查询
GET    /api/tracking/users                   # 用户列表（含属性筛选）
  Query: project_id, page, filters（JSON 序列化的筛选条件）
GET    /api/tracking/users/:distinct_id      # 用户详情（属性 + 最近事件流水）
GET    /api/tracking/users/:distinct_id/events  # 用户事件时间线
  Query: page, event_name, start_time, end_time
```

---

### 20.7 埋点管理前端页面规划

在 `monitor-web/src/views/` 下新增 `tracking/` 目录：

```
tracking/
├── events/               # 事件管理
│   ├── index.vue         # 事件列表（事件名、分类、数据量、状态）
│   └── detail.vue        # 事件详情（属性列表、近期趋势迷你图）
├── analysis/
│   ├── event-analysis.vue    # 事件分析（折线/柱状图，支持分组、筛选）
│   ├── funnel.vue            # 漏斗分析（可视化漏斗图 + 明细数据）
│   └── retention.vue         # 留存分析（热力表格 / 曲线图）
├── users/
│   ├── index.vue             # 用户列表（条件筛选、分页）
│   └── profile.vue           # 用户详情（属性面板 + 事件流水线）
└── debug/
    └── live-events.vue       # 实时事件流（Debug 模式，SSE 展示）
```

#### 核心页面交互设计

**事件分析页（参考 Mixpanel Events）**
- 左侧：选择事件、添加筛选条件、选择指标（UV/PV/Sum/Avg）
- 右侧：时间序列折线图 + 数据表格
- 支持「按维度分组」（如按浏览器、城市、设备类型）
- 时间范围选择器（最近7天/30天/自定义）

**漏斗分析页（参考神策漏斗）**
- 拖拽排序的步骤配置区
- 可视化漏斗图（各步骤转化率 + 流失率）
- 下方明细：各步骤转化人数、平均转化时长
- 支持分组对比（如 A/B 实验对比不同渠道的转化）

**留存分析页（参考 Amplitude Retention）**
- 上方：初始事件 + 回访事件 + 时间范围配置
- 中间：热力矩阵（cohort 表格，颜色深浅表示留存率高低）
- 下方：平均留存曲线图

**用户画像页（参考神策用户分群）**
- 左侧筛选面板：按用户属性筛选（城市=北京，membership=premium 等）
- 右侧用户列表：显示 distinct_id、最后访问时间、事件数等
- 点击进入用户详情：属性面板 + 时间线（事件流水）

---

### 20.8 埋点数据处理流程（后端）

```
SDK 上报 /api/v1/collect (type=track)
              ↓
    接收、鉴权（app_id + app_key）
              ↓
    IP 解析地理信息（异步，不阻塞上报）
              ↓
    写入 Redis Stream（track_events_stream）
              ↓ 异步消费
    Consumer Worker（并发多个）
        ↓               ↓
   写入 PostgreSQL   更新用户 Profile
   track_events      track_user_profiles
        ↓
   触发实时事件流推送（SSE → Debug 页面）
        ↓
   触发预聚合任务（每分钟 → track_event_stats_hourly）
```

#### 20.8.1 预聚合统计表（加速查询）

```sql
-- 事件按小时预聚合（加速事件分析查询）
CREATE TABLE track_event_stats_hourly (
    id              SERIAL PRIMARY KEY,
    project_id      INTEGER NOT NULL REFERENCES projects(id),
    event           VARCHAR(128) NOT NULL,
    hour            TIMESTAMPTZ NOT NULL,
    total_count     INTEGER DEFAULT 0,        -- 总触发次数（PV/次数）
    unique_users    INTEGER DEFAULT 0,        -- 触发的唯一用户数（UV）
    properties_summary JSONB,                 -- 主要属性的分布统计
    UNIQUE(project_id, event, hour)
);
```

---

### 20.9 SDK 模块文件结构补充

在原有 SDK 目录中，`plugins/behavior.ts` 需扩展为完整的埋点模块：

```
sdk/src/plugins/
├── behavior.ts        # 原有：用户行为追踪（点击、路由跳转）
│                      # 扩展为：完整埋点 API（track/identify/setUserProperties...）
├── auto-track.ts      # 新增：全埋点/无埋点自动采集（$page_view/$element_click/$page_leave）
├── exposure.ts        # 新增：元素曝光半自动采集（IntersectionObserver）
└── profile.ts         # 新增：用户画像（Profile 操作 set/set_once/append/unset）

sdk/src/core/
├── identity.ts        # 新增：用户身份管理（匿名 ID/登录 ID/ID 关联）
├── super-props.ts     # 新增：超级属性管理（register/unregister）
└── timer.ts           # 新增：事件时长计时器（trackTimerStart/End）
```

---

### 20.10 埋点规范与最佳实践建议

**事件命名规范（对标神策事件方案）**
```
动词_名词          → purchase_product / view_page / click_button
$前缀              → 系统预置事件（$page_view / $element_click）
小写+下划线        → 统一格式，避免大小写混用
```

**属性命名规范**
```
$前缀              → SDK 自动采集的预置属性（$browser / $os / $page_url）
业务前缀_属性名    → product_id / order_amount / user_level
避免使用            → id（太通用）/ data（无意义）
```

**埋点方案分层（参考 Mixpanel 最佳实践）**
```
Level 1 - 全埋点（自动）：$page_view / $element_click
  → 覆盖所有页面流量，无需开发干预，运营事后分析

Level 2 - 关键路径代码埋点（手动）：
  → 核心业务转化：注册 / 登录 / 购买 / 提交
  → 这些事件需要携带业务属性，全埋点无法满足

Level 3 - 用户属性：
  → identify + setUserProperties
  → 用于用户分群分析（membership / city / age_group）
```

---

## 十七、开发阶段规划

### Phase 1：基础框架搭建
1. 初始化 SDK 项目（Rollup + TS）
2. 初始化 Vue3 + Vite 前端项目
3. 初始化 Rust + Axum 后端项目
4. 配置数据库连接、迁移

### Phase 2：核心监控功能
1. SDK 实现错误监听、接口监听、性能采集
2. 后端实现数据上报接口、数据存储
3. 前端实现项目创建、错误列表展示
4. **埋点基础**：SDK 实现 `track` / `identify` / `setUserProperties` / `registerSuperProperties` 核心 API
5. **埋点基础**：后端扩展 `/api/v1/collect` 支持 `track` / `profile` / `track_signup` 类型
6. **埋点基础**：创建 `track_events` / `track_user_profiles` / `track_id_mapping` 数据库表

### Phase 3：数据可视化
1. 仪表盘图表（ECharts）
2. 错误趋势、浏览器分布、设备分布
3. 实时数据推送（SSE）
4. **埋点全埋点**：SDK 实现 `auto-track.ts`（$page_view / $element_click / $page_leave 自动采集）
5. **埋点管理平台**：前端事件定义管理页、事件属性管理页
6. **埋点分析**：事件分析页（折线图 + 分组维度）

### Phase 4：告警与 AI
1. 告警规则配置
2. Webhook/邮件通知
3. AI 分析集成（调用 LLM API）
4. Source Map 上传与解析
5. **埋点分析**：漏斗分析（定义漏斗 + 可视化漏斗图）
6. **埋点分析**：留存分析（cohort 热力矩阵 + 曲线图）
7. **埋点调试**：实时事件流 Debug 页面（SSE 推送）

### Phase 5：前端埋点信息采集
1. **SDK 自动采集增强**：完善页面浏览、点击、离开事件的上下文信息
2. **埋点曝光追踪**：SDK 实现 `exposure.ts`（IntersectionObserver 半自动曝光）
3. **用户身份与属性采集**：支持 identify / profile / track_signup 数据闭环
4. **埋点用户画像**：用户列表（属性筛选）+ 用户详情（事件时间线）

---

## 十八、已确认需求

1. **AI 模型**：接入国内大模型（DeepSeek / 通义千问 / 文心），通过标准 OpenAI 兼容接口封装，便于后续切换
2. **告警渠道**：第一阶段仅实现页面内 SSE 实时推送提醒，架构预留 Webhook 扩展接口
3. **部署规模**：中小规模（几十个网站，日活几千~几万），PostgreSQL + Redis 即可满足，暂不分库分表
4. **权限模型**：采用 super_admin / admin / owner / member / readonly 五级权限
5. **SDK 上报策略**：混合策略（P0 错误实时上报，其他批量上报）
6. **数据保留**：按项目配置，默认 30 天（埋点数据默认 90 天，可配置）
7. **前端 UI**：Element Plus
8. **SSE 推送范围**：用户登录后推送其所有有权限项目的告警
9. **埋点 API 风格**：参考 Mixpanel/Amplitude 现代设计（track / identify / setUserProperties），兼容 Sensors Data 用户关联体系
10. **全埋点策略**：自动采集 $page_view / $element_click / $page_leave，可通过初始化配置单独开关
11. **埋点与监控共用 SDK**：埋点功能作为 SDK 内置模块集成到现有 JS 监控 SDK 中，共用上报通道和队列
12. **用户身份**：采用简易 IDM（匿名 ID → 登录 ID 关联），不实现复杂的全域 ID 合并

---

## 十九、开发阶段与里程碑

| 阶段 | 内容 | 预计文件数 |
|------|------|-----------|
| Phase 1 | 基础框架：SDK 项目 + Vue3 项目 + Rust 项目初始化 | ~30 |
| Phase 2 | 核心监控：SDK 采集上报 + 后端存储 + 前端项目/错误列表 + **埋点核心 API + track_events 表** | ~60 |
| Phase 3 | 数据可视化：ECharts 仪表盘 + SSE 实时推送 + **全埋点采集 + 埋点管理平台 + 事件分析页** | ~45 |
| Phase 4 | AI 与告警：Source Map 上传/解析 + AI 分析 + 告警规则 + **漏斗分析 + 留存分析 + 实时事件流** | ~50 |
| Phase 5 | 前端埋点信息采集：**用户画像 + 曝光追踪 + 采集闭环** | ~25 |
