# 附录：SDK 设计

## SDK 采集与上报设计

### 上报策略（混合策略）

| 错误级别 | 处理方式 | 说明 |
|---------|---------|------|
| P0（SyntaxError, ReferenceError） | 实时上报 | 立即发送，不等待批量 |
| P1（TypeError, 资源加载失败） | 批量上报 | 5秒或队列满10条时发送 |
| 性能数据 | 采样 + 批量 | 默认采样率 10%，批量上报 |
| 接口错误 | 实时上报 | 接口失败立即上报 |

### 本地存储与队列

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

### 错误去重（客户端）

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

### 敏感数据脱敏

| 数据类型 | 脱敏规则 |
|---------|---------|
| 请求/响应 Body | 自动检测并替换：`password`, `token`, `secret`, `apiKey`, `authorization` 等字段值为 `[REDACTED]` |
| URL 查询参数 | 自动移除：`token`, `auth`, `key`, `secret` 等参数 |
| 用户输入 | 输入框 type=password 的内容替换为 `[REDACTED]` |
| Cookie | 默认不上报，如需上报则过滤敏感字段 |

---

## 采集数据详情

### 页面信息
- `url` / `referrer` / `title`
- `viewport` 尺寸 / `screen` 分辨率
- `language` / `timezone`

### 设备信息
- `userAgent`（解析出 browser、browserVersion、os、osVersion、device）
- `deviceMemory` / `hardwareConcurrency`
- `connection`（网络类型：4G/WiFi）

### 错误信息
- 错误类型：`js` / `promise` / `resource` / `vue` / `react`
- 错误消息、完整堆栈
- 资源加载失败的 URL
- Vue/React 组件名（如检测到框架）

### 接口信息
- 请求 URL、Method、Headers、Body（脱敏）
- 响应 Status、Headers、Body（截断至 10KB）
- 请求耗时

### 面包屑（操作路径）
- 点击事件（目标元素 selector）
- 路由跳转
- 控制台日志
- 接口请求/响应
- 用户输入（脱敏）

### 性能指标
- FP、FCP、LCP、CLS、TTFB、FID
- 页面加载时间、DNS 解析时间

---

## SDK 模块文件结构

```
sdk/src/
├── core/
│   ├── monitor.ts        # 主入口，初始化配置
│   ├── reporter.ts       # 数据上报模块
│   ├── store.ts          # 本地缓存/队列
│   ├── identity.ts       # 用户身份管理（匿名 ID / 登录 ID / ID 关联）
│   ├── super-props.ts    # 超级属性管理（register / unregister）
│   ├── timer.ts          # 事件时长计时器（trackTimerStart / trackTimerEnd）
│   └── utils.ts          # 工具函数
├── plugins/
│   ├── error.ts          # JS 错误监听
│   ├── console.ts        # 控制台日志劫持
│   ├── network.ts        # fetch / XHR 监控
│   ├── performance.ts    # 性能指标采集
│   ├── behavior.ts       # 用户行为追踪（代码埋点）
│   ├── auto-track.ts     # 全埋点/无埋点自动采集
│   ├── exposure.ts       # 曝光追踪
│   ├── profile.ts        # 用户画像操作
│   └── breadcrumb.ts     # 面包屑
├── types/
│   └── index.ts          # 类型定义
└── index.ts              # SDK 入口
```

---

## SDK 初始化配置

```typescript
interface MonitorConfig {
  appId: string;
  appKey: string;
  server: string;
  
  // 错误监控
  enableError?: boolean;
  enableNetwork?: boolean;
  enablePerformance?: boolean;
  
  // 埋点
  tracking?: TrackingConfig;
  
  // 采样率
  sampleRate?: number;
  
  // 上报配置
  maxQueueSize?: number;
  flushInterval?: number;
  
  // 用户信息
  release?: string;
  environment?: 'production' | 'staging' | 'development';
}

interface TrackingConfig {
  enableTracking: boolean;
  autoTrack: {
    pageView: boolean;
    click: boolean;
    pageLeave: boolean;
    exposure: boolean;
  };
  anonymousIdPrefix: string;
  trackFlushInterval: number;
  trackMaxBatchSize: number;
}
```

