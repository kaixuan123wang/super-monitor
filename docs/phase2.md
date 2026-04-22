# Phase 2：核心监控功能

## 目标

1. SDK 实现错误监听、接口监听、性能采集
2. 后端实现数据上报接口、数据存储
3. 前端实现项目创建、错误列表展示
4. **埋点基础**：SDK 实现 `track` / `identify` / `setUserProperties` / `registerSuperProperties` 核心 API
5. **埋点基础**：后端扩展 `/api/v1/collect` 支持 `track` / `profile` / `track_signup` 类型
6. **埋点基础**：创建 `track_events` / `track_user_profiles` / `track_id_mapping` 数据库表

---

## 2.1 SDK 核心监控实现

### 2.1.1 错误监听（plugins/error.ts）

采集内容：
- JS 运行时错误（window.onerror）
- Promise 未捕获错误（unhandledrejection）
- 资源加载错误（img/script/link 的 error 事件）

```typescript
interface ErrorData {
  type: 'js' | 'promise' | 'resource';
  message: string;
  stack?: string;
  source_url?: string;
  line?: number;
  column?: number;
  // ... 其他字段见采集数据详情
}
```

### 2.1.2 接口监听（plugins/network.ts）

劫持方式：
- 重写 `XMLHttpRequest.prototype.open/send`
- 重写 `window.fetch`

采集内容：
- 请求 URL、Method、Headers、Body（脱敏）
- 响应 Status、Headers、Body（截断至 10KB）
- 请求耗时

### 2.1.3 性能采集（plugins/performance.ts）

使用 Performance API 采集：
- FP（First Paint）
- FCP（First Contentful Paint）
- LCP（Largest Contentful Paint）
- CLS（Cumulative Layout Shift）
- TTFB（Time to First Byte）
- 页面加载时间、DNS 解析时间

采样率：默认 10%

### 2.1.4 上报策略（混合策略）

| 错误级别 | 处理方式 | 说明 |
|---------|---------|------|
| P0（SyntaxError, ReferenceError） | 实时上报 | 立即发送，不等待批量 |
| P1（TypeError, 资源加载失败） | 批量上报 | 5秒或队列满10条时发送 |
| 性能数据 | 采样 + 批量 | 默认采样率 10%，批量上报 |
| 接口错误 | 实时上报 | 接口失败立即上报 |

### 2.1.5 本地存储与队列

```typescript
interface StoreConfig {
  maxQueueSize: 100;           // 最大队列长度
  flushInterval: 5000;         // 批量上报间隔（ms）
  retryMaxCount: 3;            // 最大重试次数
  retryInterval: 30000;        // 重试间隔（ms）
  storageType: 'indexedDB';    // 优先 IndexedDB，降级 localStorage
}
```

队列管理：
- 上报失败时存入 IndexedDB，网络恢复后按 FIFO 顺序补发
- 队列满时丢弃最旧的数据（优先保留 P0 错误）
- 页面 `beforeunload` 时强制 flush 队列

### 2.1.6 错误去重（客户端）

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

### 2.1.7 敏感数据脱敏

| 数据类型 | 脱敏规则 |
|---------|---------|
| 请求/响应 Body | 自动检测并替换：`password`, `token`, `secret`, `apiKey`, `authorization` 等字段值为 `[REDACTED]` |
| URL 查询参数 | 自动移除：`token`, `auth`, `key`, `secret` 等参数 |
| 用户输入 | 输入框 type=password 的内容替换为 `[REDACTED]` |
| Cookie | 默认不上报，如需上报则过滤敏感字段 |

### 2.1.8 采集数据详情

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

## 2.2 埋点核心 API（SDK）

### 初始化配置

```typescript
interface TrackingConfig {
  enableTracking: boolean;              // 是否开启埋点（默认 true）
  autoTrack: {
    pageView: boolean;                  // 自动采集页面浏览
    click: boolean;                     // 自动采集元素点击
    pageLeave: boolean;                 // 自动采集页面离开+停留时长
    exposure: boolean;                  // 半自动曝光采集
  };
  anonymousIdPrefix: string;            // 匿名 ID 前缀（默认 'anon_'）
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
    autoTrack: { pageView: true, click: true, pageLeave: true, exposure: false },
  },
});
```

