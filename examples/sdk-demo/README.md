# SDK 埋点功能示例项目

本项目用于演示和测试前端监控 SDK 的全部功能，包含完整的交互示例和实时上报数据监控面板。

## 功能覆盖

| 功能模块 | 覆盖内容 |
|---------|---------|
| **初始化配置** | appId / appKey / server / environment / release / debug |
| **错误监控** | JS 错误、Promise 拒绝、资源加载失败、Console 日志捕获 |
| **性能监控** | FP、FCP、LCP、CLS、TTFB、FID 等 Web Vitals 指标 |
| **网络监控** | XHR / fetch 拦截、请求/响应体记录、脱敏处理 |
| **手动埋点** | `track()`、`report()`、自定义事件与属性、P0 实时上报 |
| **自动埋点** | pageView、click、pageLeave、exposure（曝光） |
| **用户身份** | `identify()`、`identify_anonymous()`、`logout()`、匿名/登录 ID 切换 |
| **用户属性** | `setUserProperties()`、`setUserPropertiesOnce()`、`appendUserProperties()`、`unsetUserProperty()` |
| **超级属性** | `registerSuperProperties()`、`unregisterSuperProperty()`、`clearSuperProperties()` |
| **计时器** | `trackTimerStart()` / `trackTimerEnd()`（自动计算 `$event_duration`） |
| **面包屑** | `addBreadcrumb()`、自动 click / navigation / xhr / console 面包屑 |
| **上报策略** | 批量上报、失败重试、页面关闭同步 flush、队列管理 |

## 项目结构

```
sdk-demo/
├── index.html          # 首页 - 覆盖全部功能按钮
├── about.html          # 关于页 - 测试 pageView / pageLeave
├── detail.html         # 详情页 - 测试曝光埋点 / 购买流程计时
├── dashboard.html      # 监控面板 - 实时查看上报数据
├── src/
│   ├── main.js         # SDK 初始化与事件绑定
│   └── styles.css      # 样式
├── mock-server.mjs     # 模拟上报服务器
├── vite.config.js      # Vite 配置（含代理）
└── package.json
```

## 快速开始

### 1. 安装依赖

```bash
cd examples/sdk-demo
npm install
```

### 2. 启动模拟上报服务器

```bash
npm run mock-server
# 或: node mock-server.mjs
```

模拟服务器运行在 `http://localhost:3456`，提供以下接口：
- `POST /api/v1/collect` — 接收 SDK 上报数据
- `GET /api/v1/records` — 查询上报记录
- `DELETE /api/v1/records` — 清空记录

### 3. 启动前端服务

```bash
npm run dev
```

前端服务运行在 `http://localhost:5173`，Vite 代理会自动将 `/api/v1/collect` 转发到模拟服务器。

### 4. 打开浏览器体验

- 首页 `http://localhost:5173/index.html` — 点击各功能按钮测试 SDK
- 监控面板 `http://localhost:5173/dashboard.html` — 实时查看上报数据

## 使用说明

### 首页功能测试

1. **用户身份** — 输入用户 ID 点击「登录」测试 `identify()`，点击「登出」测试 `logout()`
2. **手动埋点** — 点击「上报简单事件」、「上报带属性事件」测试 `track()`
3. **计时器** — 点击「开始计时」和「结束计时」测试 `trackTimerStart/End()`
4. **用户属性** — 点击各属性按钮测试 `setUserProperties` / `setUserPropertiesOnce` / `appendUserProperties`
5. **超级属性** — 输入键值注册超级属性，后续所有 `track` 事件会自动附加
6. **面包屑** — 点击添加不同级别的面包屑
7. **错误触发** — 点击「触发 JS 错误」、「触发 Promise 错误」测试错误捕获
8. **网络请求** — 点击 GET / POST 按钮测试网络监控
9. **曝光埋点** — 滚动页面到底部，当元素进入视口时自动触发曝光事件

### 页面跳转测试

在首页、关于页、详情页之间跳转，测试：
- `pageView` 自动埋点
- `pageLeave` 自动埋点
- `breadcrumb` 中的 navigation 记录

### 监控面板

打开 Dashboard 页面可实时查看所有上报数据，包括：
- 事件类型（track / error / network / performance / breadcrumb / profile）
- 完整的上报 payload（含 __context 上下文信息）
- 统计数字（总上报数、错误数、埋点数、性能数、网络数）
