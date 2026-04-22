# 附录：数据库设计

## 核心表结构

### 用户表

```sql
CREATE TABLE users (
    id              SERIAL PRIMARY KEY,
    username        VARCHAR(50) NOT NULL UNIQUE,
    email           VARCHAR(100) NOT NULL UNIQUE,
    password_hash   VARCHAR(255) NOT NULL,
    role            VARCHAR(20) NOT NULL DEFAULT 'member',
    group_id        INTEGER REFERENCES groups(id),
    avatar          VARCHAR(255),
    last_login_at   TIMESTAMPTZ,
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    updated_at      TIMESTAMPTZ DEFAULT NOW()
);
```

### 分组/团队表

```sql
CREATE TABLE groups (
    id              SERIAL PRIMARY KEY,
    name            VARCHAR(100) NOT NULL,
    description     TEXT,
    owner_id        INTEGER NOT NULL REFERENCES users(id),
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    updated_at      TIMESTAMPTZ DEFAULT NOW()
);
```

### 项目表

```sql
CREATE TABLE projects (
    id              SERIAL PRIMARY KEY,
    name            VARCHAR(100) NOT NULL,
    app_id          VARCHAR(32) NOT NULL UNIQUE,
    app_key         VARCHAR(64) NOT NULL UNIQUE,
    group_id        INTEGER NOT NULL REFERENCES groups(id),
    owner_id        INTEGER NOT NULL REFERENCES users(id),
    description     TEXT,
    alert_threshold INTEGER DEFAULT 10,
    alert_webhook   VARCHAR(500),
    data_retention_days INTEGER DEFAULT 30,
    environment     VARCHAR(20) DEFAULT 'production',
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    updated_at      TIMESTAMPTZ DEFAULT NOW()
);
```

### 项目成员关联表

```sql
CREATE TABLE project_members (
    id              SERIAL PRIMARY KEY,
    project_id      INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    user_id         INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role            VARCHAR(20) NOT NULL DEFAULT 'member',
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(project_id, user_id)
);
```

### JS 错误日志表（按时间范围分区）

```sql
CREATE TABLE js_errors (
    id              BIGSERIAL,
    project_id      INTEGER NOT NULL REFERENCES projects(id),
    app_id          VARCHAR(32) NOT NULL,
    message         TEXT NOT NULL,
    stack           TEXT,
    error_type      VARCHAR(50) NOT NULL,
    source_url      VARCHAR(500),
    line            INTEGER,
    column          INTEGER,
    user_agent      VARCHAR(500),
    browser         VARCHAR(50),
    browser_version VARCHAR(30),
    os              VARCHAR(50),
    os_version      VARCHAR(30),
    device          VARCHAR(50),
    device_type     VARCHAR(20),
    device_memory   VARCHAR(20),
    hardware_concurrency INTEGER,
    connection_type VARCHAR(20),
    ip              INET,
    country         VARCHAR(50),
    city            VARCHAR(50),
    sdk_version     VARCHAR(20),
    release         VARCHAR(50),
    environment     VARCHAR(20),
    url             VARCHAR(500),
    referrer        VARCHAR(500),
    viewport        VARCHAR(30),
    screen_resolution VARCHAR(30),
    language        VARCHAR(10),
    timezone        VARCHAR(50),
    breadcrumb      JSONB,
    extra           JSONB,
    fingerprint     VARCHAR(64),
    is_ai_analyzed  BOOLEAN DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, created_at)
) PARTITION BY RANGE (created_at);

-- 按月分区（示例：2024年1月）
CREATE TABLE js_errors_2024_01 PARTITION OF js_errors
    FOR VALUES FROM ('2024-01-01') TO ('2024-02-01');
```

### 接口报错日志表

```sql
CREATE TABLE network_errors (
    id              BIGSERIAL PRIMARY KEY,
    project_id      INTEGER NOT NULL REFERENCES projects(id),
    app_id          VARCHAR(32) NOT NULL,
    url             VARCHAR(500) NOT NULL,
    method          VARCHAR(10) NOT NULL,
    status          INTEGER,
    request_headers JSONB,
    request_body    TEXT,
    response_headers JSONB,
    response_text   TEXT,
    duration        INTEGER,
    error_type      VARCHAR(50),
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
```

