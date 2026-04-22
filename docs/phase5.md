# Phase 5：管理功能

## 目标

1. 用户注册/登录/权限
2. 分组管理
3. Chrome 插件
4. 系统优化与压测
5. **埋点用户画像**：用户列表（属性筛选）+ 用户详情（事件时间线）
6. **埋点曝光追踪**：SDK 实现 `exposure.ts`（IntersectionObserver 半自动曝光）

---

## 5.1 用户认证与权限

### 5.1.1 认证接口

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

### 5.1.2 用户管理 API

```
GET  /api/users                   # 用户列表（super_admin / admin 可查看全部）
  Query: page, page_size, group_id, role, keyword
POST /api/users                   # 创建用户（admin 以上）
PUT  /api/users/:id               # 更新用户信息/角色
DELETE /api/users/:id             # 删除用户（super_admin）
```

### 5.1.3 分组管理 API

```
GET  /api/groups                  # 分组列表
POST /api/groups                  # 创建分组
GET  /api/groups/:id              # 分组详情（含成员列表）
PUT  /api/groups/:id              # 更新分组
DELETE /api/groups/:id            # 删除分组（需无项目）
GET  /api/groups/:id/invite-code  # 获取邀请码
POST /api/groups/:id/join         # 通过邀请码加入分组
```

---

## 5.2 权限模型

### 5.2.1 角色定义

| 角色 | 权限范围 | 说明 |
|------|---------|------|
| **super_admin** | 全系统管理 | 创建/删除分组，管理所有用户，系统配置 |
| **admin** | 分组级管理 | 创建项目，管理分组成员，查看分组所有项目 |
| **owner** | 项目级管理 | 项目 Owner，可修改项目设置、添加成员、配置告警 |
| **member** | 项目级读写 | 查看错误、触发 AI 分析、创建告警规则 |
| **readonly** | 项目级只读 | 仅查看错误和统计数据，不可修改任何配置 |

### 5.2.2 权限矩阵

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

### 5.2.3 权限校验中间件

```rust
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

## 5.3 Chrome 插件

### 5.3.1 功能规划

| 组件 | 功能 |
|------|------|
| content-script | 页面加载时自动注入 SDK，监听页面错误 |
| popup | 迷你监控面板，显示当前页面：错误数、性能指标、最近错误列表 |
| background | 监听浏览器错误事件，管理跨页面状态 |

### 5.3.2 通信流程

```
页面 JS 错误 → content-script 捕获 → background 聚合
                                    ↓
                              popup 打开时请求数据
                                    ↓
                              调用监控端 API 获取项目信息
```

### 5.3.3 插件配置

```json
{
  "manifest_version": 3,
  "name": "JS Monitor",
  "version": "1.0.0",
  "permissions": ["activeTab", "storage"],
  "host_permissions": ["<all_urls>"],
  "content_scripts": [{
    "matches": ["<all_urls>"],
    "js": ["content-script.js"],
    "run_at": "document_start"
  }],
  "action": {
    "default_popup": "popup/index.html"
  },
  "background": {
    "service_worker": "background.js"
  }
}
```

---

## 5.4 埋点用户画像

### 5.4.1 用户列表页

功能：
- 左侧筛选面板：按用户属性筛选（城市=北京，membership=premium 等）
- 右侧用户列表：显示 distinct_id、最后访问时间、事件数等
- 点击进入用户详情

### 5.4.2 用户详情页

功能：
- 属性面板：展示用户所有属性
- 事件时间线：按时间倒序展示用户行为流水

### 5.4.3 用户画像 API

```
GET    /api/tracking/users                   # 用户列表（含属性筛选）
  Query: project_id, page, filters（JSON 序列化的筛选条件）
GET    /api/tracking/users/:distinct_id      # 用户详情（属性 + 最近事件流水）
GET    /api/tracking/users/:distinct_id/events  # 用户事件时间线
  Query: page, event_name, start_time, end_time
```

---

## 5.5 埋点曝光追踪

### 5.5.1 半自动曝光采集

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

### 5.5.2 SDK 实现

- 使用 IntersectionObserver 监听元素可见性
- 当元素从不可见→可见时，自动发送 data-track-event 指定的埋点事件
- 支持 once 模式（只触发一次）和 always 模式（每次出现都触发）

---

## 5.6 系统优化与压测

### 5.6.1 性能优化点

| 优化项 | 措施 |
|--------|------|
| 数据库查询 | 预聚合统计表 + 合理索引 |
| 接口响应 | Redis 缓存热点数据 |
| SDK 上报 | 批量 + 采样 + 本地队列 |
| 前端渲染 | ECharts 虚拟滚动（大数据量） |
| AI 分析 | 异步队列 + 缓存 |

### 5.6.2 压测指标

| 指标 | 目标 |
|------|------|
| SDK 上报接口 QPS | 5000+ |
| 查询接口 P99 延迟 | < 200ms |
| 仪表盘加载时间 | < 2s |
| AI 分析平均耗时 | < 10s |

---

## 5.7 本阶段验收标准

- [ ] 用户能注册、登录、登出
- [ ] 权限中间件能正确拦截越权操作
- [ ] 分组管理能创建、邀请、加入
- [ ] Chrome 插件能显示当前页面错误和性能指标
- [ ] 用户画像能筛选用户并查看事件时间线
- [ ] 曝光追踪能正确触发 IntersectionObserver 事件
- [ ] 系统能承受目标 QPS 压力
