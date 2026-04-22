# Phase 1：基础框架搭建

## 目标

初始化三个核心项目：SDK、前端、后端，配置好基础依赖、目录结构、数据库连接和迁移。

---

## 1.1 SDK 项目初始化

### 技术选型
- 语言：TypeScript
- 构建：Rollup（输出 UMD / ESM / IIFE 三种格式）
- 目标体积：< 15KB（gzip）
- 兼容性：IE11+（Promise polyfill 可选）

### 目录结构
```
sdk/
├── src/
│   ├── core/
│   │   ├── monitor.ts        # 主入口，初始化配置
│   │   ├── reporter.ts       # 数据上报模块
│   │   ├── store.ts          # 本地缓存/队列
│   │   ├── identity.ts       # 用户身份管理
│   │   ├── super-props.ts    # 超级属性管理
│   │   ├── timer.ts          # 事件时长计时器
│   │   └── utils.ts          # 工具函数
│   ├── plugins/
│   │   ├── error.ts          # JS 错误监听
│   │   ├── console.ts        # 控制台日志劫持
│   │   ├── network.ts        # fetch / XHR 监控
│   │   ├── performance.ts    # 性能指标采集
│   │   ├── behavior.ts       # 用户行为追踪
│   │   ├── auto-track.ts     # 全埋点自动采集
│   │   ├── exposure.ts       # 曝光追踪
│   │   ├── profile.ts        # 用户画像操作
│   │   └── breadcrumb.ts     # 面包屑
│   ├── types/
│   │   └── index.ts          # 类型定义
│   └── index.ts              # SDK 入口
├── build/                    # 打包输出
├── rollup.config.js
├── tsconfig.json
└── package.json
```

### package.json 关键配置
```json
{
  "name": "@js-monitor/sdk",
  "version": "1.0.0",
  "main": "build/sdk.umd.js",
  "module": "build/sdk.esm.js",
  "browser": "build/sdk.iife.js",
  "types": "build/index.d.ts",
  "scripts": {
    "build": "rollup -c",
    "dev": "rollup -c -w",
    "test": "jest"
  }
}
```

### Rollup 配置要点
- 输入：`src/index.ts`
- 输出格式：`umd`、`es`、`iife`
- 插件：`@rollup/plugin-typescript`、`@rollup/plugin-terser`（生产环境）
- 外部依赖：零依赖（不声明 external）

---

## 1.2 前端项目初始化

### 技术选型
| 模块 | 技术 |
|------|------|
| 框架 | Vue 3 + Composition API |
| 构建 | Vite 6 |
| 状态管理 | Pinia |
| 路由 | Vue Router 4 |
| UI 框架 | Element Plus |
| 图表 | ECharts 5 + vue-echarts |
| HTTP 客户端 | Axios |
| 实时通信 | SSE |

### 目录结构
```
monitor-web/
├── src/
│   ├── api/                  # API 请求封装
│   ├── views/
│   │   ├── dashboard/        # 首页图表概览
│   │   ├── project/          # 项目管理
│   │   ├── errors/           # 错误列表与详情
│   │   ├── network/          # 接口报错统计
│   │   ├── sourcemap/        # Source Map 上传管理
│   │   ├── ai-analysis/      # AI 报错分析结果
│   │   ├── tracking/         # 埋点功能模块
│   │   ├── user/             # 用户管理
│   │   ├── group/            # 分组/团队管理
│   │   └── settings/         # 告警配置、通知设置
│   ├── components/
│   │   ├── charts/           # ECharts 图表组件
│   │   ├── error-list/       # 报错列表组件
│   │   ├── breadcrumb/       # 面包屑导航
│   │   └── layout/           # 布局组件
│   ├── stores/               # Pinia 状态管理
│   ├── router/               # Vue Router
│   ├── utils/
│   └── App.vue
├── package.json
├── vite.config.ts
└── tsconfig.json
```

### Vite 配置要点
- 端口：3000
- 代理：`/api` → `http://localhost:8080`
- 自动导入 Element Plus 组件
- 路径别名：`@/` → `src/`

---

## 1.3 后端项目初始化

### 技术选型
| 模块 | 技术 |
|------|------|
| 框架 | Axum（Rust） |
| ORM | SeaORM |
| 数据库 | PostgreSQL 15 |
| 缓存 | Redis |
| 消息队列 | Redis Stream |
| 异步运行时 | Tokio |