### 性能数据表

```sql
CREATE TABLE performance_data (
    id              BIGSERIAL PRIMARY KEY,
    project_id      INTEGER NOT NULL REFERENCES projects(id),
    app_id          VARCHAR(32) NOT NULL,
    url             VARCHAR(500),
    fp              INTEGER,
    fcp             INTEGER,
    lcp             INTEGER,
    cls             NUMERIC(10, 4),
    ttfb            INTEGER,
    tti             INTEGER,
    load_time       INTEGER,
    dns_time        INTEGER,
    tcp_time        INTEGER,
    ssl_time        INTEGER,
    dom_parse_time  INTEGER,
    resource_count  INTEGER,
    resource_size   INTEGER,
    user_agent      VARCHAR(500),
    browser         VARCHAR(50),
    device_type     VARCHAR(20),
    sdk_version     VARCHAR(20),
    release         VARCHAR(50),
    environment     VARCHAR(20),
    created_at      TIMESTAMPTZ DEFAULT NOW()
);
```

### Source Map 文件表

```sql
CREATE TABLE source_maps (
    id              SERIAL PRIMARY KEY,
    project_id      INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    version         VARCHAR(50) NOT NULL,
    filename        VARCHAR(255) NOT NULL,
    original_filename VARCHAR(255),
    file_path       VARCHAR(500) NOT NULL,
    file_size       INTEGER,
    uploaded_by     INTEGER REFERENCES users(id),
    uploaded_at     TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(project_id, version, filename)
);
```

### AI 分析结果表

```sql
CREATE TABLE ai_analyses (
    id              SERIAL PRIMARY KEY,
    error_id        BIGINT NOT NULL,
    project_id      INTEGER NOT NULL REFERENCES projects(id),
    error_type      VARCHAR(50),
    original_stack  TEXT,
    analyzed_stack  TEXT,
    ai_suggestion   TEXT,
    ai_confidence   NUMERIC(3, 2),
    probable_file   VARCHAR(255),
    probable_line   INTEGER,
    probable_code   TEXT,
    severity_score  INTEGER,
    model_used      VARCHAR(50),
    prompt_tokens   INTEGER,
    completion_tokens INTEGER,
    total_tokens    INTEGER,
    cost_ms         INTEGER,
    is_cached       BOOLEAN DEFAULT FALSE,
    cache_key       VARCHAR(64),
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    updated_at      TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(error_id)
);
```

### 告警规则表

```sql
CREATE TABLE alert_rules (
    id              SERIAL PRIMARY KEY,
    project_id      INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name            VARCHAR(100) NOT NULL,
    rule_type       VARCHAR(30) NOT NULL,
    threshold       INTEGER NOT NULL,
    interval_minutes INTEGER DEFAULT 1,
    webhook_url     VARCHAR(500),
    email           VARCHAR(100),
    enabled         BOOLEAN DEFAULT TRUE,
    created_by      INTEGER NOT NULL REFERENCES users(id),
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    updated_at      TIMESTAMPTZ DEFAULT NOW()
);
```

### 告警记录表

```sql
CREATE TABLE alert_logs (
    id              SERIAL PRIMARY KEY,
    rule_id         INTEGER NOT NULL REFERENCES alert_rules(id),
    project_id      INTEGER NOT NULL REFERENCES projects(id),
    alert_content   TEXT NOT NULL,
    status          VARCHAR(20) DEFAULT 'pending',
    error_count     INTEGER,
    sent_at         TIMESTAMPTZ,
    created_at      TIMESTAMPTZ DEFAULT NOW()
);
```

### 错误聚合统计表

```sql
CREATE TABLE error_stats_hourly (
    id              SERIAL PRIMARY KEY,
    project_id      INTEGER NOT NULL REFERENCES projects(id),
    hour            TIMESTAMPTZ NOT NULL,
    error_type      VARCHAR(50),
    fingerprint     VARCHAR(64),
    message_pattern VARCHAR(255),
    count           INTEGER DEFAULT 0,
    affected_users  INTEGER DEFAULT 0,
    affected_pages  INTEGER DEFAULT 0,
    browser_breakdown JSONB,
    os_breakdown    JSONB,
    UNIQUE(project_id, hour, fingerprint)
);
```