### 代码埋点核心 API

```typescript
// ① 追踪自定义事件
Monitor.track(eventName: string, properties?: Record<string, any>): void

// ② 用户识别：匿名用户 → 登录用户
Monitor.identify(userId: string): void

// ③ 设置用户属性
Monitor.setUserProperties(properties: Record<string, any>): void

// ④ 用户属性追加（列表类型）
Monitor.appendUserProperties(properties: Record<string, string[]>): void

// ⑤ 设置全局超级属性
Monitor.registerSuperProperties(properties: Record<string, any>): void

// ⑥ 清除超级属性
Monitor.unregisterSuperProperty(propertyName: string): void
Monitor.clearSuperProperties(): void

// ⑦ 事件时长统计
Monitor.trackTimerStart(eventName: string): void
Monitor.trackTimerEnd(eventName: string, properties?: Record<string, any>): void

// ⑧ 用户注销
Monitor.logout(): void

// ⑨ 设置匿名 ID（高级用法）
Monitor.identify_anonymous(anonymousId: string): void
```

### 用户身份体系

```
访问网站
  ↓
分配匿名 ID（UUID，存 localStorage）
  ↓ 用户登录
调用 identify(userId)
  ↓
后端关联：匿名 ID → 登录 ID（profile_merge）
  ↓ 用户登出
调用 logout()，重置为新匿名 ID
```

---

## 2.3 后端数据上报接口

### 2.3.1 SDK 数据上报接口

```
POST /api/v1/collect              # SDK 数据上报（按 app_id + app_key 校验）
  Headers: X-App-Id, X-App-Key
  Body: { type: 'error'|'network'|'performance'|'breadcrumb', data: {...} }
  Response: { code: 0, message: 'ok' }

GET  /api/v1/collect/health       # SDK 健康检查
  Response: { status: 'ok', version: '1.0.0' }
```

### 2.3.2 埋点数据上报扩展

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
    "properties": { "product_id": "sku_001", "price": 99.9 },
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
    "operation": "set",
    "properties": { "$name": "张三", "membership": "premium" }
  }
}

// 用户 ID 关联（登录时调用）
POST /api/v1/collect
{
  "type": "track_signup",
  "data": {
    "distinct_id": "user_123",
    "original_id": "anon_abc123",
    "is_login_id": true
  }
}
```

---

## 2.4 数据库表创建

### 2.4.1 核心监控表

详见 [database.md](database.md) 中的：
- `users` 表
- `groups` 表
- `projects` 表
- `project_members` 表
- `js_errors` 表
- `network_errors` 表
- `performance_data` 表

### 2.4.2 埋点相关表

详见 [database.md](database.md) 中的：
- `track_events` 表
- `track_user_profiles` 表
- `track_id_mapping` 表

---

## 2.5 前端页面实现

### 2.5.1 项目创建页

功能：
- 表单：项目名称、所属分组、描述
- 自动生成 app_id 和 app_key
- 显示 SDK 接入代码示例

### 2.5.2 错误列表页

功能：
- 表格展示错误列表（时间、类型、消息、URL、浏览器）
- 筛选：时间范围、错误类型、浏览器、版本
- 分页
- 点击行进入错误详情

### 2.5.3 错误详情页

功能：
- 错误基本信息（消息、类型、URL、设备信息）
- 堆栈信息（如有 Source Map 则显示解析后的）
- 面包屑操作路径
- AI 分析按钮（触发 AI 分析）

---

## 2.6 本阶段验收标准

- [ ] SDK 能正确捕获 JS 错误、Promise 错误、资源加载错误
- [ ] SDK 能正确劫持 fetch/XHR 并采集接口数据
- [ ] SDK 能采集性能指标（FP/FCP/LCP/CLS/TTFB）
- [ ] SDK 埋点 API（track/identify/setUserProperties/registerSuperProperties）可用
- [ ] 后端 `/api/v1/collect` 能接收并存储错误、网络、性能、埋点数据
- [ ] 前端能创建项目并查看项目列表
- [ ] 前端能查看错误列表和错误详情
- [ ] 数据库表创建成功，数据能正常写入