### 目录结构
```
monitor-server/
├── src/
│   ├── main.rs               # 服务入口
│   ├── config.rs             # 配置管理
│   ├── router.rs             # 路由注册
│   ├── middleware/
│   │   ├── auth.rs           # JWT 鉴权
│   │   ├── cors.rs           # 跨域
│   │   └── rate_limit.rs     # 限流
│   ├── handlers/
│   │   ├── sdk.rs            # SDK 数据上报
│   │   ├── project.rs        # 项目 CRUD
│   │   ├── error.rs          # 错误数据查询
│   │   ├── network.rs        # 接口报错查询
│   │   ├── sourcemap.rs      # Source Map 上传/解析
│   │   ├── ai_analysis.rs    # AI 分析
│   │   ├── auth.rs           # 登录/注册
│   │   ├── user.rs           # 用户管理
│   │   ├── group.rs          # 分组管理
│   │   ├── alert.rs          # 告警规则
│   │   ├── dashboard.rs      # 仪表盘统计
│   │   ├── tracking.rs       # 埋点事件定义
│   │   ├── track_analysis.rs # 事件分析查询
│   │   └── track_users.rs    # 用户画像查询
│   ├── services/
│   │   ├── ai_service.rs
│   │   ├── alert_service.rs
│   │   ├── sourcemap_service.rs
│   │   ├── stats_service.rs
│   │   ├── track_service.rs
│   │   └── identity_service.rs
│   ├── models/               # SeaORM Entity
│   ├── db.rs                 # 数据库连接
│   └── lib.rs
├── migrations/               # SeaORM 数据库迁移
├── Cargo.toml
└── Dockerfile
```

### Cargo.toml 关键依赖
```toml
[dependencies]
axum = "0.7"
tokio = { version = "1", features = ["full"] }
sea-orm = { version = "1", features = ["sqlx-postgres", "runtime-tokio-native-tls"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace"] }
jsonwebtoken = "9"
bcrypt = "0.15"
redis = { version = "0.24", features = ["tokio-comp"] }
reqwest = { version = "0.11", features = ["json"] }
sourcemap = "6"
config = "0.14"
chrono = "0.4"
uuid = { version = "1", features = ["v4"] }
```

---

## 1.4 数据库连接与迁移

### 配置管理（config.rs）
```rust
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub jwt_secret: String,
    pub ai_api_key: String,
    pub server_port: u16,
    pub sse_port: u16,
}

impl Config {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        config::Config::builder()
            .add_source(config::Environment::default())
            .build()?
            .try_deserialize()
    }
}
```

### 数据库连接（db.rs）
```rust
use sea_orm::{Database, DatabaseConnection};

pub async fn connect(database_url: &str) -> Result<DatabaseConnection, sea_orm::DbErr> {
    Database::connect(database_url).await
}
```

### 初始迁移
使用 `sea-orm-cli` 生成初始迁移：
```bash
# 安装 CLI
cargo install sea-orm-cli

# 生成迁移
sea-orm-cli migrate init

# 创建第一个迁移（用户表）
sea-orm-cli migrate generate create_users_table
```

---

## 1.5 开发环境配置

### docker-compose.yml（开发环境）
```yaml
version: '3.8'

services:
  postgres:
    image: postgres:15-alpine
    environment:
      POSTGRES_USER: monitor
      POSTGRES_PASSWORD: monitor123
      POSTGRES_DB: js_monitor_dev
    ports:
      - "5432:5432"
    volumes:
      - pgdata:/var/lib/postgresql/data

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"

volumes:
  pgdata:
```

### 环境变量示例（.env）
```
DATABASE_URL=postgres://monitor:monitor123@localhost/js_monitor_dev
REDIS_URL=redis://localhost:6379
JWT_SECRET=your-jwt-secret-key-here
AI_API_KEY=your-ai-api-key
SERVER_PORT=8080
SSE_PORT=8081
RUST_LOG=debug
```

---

## 1.6 本阶段验收标准

- [ ] SDK 项目能成功构建出 UMD/ESM/IIFE 三种格式
- [ ] 前端项目能正常启动，显示基础布局
- [ ] 后端项目能正常启动，/health 接口返回 ok
- [ ] 数据库迁移能正常执行，基础表创建成功
- [ ] docker-compose 能一键启动 PostgreSQL + Redis