---

## 埋点相关表

### 用户行为事件表

```sql
CREATE TABLE track_events (
    id              BIGSERIAL,
    project_id      INTEGER NOT NULL REFERENCES projects(id),
    app_id          VARCHAR(32) NOT NULL,
    distinct_id     VARCHAR(128) NOT NULL,
    anonymous_id    VARCHAR(128),
    user_id         VARCHAR(128),
    is_login_id     BOOLEAN DEFAULT FALSE,
    event           VARCHAR(128) NOT NULL,
    event_type      VARCHAR(20) DEFAULT 'custom',
    properties      JSONB,
    super_properties JSONB,
    session_id      VARCHAR(64),
    event_duration  NUMERIC(10, 3),
    page_url        VARCHAR(1000),
    page_title      VARCHAR(255),
    referrer        VARCHAR(1000),
    user_agent      VARCHAR(500),
    browser         VARCHAR(50),
    browser_version VARCHAR(30),
    os              VARCHAR(50),
    os_version      VARCHAR(30),
    device_type     VARCHAR(20),
    screen_width    INTEGER,
    screen_height   INTEGER,
    viewport_width  INTEGER,
    viewport_height INTEGER,
    language        VARCHAR(10),
    timezone        VARCHAR(50),
    ip              INET,
    country         VARCHAR(50),
    city            VARCHAR(50),
    sdk_version     VARCHAR(20),
    release         VARCHAR(50),
    environment     VARCHAR(20),
    client_time     TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, created_at)
) PARTITION BY RANGE (created_at);

-- 按月分区
CREATE TABLE track_events_2024_01 PARTITION OF track_events
    FOR VALUES FROM ('2024-01-01') TO ('2024-02-01');
```

### 用户画像表

```sql
CREATE TABLE track_user_profiles (
    id              BIGSERIAL PRIMARY KEY,
    project_id      INTEGER NOT NULL REFERENCES projects(id),
    distinct_id     VARCHAR(128) NOT NULL,
    anonymous_id    VARCHAR(128),
    user_id         VARCHAR(128),
    name            VARCHAR(100),
    email           VARCHAR(200),
    phone           VARCHAR(20),
    properties      JSONB NOT NULL DEFAULT '{}',
    first_visit_at  TIMESTAMPTZ,
    last_visit_at   TIMESTAMPTZ,
    total_events    INTEGER DEFAULT 0,
    total_sessions  INTEGER DEFAULT 0,
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    updated_at      TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(project_id, distinct_id)
);
```

### 用户 ID 关联表

```sql
CREATE TABLE track_id_mapping (
    id              BIGSERIAL PRIMARY KEY,
    project_id      INTEGER NOT NULL REFERENCES projects(id),
    anonymous_id    VARCHAR(128) NOT NULL,
    login_id        VARCHAR(128) NOT NULL,
    merged_at       TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(project_id, anonymous_id)
);
```

### 事件定义表

```sql
CREATE TABLE track_event_definitions (
    id              SERIAL PRIMARY KEY,
    project_id      INTEGER NOT NULL REFERENCES projects(id),
    event_name      VARCHAR(128) NOT NULL,
    display_name    VARCHAR(200),
    description     TEXT,
    event_type      VARCHAR(20) DEFAULT 'custom',
    category        VARCHAR(50),
    status          VARCHAR(20) DEFAULT 'active',
    created_by      INTEGER REFERENCES users(id),
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    updated_at      TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(project_id, event_name)
);
```

### 事件属性定义表

```sql
CREATE TABLE track_event_properties (
    id              SERIAL PRIMARY KEY,
    project_id      INTEGER NOT NULL REFERENCES projects(id),
    event_name      VARCHAR(128),
    property_name   VARCHAR(128) NOT NULL,
    display_name    VARCHAR(200),
    property_type   VARCHAR(20) NOT NULL,
    description     TEXT,
    is_required     BOOLEAN DEFAULT FALSE,
    example_value   VARCHAR(255),
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(project_id, event_name, property_name)
);
```

### 用户属性定义表

