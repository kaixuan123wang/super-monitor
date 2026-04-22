# 附录：部署与运维

## 部署架构

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

---

## Docker Compose 配置

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

---

## Nginx 配置

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

---

## 环境变量

| 变量名 | 说明 | 示例 |
|--------|------|------|
| DATABASE_URL | PostgreSQL 连接字符串 | `postgres://monitor:password@localhost/js_monitor` |
| REDIS_URL | Redis 连接字符串 | `redis://localhost:6379` |
| JWT_SECRET | JWT 签名密钥 | `your-secret-key` |
| AI_API_KEY | AI 服务 API 密钥 | `sk-...` |
| AI_PROVIDER | AI 服务提供商 | `deepseek` |
| AI_MODEL | AI 模型名称 | `deepseek-chat` |
| SERVER_PORT | API 服务端口 | `8080` |
| SSE_PORT | SSE 服务端口 | `8081` |
| RUST_LOG | 日志级别 | `info` |

---

## 日志与监控

| 组件 | 日志位置 | 轮转策略 |
|------|---------|---------|
| Nginx | /var/log/nginx/ | logrotate 每日 |
| Rust 后端 | stdout → docker logs | docker 日志驱动 |
| PostgreSQL | /var/lib/postgresql/data/log/ | 7 天自动清理 |

**自身监控**：
- 后端暴露 `/health` 和 `/metrics` 端点（Prometheus 格式）
- 关键指标：QPS、错误率、P99 延迟、AI 调用成功率、告警延迟

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

---

## 升级与维护

### 数据库迁移

```bash
# 生成新迁移
cd monitor-server
sea-orm-cli migrate generate migration_name

# 执行迁移
cargo run --bin migration
```

### 滚动更新

```bash
# 构建新版本
docker-compose build monitor-server

# 滚动更新（零停机）
docker-compose up -d --no-deps --build monitor-server
```

### 备份策略

| 数据 | 备份方式 | 频率 |
|------|---------|------|
| PostgreSQL | pg_dump + 对象存储 | 每日凌晨 |
| Redis | RDB 快照 | 每小时 |
| Source Map 文件 | 对象存储多副本 | 实时同步 |
