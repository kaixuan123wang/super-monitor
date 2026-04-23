# JS 监控平台 — 技术设计方案

构建一个完整的 JS 监控平台，包含 **脚本端 SDK**（可注入任意网站）和 **监控端管理系统**（Vue3 + Vite + Rust），实现前端错误监控、接口监控、AI 智能分析、实时告警和前端埋点信息采集等功能。

---

## 项目目录结构

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

## 技术栈选型

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

## 已确认需求

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

## 开发阶段与里程碑

| 阶段 | 内容 | 预计文件数 |
|------|------|-----------|
| Phase 1 | 基础框架：SDK 项目 + Vue3 项目 + Rust 项目初始化 | ~30 |
| Phase 2 | 核心监控：SDK 采集上报 + 后端存储 + 前端项目/错误列表 + **埋点核心 API + track_events 表** | ~60 |
| Phase 3 | 数据可视化：ECharts 仪表盘 + SSE 实时推送 + **全埋点采集 + 埋点管理平台 + 事件分析页** | ~45 |
| Phase 4 | AI 与告警：Source Map 上传/解析 + AI 分析 + 告警规则 + **漏斗分析 + 留存分析 + 实时事件流** | ~50 |
| Phase 5 | 前端埋点信息采集：**用户画像 + 曝光追踪 + 采集闭环** | ~25 |

---

## 各阶段详细文档

- [Phase 1：基础框架搭建](phase1.md)
- [Phase 2：核心监控功能](phase2.md)
- [Phase 3：数据可视化](phase3.md)
- [Phase 4：AI 与告警](phase4.md)
- [Phase 5：前端埋点信息采集](phase5.md)
- [附录：数据库设计](database.md)
- [附录：API 设计](api.md)
- [附录：SDK 设计](sdk.md)
- [附录：埋点模块设计](tracking.md)
- [附录：部署与运维](deployment.md)