```sql
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

### 漏斗定义表

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

### 留存分析配置表

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

### 事件预聚合统计表

```sql
CREATE TABLE track_event_stats_hourly (
    id              SERIAL PRIMARY KEY,
    project_id      INTEGER NOT NULL REFERENCES projects(id),
    event           VARCHAR(128) NOT NULL,
    hour            TIMESTAMPTZ NOT NULL,
    total_count     INTEGER DEFAULT 0,
    unique_users    INTEGER DEFAULT 0,
    properties_summary JSONB,
    UNIQUE(project_id, event, hour)
);
```

---

## 索引设计

### js_errors 核心查询索引

```sql
CREATE INDEX idx_js_errors_project_time ON js_errors(project_id, created_at DESC);
CREATE INDEX idx_js_errors_fingerprint ON js_errors(fingerprint, created_at DESC);
CREATE INDEX idx_js_errors_type ON js_errors(error_type, created_at DESC);
CREATE INDEX idx_js_errors_url ON js_errors(source_url) WHERE source_url IS NOT NULL;
CREATE INDEX idx_js_errors_browser ON js_errors(browser, created_at DESC);
CREATE INDEX idx_js_errors_release ON js_errors(release, created_at DESC);
CREATE INDEX idx_js_errors_is_ai_analyzed ON js_errors(is_ai_analyzed) WHERE is_ai_analyzed = FALSE;
```

### network_errors 索引

```sql
CREATE INDEX idx_network_errors_project_time ON network_errors(project_id, created_at DESC);
CREATE INDEX idx_network_errors_url ON network_errors(url);
CREATE INDEX idx_network_errors_status ON network_errors(status);
```

### performance_data 索引

```sql
CREATE INDEX idx_perf_project_time ON performance_data(project_id, created_at DESC);
CREATE INDEX idx_perf_url ON performance_data(url);
```

### AI 分析索引

```sql
CREATE INDEX idx_ai_analysis_project ON ai_analyses(project_id, created_at DESC);
CREATE INDEX idx_ai_analysis_cache ON ai_analyses(cache_key) WHERE is_cached = TRUE;
```

### 告警索引

```sql
CREATE INDEX idx_alert_logs_project_time ON alert_logs(project_id, created_at DESC);
CREATE INDEX idx_alert_logs_status ON alert_logs(status) WHERE status = 'pending';
```

### 统计表索引

```sql
CREATE INDEX idx_stats_project_hour ON error_stats_hourly(project_id, hour DESC);
```

### 埋点事件索引

```sql
CREATE INDEX idx_track_events_project_time ON track_events(project_id, created_at DESC);
CREATE INDEX idx_track_events_distinct_id ON track_events(distinct_id, created_at DESC);
CREATE INDEX idx_track_events_user_id ON track_events(user_id, created_at DESC) WHERE user_id IS NOT NULL;
CREATE INDEX idx_track_events_event_name ON track_events(project_id, event, created_at DESC);
CREATE INDEX idx_track_events_session ON track_events(session_id);
CREATE INDEX idx_track_events_properties ON track_events USING GIN(properties);
```

### 用户画像索引

```sql
CREATE INDEX idx_track_profiles_project ON track_user_profiles(project_id);
CREATE INDEX idx_track_profiles_user_id ON track_user_profiles(user_id) WHERE user_id IS NOT NULL;
CREATE INDEX idx_track_profiles_properties ON track_user_profiles USING GIN(properties);
```

### ID 关联索引

```sql
CREATE INDEX idx_id_mapping_login ON track_id_mapping(project_id, login_id);
```

---

## 数据保留与清理策略

| 表名 | 保留策略 | 清理方式 |
|------|---------|---------|
| js_errors | 按项目配置（默认30天，可配置） | 定时任务删除旧分区 |
| network_errors | 同 js_errors | 定时任务 DELETE |
| performance_data | 90天 | 定时任务 DELETE |
| ai_analyses | 永久保留（数据量小） | 不清理 |
| alert_logs | 180天 | 定时任务 DELETE |
| error_stats_hourly | 365天 | 定时任务 DELETE |
| track_events | 默认90天，可配置 | 定时任务删除旧分区 |
| track_event_stats_hourly | 365天 | 定时任务 DELETE |

**清理机制**：后端启动一个定时任务（每天凌晨3点），删除超期的 `js_errors` 分区和其他表的历史数据。
