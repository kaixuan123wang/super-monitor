# monitor-server

JS 监控平台后端（Rust + Axum + SeaORM + PostgreSQL + Redis）。

## 开发启动

1. 启动完整开发栈（项目根目录）：
   ```bash
   docker compose -f ../docker-compose.yml up -d
   ```
2. 复制环境变量：
   ```bash
   cp .env.example .env
   ```
3. 如只本地运行后端，可手动执行数据库迁移：
   ```bash
   cargo run -p migration -- up
   ```
4. 启动服务：
   ```bash
   cargo run
   ```
5. 健康检查：
   ```bash
   curl http://localhost:8080/health
   ```

## 目录

```
monitor-server/
├── src/
│   ├── main.rs           # 服务入口
│   ├── lib.rs
│   ├── config.rs         # 配置加载
│   ├── db.rs             # SeaORM 连接
│   ├── router.rs         # 路由 + 中间件
│   ├── error.rs          # 统一错误
│   ├── middleware/       # 鉴权 / 限流（Phase 2+）
│   ├── handlers/         # HTTP 处理器
│   ├── services/         # 业务逻辑
│   └── models/           # SeaORM Entity
└── migration/            # SeaORM 数据库迁移（独立 crate）
```

## 迁移命令

```bash
# 安装 CLI（首次）
cargo install sea-orm-cli

# 生成 Entity（基于数据库）
sea-orm-cli generate entity -o src/models --with-serde both

# 新增迁移
sea-orm-cli migrate generate <name>

# 执行迁移
cargo run -p migration -- up
cargo run -p migration -- down
cargo run -p migration -- refresh
```
