# JS 监控平台

一个完整的 JS 监控平台，包含：

- **脚本端 SDK**（原生 TypeScript，零依赖，浏览器 / Chrome 插件注入）
- **监控端前端**（Vue 3 + Vite + Element Plus）
- **监控端后端**（Rust + Axum + SeaORM + PostgreSQL + Redis）

完整技术方案参见 [`docs/`](./docs/README.md)。

---

## 仓库结构

```
.
├── sdk/                  # 脚本端 SDK（Rollup 输出 UMD / ESM / IIFE）
├── monitor-web/          # 监控端前端（Vue3 + Vite）
├── monitor-server/       # 监控端后端（Rust workspace：server + migration）
├── docker-compose.yml    # 开发环境：PostgreSQL + Redis
├── docs/                 # 技术设计文档（含各阶段 phase1~5.md）
└── PLAN.md               # 原始完整规划
```

---

## Phase 1：基础框架搭建（本次交付）

**目标**：三个子项目初始化完成，可独立构建 / 启动，数据库迁移骨架就绪。

### 快速开始

```bash
# 1. 启动 PostgreSQL + Redis
docker compose up -d

# 2. 后端
cd monitor-server
cp .env.example .env
cargo run -p migration -- up         # 首次执行：建 users 表
cargo run                             # http://localhost:8080/health

# 3. 前端
cd ../monitor-web
pnpm install
pnpm dev                              # http://localhost:3000

# 4. SDK
cd ../sdk
pnpm install
pnpm build                            # 输出 build/sdk.{umd,esm,iife}.js
```

### 验收项

- [x] SDK 能成功构建出 `build/sdk.umd.js` / `sdk.esm.js` / `sdk.iife.js`
- [x] 前端能正常启动，展示含侧边栏的基础布局
- [x] 后端能正常启动，`GET /health` 返回 `{"code":0,"message":"ok","data":{"status":"ok",...}}`
- [x] 数据库迁移能执行（至少 1 个 migration：`create_users_table`）
- [x] `docker compose up -d` 一键启动 PostgreSQL + Redis（含健康检查）

---

## 后续阶段

| 阶段 | 内容 | 状态 |
|------|------|------|
| Phase 1 | 基础框架搭建 | ✅ |
| Phase 2 | 核心监控 + 埋点核心 API | ⬜ |
| Phase 3 | 数据可视化 + 全埋点 + 事件分析 | ⬜ |
| Phase 4 | AI + 告警 + 漏斗 / 留存分析 | ⬜ |
| Phase 5 | 权限 / 用户 / Chrome 插件 / 部署 | ⬜ |

详见 `docs/phaseX.md`。
