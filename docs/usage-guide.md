# JS 监控平台使用教程

> 本教程涵盖 SDK 埋点接入与管理端使用两部分，帮助你从零开始完成前端监控与埋点分析的全链路配置。

---

## 目录

1. [快速开始](#一快速开始)
2. [SDK 使用教程](#二sdk-使用教程)
3. [管理端使用教程](#三管理端使用教程)
4. [常见问题](#四常见问题)

---

## 一、快速开始

### 1.1 环境准备

确保本地已安装：

- Docker & Docker Compose
- Node.js 18+（如需要本地构建前端）
- Rust 工具链（如需要本地构建后端）

### 1.2 一键启动

```bash
cd /path/to/project

# 复制环境变量模板
cp .env.example .env

# 启动全部服务（PostgreSQL + Redis + 后端 + 前端 + Nginx）
docker-compose up -d

# 查看后端日志
docker logs -f js_monitor_server
```

启动后访问：

| 服务 | 地址 |
|------|------|
| 管理端 | http://localhost |
| 后端 API | http://localhost/api |
| SDK 上报 | http://localhost/api/v1/collect |

### 1.3 获取 SDK 构建产物

```bash
cd sdk
pnpm install
pnpm build

# 产物位于 build/ 目录
# build/sdk.umd.js    — UMD 格式
# build/sdk.esm.js    — ESM 格式
# build/sdk.iife.js   — IIFE 格式（浏览器 script 标签直接引入）
```

将产物部署到 CDN 或静态服务器，页面中引用即可。

---

## 二、SDK 使用教程

### 2.1 引入 SDK

#### 方式一：ES Module（推荐，现代构建工具）

```bash
npm install @js-monitor/sdk
```

```typescript
import Monitor from '@js-monitor/sdk';

Monitor.init({
  appId: 'your-app-id',
  appKey: 'your-app-key',
  server: 'https://monitor.example.com',
});
```

#### 方式二：浏览器 Script 标签

```html
<script src="https://your-cdn.com/sdk.iife.js"></script>
<script>
  Monitor.init({
    appId: 'your-app-id',
    appKey: 'your-app-key',
    server: 'https://monitor.example.com',
  });
</script>
```

### 2.2 初始化配置

完整配置示例：

```typescript
Monitor.init({
  // 必填项
  appId: 'app_xxx',
  appKey: 'key_xxx',
  server: 'https://monitor.example.com',

  // 选填项
  release: '1.2.3',                    // 代码版本（用于 Source Map 关联）
  environment: 'production',           // 运行环境
  debug: false,                        // 开启调试日志
  performanceSampleRate: 0.1,          // 性能采样率（默认 10%）

  // 监控插件开关（默认全部开启）
  plugins: {
    error: true,        // JS 错误、Promise 错误、资源加载错误
    console: true,      // 控制台日志劫持（写入面包屑）
    network: true,      // fetch / XHR 接口监控
    performance: true,  // 性能指标采集
    breadcrumb: true,   // 用户操作路径
  },

  // 埋点配置
  tracking: {
    enableTracking: true,
    autoTrack: {
      pageView: true,   // 自动采集页面浏览
      click: true,      // 自动采集元素点击
      pageLeave: true,  // 自动采集页面离开+停留时长
      exposure: false,  // 半自动曝光采集（需元素标记 data-track-imp）
    },
    trackFlushInterval: 3000,   // 埋点批量上报间隔（ms）
    trackMaxBatchSize: 20,      // 埋点批量上报最大条数
  },

  // 脱敏配置
  sanitize: {
    sensitiveFields: ['password', 'token', 'secret'],
    sensitiveQueryKeys: ['token', 'auth', 'key'],
    maxBodySize: 10240,
  },
});
```

### 2.3 代码埋点核心 API

#### ① 追踪自定义事件（`track`）

```typescript
// 简单事件
Monitor.track('button_click');

// 带属性的事件
Monitor.track('purchase', {
  product_id: 'sku_001',
  product_name: '商品A',
  price: 99.9,
  quantity: 2,
  currency: 'CNY',
  payment_method: 'alipay',
});

// 事件会自动附加以下预置属性：
// $page_url, $page_title, $browser, $os, $device_type,
// 以及你通过 registerSuperProperties 注册的全局属性
```

#### ② 用户识别（`identify`）

```typescript
// 用户登录成功后调用，将匿名用户关联到登录用户
Monitor.identify('user_123456');

// 调用后，后续所有事件的 distinct_id 将使用登录用户 ID
// 同时会上报一条 track_signup 关联记录到服务端
```

#### ③ 设置用户属性（`setUserProperties`）

```typescript
// 设置/更新用户属性（覆盖已有值）
Monitor.setUserProperties({
  $name: '张三',
  $email: 'zhangsan@example.com',
  membership: 'premium',
  signup_date: '2024-01-01',
  city: '北京',
});

// 仅首次设置时生效（不会覆盖已有值）
Monitor.setUserPropertiesOnce({
  first_referral: 'wechat_article',
});

// 向列表类型属性追加值
Monitor.appendUserProperties({
  tags: ['新用户', '活跃'],
});

// 删除某个属性
Monitor.unsetUserProperty('temporary_field');
```

> 属性名以 `$` 开头的是系统预置属性，管理端会做特殊展示。

#### ④ 超级属性（全局附加属性）

```typescript
// 注册超级属性 — 后续所有事件都会自动携带这些属性
Monitor.registerSuperProperties({
  app_version: '2.1.0',
  channel: 'organic',
  experiment_group: 'A',
});

// 注销单个超级属性
Monitor.unregisterSuperProperty('experiment_group');

// 清空全部超级属性
Monitor.clearSuperProperties();
```

#### ⑤ 事件时长统计（`trackTimer`）

```typescript
// 开始计时
Monitor.trackTimerStart('video_play');

// ... 用户播放视频 ...

// 结束计时，自动上报事件并附加 $event_duration 字段（秒）
Monitor.trackTimerEnd('video_play', {
  video_id: 'v_001',
  title: '教程1',
});

// 上报的事件属性为：
// {
//   event: 'video_play',
//   properties: { video_id: 'v_001', title: '教程1', $event_duration: 125.234 }
// }
```

#### ⑥ 用户注销（`logout`）

```typescript
// 用户登出时调用，清除登录身份，重置为新的匿名 ID
Monitor.logout();
```

#### ⑦ 手动设置匿名 ID（高级用法）

```typescript
// 跨端统一匿名标识时使用
Monitor.identify_anonymous('custom_anon_id_123');
```

### 2.4 全埋点自动采集

SDK 初始化后，根据 `tracking.autoTrack` 配置自动采集以下预置事件，**无需手动调用 `track`**：

#### `$page_view` — 页面浏览

```json
{
  "event": "$page_view",
  "properties": {
    "$page_url": "https://example.com/product/123",
    "$page_title": "商品详情页",
    "$referrer": "https://example.com/home",
    "$viewport_width": 1920,
    "$viewport_height": 1080,
    "$is_first_visit": false,
    "$is_first_day": false
  }
}
```

> 兼容 SPA：自动监听 `popstate`、`hashchange`，并劫持 `history.pushState/replaceState`。

#### `$element_click` — 元素点击

```json
{
  "event": "$element_click",
  "properties": {
    "$element_id": "btn-submit",
    "$element_class": "btn btn-primary",
    "$element_type": "button",
    "$element_content": "立即购买",
    "$element_path": "body > div > button#btn-submit",
    "$page_x": 120,
    "$page_y": 350
  }
}
```

> 自动识别可交互元素：`a`、`button`、`input`、带 `onclick` 的元素、`cursor: pointer` 的元素。

#### `$page_leave` — 页面离开

```json
{
  "event": "$page_leave",
  "properties": {
    "$page_url": "https://example.com/product/123",
    "$stay_duration": 45.2,
    "$leave_reason": "navigation"
  }
}
```

### 2.5 曝光追踪（半自动采集）

对需要追踪曝光的元素添加 HTML 属性：

```html
<div
  data-track-imp="true"
  data-track-event="product_exposure"
  data-track-attrs='{"product_id": "sku_001", "position": 3, "list": "首页推荐"}'
  data-track-mode="once"
>
  商品卡片内容
</div>
```

| 属性 | 说明 |
|------|------|
| `data-track-imp` | 标记元素需要采集曝光，`true` 时启用 |
| `data-track-event` | 曝光事件名，未提供时默认 `$element_exposure` |
| `data-track-attrs` | JSON 字符串，作为业务属性合入事件 |
| `data-track-mode` | `once` 只触发一次，`always` 每次重新进入视口都触发 |

同时需要在初始化时开启曝光采集：

```typescript
Monitor.init({
  // ...
  tracking: {
    autoTrack: {
      exposure: true,   // ← 必须开启
    },
  },
});
```

### 2.6 错误/性能/网络监控（自动采集）

SDK 默认自动采集以下数据，**无需额外代码**：

| 数据类型 | 采集内容 | 配置项 |
|---------|---------|--------|
| JS 错误 | `window.onerror`、`unhandledrejection`、资源加载错误 | `plugins.error` |
| 接口监控 | `fetch` / `XHR` 请求/响应、耗时、失败状态 | `plugins.network` |
| 性能指标 | FP、FCP、LCP、CLS、TTFB、FID、DNS、TCP、加载时间 | `plugins.performance` |
| 面包屑 | 点击、路由跳转、控制台日志、接口请求 | `plugins.breadcrumb` |

> 敏感数据（如密码、Token）会自动脱敏替换为 `[REDACTED]`。

### 2.7 Vue 项目集成示例

```typescript
// main.ts
import { createApp } from 'vue';
import App from './App.vue';
import Monitor from '@js-monitor/sdk';

Monitor.init({
  appId: import.meta.env.VITE_MONITOR_APP_ID,
  appKey: import.meta.env.VITE_MONITOR_APP_KEY,
  server: import.meta.env.VITE_MONITOR_SERVER,
  release: import.meta.env.VITE_APP_VERSION,
  environment: import.meta.env.MODE,
  tracking: {
    autoTrack: { pageView: true, click: true, pageLeave: true, exposure: true },
  },
});

// 用户登录后
// Monitor.identify(userId);

// 用户登出时
// Monitor.logout();

createApp(App).mount('#app');
```

### 2.8 React 项目集成示例

```typescript
// index.tsx
import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import Monitor from '@js-monitor/sdk';

Monitor.init({
  appId: process.env.REACT_APP_MONITOR_APP_ID!,
  appKey: process.env.REACT_APP_MONITOR_APP_KEY!,
  server: process.env.REACT_APP_MONITOR_SERVER!,
  release: process.env.REACT_APP_VERSION,
});

// 在登录逻辑中调用
// Monitor.identify(userId);

const root = ReactDOM.createRoot(document.getElementById('root')!);
root.render(<App />);
```

### 2.9 最佳实践

1. **尽早初始化**：SDK 应在页面最头部初始化，确保能捕获早期错误。
2. **release 字段**：每次发版时更新，与 Source Map 版本对齐，便于错误定位。
3. **identify 时机**：用户登录成功后立即调用，不要延迟到页面跳转后。
4. **超级属性**：适合放置不频繁变化的上下文，如 `app_version`、`channel`。
5. **事件命名**：使用小写+下划线格式，如 `complete_purchase`，保持语义清晰。
6. **曝光追踪**：对商品卡片、广告位等关键元素使用 `data-track-imp`，不要全量标记。

---

## 三、管理端使用教程

### 3.1 注册与登录

首次访问管理端（http://localhost）时，需要先注册管理员账号：

1. 点击「注册」按钮
2. 填写用户名、邮箱、密码
3. 选择创建新分组（系统会自动创建分组并设你为管理员）
4. 注册成功后自动登录

登录后，系统会建立 SSE 长连接，实时推送你权限范围内的告警和错误。

### 3.2 项目创建与管理

#### 创建项目

1. 点击左侧菜单「项目管理」
2. 点击「新建项目」按钮
3. 填写：
   - **项目名称**：如「官网」
   - **所属分组**：选择你所在的分组
   - **描述**：可选
   - **错误阈值**：默认 10（1分钟内错误数超过此值触发告警）
   - **数据保留天数**：默认 30 天
4. 保存后，系统**自动生成** `app_id` 和 `app_key`

#### 获取 SDK 接入代码

项目创建成功后，在项目列表中点击「接入代码」，复制以下示例：

```html
<script src="https://your-cdn/sdk.iife.js"></script>
<script>
  Monitor.init({
    appId: 'app_xxxxxxxx',
    appKey: 'key_xxxxxxxxxxxxxxxx',
    server: 'https://monitor.example.com',
  });
</script>
```

将代码粘贴到你的网站 `<head>` 标签内即可开始采集。

### 3.3 错误监控

#### 错误列表

进入「错误监控」页面，可查看：

- 错误时间、类型（TypeError / ReferenceError 等）、消息、页面 URL、浏览器
- 筛选：时间范围、错误类型、浏览器、关键字
- 按指纹聚合：相同错误合并展示

#### 错误详情

点击某条错误进入详情抽屉：

- **基本信息**：错误消息、类型、URL、设备信息
- **堆栈信息**：原始堆栈（如有 Source Map 则显示解析后的源码位置）
- **面包屑**：用户操作路径，帮助复现错误现场
- **AI 分析**：点击「AI 分析」按钮，系统自动调用 LLM 分析错误原因并给出修复建议

### 3.4 接口监控

进入「接口监控」页面，查看：

- 接口报错列表：URL、Method、Status、耗时、请求/响应详情
- 统计图表：状态码分布、请求方法分布、平均耗时趋势
- 筛选：按 URL、状态码、时间范围查询

### 3.5 Source Map 管理

为了将压缩后的堆栈还原到原始源码位置，需要上传 Source Map：

1. 进入「Source Map」页面
2. 点击「上传 Source Map」
3. 选择文件（`.map` 格式），填写 **Release 版本号**（需与 SDK 初始化时的 `release` 字段一致）
4. 上传成功后，错误详情中的堆栈会自动解析为原始文件、行号、列号

> 建议将上传步骤集成到 CI/CD 流水线中，每次构建后自动上传。

### 3.6 AI 分析

进入「AI 分析」页面：

- **触发分析**：点击某条错误右侧的「AI 分析」按钮，任务进入队列异步执行
- **分析结果**：包含错误根因、修复方案、严重程度（1-5 分）、可能出错的文件和行号、置信度
- **批量分析**：可对相同指纹的错误批量触发分析

> AI 分析受项目级限流保护：单个项目每分钟最多 20 次请求。相同指纹的结果会缓存 7 天。

### 3.7 告警配置

进入「告警设置」页面：

#### 创建告警规则

1. 切换到「告警规则」Tab
2. 点击「新建规则」
3. 配置：
   - **规则名称**：如「生产环境错误激增」
   - **规则类型**：
     - `error_spike`：1分钟内错误数 > N
     - `failure_rate`：接口失败率 > X%
     - `new_error`：出现新的指纹
     - `p0_error`：出现 P0 级错误（SyntaxError / ReferenceError）
     - `error_trend`：错误数较上小时增长 X%
   - **阈值**：根据规则类型填写
   - **Webhook URL**：可选，支持飞书/钉钉/企微/Slack
   - **通知邮箱**：可选
4. 保存后规则立即生效

#### 告警通知

- **SSE 实时推送**：前端页面会自动弹出告警通知（需保持页面打开）
- **Webhook**：触发时会向配置的 URL 发送 POST 请求
- **告警历史**：切换到「告警日志」Tab 查看历史告警记录

> 告警去重：相同规则在 10 分钟内对同一指纹只告警一次。30 分钟内触发 ≥3 次会自动升级为 critical 级别。

### 3.8 埋点管理

#### 事件管理

进入「用户埋点 → 事件管理」：

- **已采集事件**：自动展示 SDK 上报的所有事件，含总次数、去重用户数、最后上报时间
- **事件定义**：可手动创建/编辑事件定义（事件名、展示名、分类、描述、属性定义）
- **属性列表**：查看所有事件属性及其类型

点击某个事件可进入「事件详情」页，查看近 7 天趋势迷你图和属性分布。

#### 事件分析

进入「用户埋点 → 事件分析」：

1. 选择要分析的事件（支持多选对比）
2. 选择时间范围（最近 7 天 / 30 天）
3. 选择指标：
   - **PV**：事件总次数
   - **UV**：触发事件的去重用户数
4. 选择分组维度（可选）：浏览器 / 操作系统 / 设备类型 / 环境
5. 点击「查询」，右侧展示折线图 + 数据明细表

#### 漏斗分析

进入「用户埋点 → 漏斗分析」：

1. 点击「新建漏斗」
2. 配置步骤：按顺序添加事件（如 `view_product` → `add_cart` → `checkout` → `purchase`）
3. 设置转化窗口期（默认 24 小时）
4. 保存后，选择时间范围执行分析
5. 查看结果：
   - 漏斗图：各步骤转化率和流失率
   - 步骤明细：人数、转化率、平均转化时长
   - 分组对比：按维度拆分漏斗

#### 留存分析

进入「用户埋点 → 留存分析」：

1. 点击「新建留存配置」
2. 选择：
   - **初始事件**：如 `$page_view` 或 `signup`
   - **回访事件**：如 `$page_view` 或 `purchase`
   - **留存周期**：默认 7 天
3. 保存后执行分析
4. 查看结果：
   - **Cohort 热力矩阵**：行表示日期 cohort，列表示第 N 天留存率
   - **平均留存曲线**：整体留存趋势

#### 用户画像

进入「用户埋点 → 用户画像」：

- **用户列表**：展示所有用户（distinct_id），含最后访问时间、累计事件数
- **筛选**：按属性条件筛选（如 `city = 北京` AND `membership = premium`）
- **用户详情**：点击用户进入详情抽屉
  - 属性面板：展示所有用户属性
  - 事件时间线：按时间倒序展示该用户的所有事件，支持按事件名/时间范围筛选

#### 实时事件流 Debug

进入「用户埋点 → 实时事件流」：

- 页面自动通过 SSE 连接接收实时埋点事件
- 支持按事件名、用户 ID 过滤
- 支持暂停/继续接收
- 事件以格式化 JSON 展示，便于调试埋点数据是否正确上报

> 此页面主要用于开发调试，确认 SDK 埋点事件是否正常到达服务端。

---

## 四、常见问题

### Q1：SDK 上报失败怎么办？

- 检查 `appId` 和 `appKey` 是否正确
- 检查 `server` 地址是否可访问
- 打开 `debug: true` 查看控制台日志
- 上报失败的数据会自动存入队列，网络恢复后补发

### Q2：为什么性能数据很少？

性能指标默认采样率 10%，如需提高：

```typescript
Monitor.init({
  performanceSampleRate: 0.5,   // 提高到 50%
});
```

### Q3：SPA 页面切换没有采集到 `$page_view`？

确保开启了 `autoTrack.pageView`，且路由变化通过 `history.pushState` / `replaceState` 实现。如使用自定义路由方式，可手动上报：

```typescript
Monitor.track('$page_view', { $page_url: location.href });
```

### Q4：如何排除某些接口不上报？

SDK 会自动排除自身上报地址。如需排除其他地址，当前版本可通过自定义 `sanitize` 配置控制脱敏，后续版本将支持 URL 白名单/黑名单。

### Q5：Source Map 上传后堆栈仍未解析？

- 确认 Source Map 的 `release` 与 SDK 初始化时的 `release` 完全一致
- 确认上传的是 `.map` 文件且格式正确
- 在错误详情页查看是否有「原始堆栈」折叠面板

---

> 如有更多问题，请参考项目 `docs/` 目录下的技术文档，或查看源码中的类型定义和注释。