---

## 代码埋点核心 API

```typescript
// 追踪自定义事件
Monitor.track(eventName: string, properties?: Record<string, any>): void

// 用户识别：匿名用户 → 登录用户
Monitor.identify(userId: string): void

// 设置用户属性
Monitor.setUserProperties(properties: Record<string, any>): void

// 用户属性追加（列表类型）
Monitor.appendUserProperties(properties: Record<string, string[]>): void

// 设置全局超级属性
Monitor.registerSuperProperties(properties: Record<string, any>): void

// 清除超级属性
Monitor.unregisterSuperProperty(propertyName: string): void
Monitor.clearSuperProperties(): void

// 事件时长统计
Monitor.trackTimerStart(eventName: string): void
Monitor.trackTimerEnd(eventName: string, properties?: Record<string, any>): void

// 用户注销
Monitor.logout(): void

// 设置匿名 ID（高级用法）
Monitor.identify_anonymous(anonymousId: string): void
```

---

## 全埋点自动采集事件

```typescript
// $page_view - 页面浏览事件
{
  event: '$page_view',
  properties: {
    $page_url: string;
    $page_title: string;
    $referrer: string;
    $referrer_title: string;
    $viewport_width: number;
    $viewport_height: number;
    $is_first_visit: boolean;
    $is_first_day: boolean;
  }
}

// $element_click - 元素点击事件
{
  event: '$element_click',
  properties: {
    $element_id: string;
    $element_class: string;
    $element_type: string;
    $element_name: string;
    $element_content: string;
    $element_path: string;
    $element_xpath: string;
    $page_url: string;
    $page_x: number;
    $page_y: number;
  }
}

// $page_leave - 页面离开事件
{
  event: '$page_leave',
  properties: {
    $page_url: string;
    $page_title: string;
    $stay_duration: number;
    $leave_reason: 'navigation' | 'close' | 'other';
  }
}
```

---

## 曝光追踪（半自动）

```html
<div
  data-track-imp="true"
  data-track-event="product_exposure"
  data-track-attrs='{"product_id": "sku_001", "position": 3, "list": "首页推荐"}'
>
  商品卡片内容
</div>
```

- 使用 IntersectionObserver 监听元素可见性
- 支持 once 模式（只触发一次）和 always 模式（每次出现都触发）

---

## 上报数据格式

```typescript
// 单条埋点事件上报
{
  type: 'track',
  data: {
    distinct_id: string;
    anonymous_id: string;
    is_login_id: boolean;
    event: string;
    properties: Record<string, any>;
    client_time: number;
  }
}

// 批量上报
{
  type: 'track_batch',
  data: Array<{
    event: string;
    distinct_id: string;
    properties: Record<string, any>;
  }>
}

// 用户属性更新
{
  type: 'profile',
  data: {
    distinct_id: string;
    is_login_id: boolean;
    operation: 'set' | 'set_once' | 'append' | 'unset';
    properties: Record<string, any>;
  }
}

// 用户 ID 关联（登录时调用）
{
  type: 'track_signup',
  data: {
    distinct_id: string;
    original_id: string;
    is_login_id: boolean;
  }
}
```

---

## 用户身份生命周期

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

## 埋点规范与最佳实践

### 事件命名规范
```
动词_名词          → purchase_product / view_page / click_button
$前缀              → 系统预置事件（$page_view / $element_click）
小写+下划线        → 统一格式，避免大小写混用
```

### 属性命名规范
```
$前缀              → SDK 自动采集的预置属性（$browser / $os / $page_url）
业务前缀_属性名    → product_id / order_amount / user_level
避免使用            → id（太通用）/ data（无意义）
```

### 埋点方案分层
```
Level 1 - 全埋点（自动）：$page_view / $element_click
  → 覆盖所有页面流量，无需开发干预，运营事后分析

Level 2 - 关键路径代码埋点（手动）：
  → 核心业务转化：注册 / 登录 / 购买 / 提交
  → 这些事件需要携带业务属性，全埋点无法满足

Level 3 - 用户属性：
  → identify + setUserProperties
  → 用于用户分群分析（membership / city / age_group）
```
