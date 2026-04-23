# @js-monitor/sdk

零依赖的浏览器端 JS 监控 / 埋点 SDK。支持前端错误监控、接口监控、性能采集、用户行为埋点、全埋点自动采集、曝光追踪等功能。

## 安装

```bash
npm install @js-monitor/sdk
```

或浏览器直接引入：

```html
<script src="https://your-cdn.com/sdk.iife.js"></script>
```

## 快速开始

```typescript
import Monitor from '@js-monitor/sdk';

Monitor.init({
  appId: 'your-app-id',
  appKey: 'your-app-key',
  server: 'https://monitor.example.com',
  release: '1.2.3',
  environment: 'production',
});

// 代码埋点
Monitor.track('purchase', { product_id: 'sku_001', price: 99.9 });

// 用户登录后关联身份
Monitor.identify('user_123');
```

## 初始化配置

```typescript
Monitor.init({
  appId: string;          // 必填，项目标识
  appKey: string;         // 必填，SDK 鉴权密钥
  server: string;         // 必填，监控服务端地址

  release?: string;       // 代码版本（用于 Source Map 关联）
  environment?: string;   // 运行环境：production / staging / development
  debug?: boolean;        // 是否开启调试日志（默认 false）
  performanceSampleRate?: number;  // 性能采样率 0~1（默认 0.1）

  // 监控插件开关（默认全部开启）
  plugins?: {
    error?: boolean;        // JS/Promise/资源错误
    console?: boolean;      // 控制台日志劫持（写入面包屑）
    network?: boolean;      // fetch / XHR 接口监控
    performance?: boolean;  // 性能指标采集
    breadcrumb?: boolean;   // 用户操作路径
  };

  // 埋点配置
  tracking?: {
    enableTracking?: boolean;
    autoTrack?: {
      pageView?: boolean;   // 自动采集页面浏览
      click?: boolean;      // 自动采集元素点击
      pageLeave?: boolean;  // 自动采集页面离开+停留时长
      exposure?: boolean;   // 半自动曝光采集（需元素标记 data-track-imp）
    };
    trackFlushInterval?: number;  // 埋点批量上报间隔 ms（默认 3000）
    trackMaxBatchSize?: number;   // 埋点批量最大条数（默认 20）
  };

  // 脱敏配置
  sanitize?: {
    sensitiveFields?: string[];     // body 中需替换的敏感字段
    sensitiveQueryKeys?: string[];  // URL 参数中需移除的 key
    maxBodySize?: number;           // body 最大截断长度（默认 10240）
  };
});
```

## 代码埋点 API

### track — 追踪自定义事件

```typescript
Monitor.track('button_click');

Monitor.track('purchase', {
  product_id: 'sku_001',
  product_name: '商品A',
  price: 99.9,
  quantity: 2,
});
```

> 事件会自动附加 `$page_url`、`$browser`、`$os` 等预置属性，以及通过 `registerSuperProperties` 注册的全局属性。

### identify — 用户识别

```typescript
// 用户登录成功后调用，将匿名用户关联到登录用户
Monitor.identify('user_123456');
```

### setUserProperties — 设置用户属性

```typescript
// 覆盖更新
Monitor.setUserProperties({
  $name: '张三',
  $email: 'zhangsan@example.com',
  membership: 'premium',
});

// 仅首次设置有效（不覆盖已有值）
Monitor.setUserPropertiesOnce({
  first_referral: 'wechat_article',
});

// 向列表类型属性追加
Monitor.appendUserProperties({
  tags: ['新用户', '活跃'],
});

// 删除属性
Monitor.unsetUserProperty('temporary_field');
```

### registerSuperProperties — 超级属性（全局附加）

```typescript
// 注册后，后续所有事件自动携带这些属性
Monitor.registerSuperProperties({
  app_version: '2.1.0',
  channel: 'organic',
});

Monitor.unregisterSuperProperty('channel');
Monitor.clearSuperProperties();
```

### trackTimer — 事件时长统计

```typescript
Monitor.trackTimerStart('video_play');
// ... 用户播放视频 ...
Monitor.trackTimerEnd('video_play', { video_id: 'v_001' });
// 自动上报事件，并附加 $event_duration 字段（秒）
```

### logout — 用户注销

```typescript
Monitor.logout();  // 清除登录身份，重置为新的匿名 ID
```

## 全埋点自动采集

SDK 根据 `tracking.autoTrack` 配置自动采集以下事件，**无需手动调用 `track`**：

| 事件名 | 触发时机 | 关键属性 |
|--------|---------|---------|
| `$page_view` | 页面加载 / SPA 路由切换 | `$page_url`, `$page_title`, `$referrer`, `$is_first_visit` |
| `$element_click` | 点击可交互元素 | `$element_id`, `$element_class`, `$element_content`, `$element_path`, `$page_x`, `$page_y` |
| `$page_leave` | 页面离开 | `$page_url`, `$stay_duration`, `$leave_reason` |

> SPA 兼容：自动监听 `popstate`、`hashchange`，劫持 `history.pushState/replaceState`。

## 曝光追踪

对需要追踪曝光的元素添加 HTML 属性：

```html
<div
  data-track-imp="true"
  data-track-event="product_exposure"
  data-track-attrs='{"product_id": "sku_001", "position": 3}'
  data-track-mode="once"
>
  商品卡片内容
</div>
```

| 属性 | 说明 |
|------|------|
| `data-track-imp` | `true` 时启用曝光追踪 |
| `data-track-event` | 曝光事件名，默认 `$element_exposure` |
| `data-track-attrs` | JSON 字符串，作为业务属性 |
| `data-track-mode` | `once` 只触发一次，`always` 每次进入视口都触发 |

同时需在初始化时开启 `tracking.autoTrack.exposure: true`。

## 自动监控（零配置）

SDK 默认自动采集以下数据：

- **JS 错误**：`window.onerror`、`unhandledrejection`、资源加载错误
- **接口监控**：`fetch` / `XHR` 请求/响应、耗时、失败状态
- **性能指标**：FP、FCP、LCP、CLS、TTFB、FID、DNS、TCP、加载时间（默认 10% 采样）
- **面包屑**：点击、路由跳转、控制台日志、接口请求

敏感数据（password、token 等）自动脱敏为 `[REDACTED]`。

## Vue 集成示例

```typescript
// main.ts
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
```

## React 集成示例

```typescript
// index.tsx
import Monitor from '@js-monitor/sdk';

Monitor.init({
  appId: process.env.REACT_APP_MONITOR_APP_ID!,
  appKey: process.env.REACT_APP_MONITOR_APP_KEY!,
  server: process.env.REACT_APP_MONITOR_SERVER!,
  release: process.env.REACT_APP_VERSION,
});
```

## 最佳实践

1. **尽早初始化**：在应用入口处初始化 SDK，确保捕获早期错误。
2. **release 对齐**：每次发版更新 `release`，与 Source Map 版本一致。
3. **identify 时机**：用户登录成功后立即调用。
4. **超级属性**：放置不频繁变化的上下文，如 `app_version`、`channel`。
5. **事件命名**：小写+下划线，如 `complete_purchase`。
6. **曝光追踪**：仅对商品卡片、广告位等关键元素标记，不要全量标记。

## 管理端使用

部署监控管理端后，访问管理页面完成以下操作：

1. **注册账号** → 创建项目 → 获取 `appId` 和 `appKey`
2. **错误监控**：查看错误列表、堆栈、面包屑、AI 分析结果
3. **接口监控**：查看接口报错、状态码分布、耗时趋势
4. **Source Map**：上传 `.map` 文件，还原压缩堆栈到原始源码
5. **告警配置**：配置错误激增、P0 错误等规则，支持 Webhook 通知
6. **埋点分析**：事件分析、漏斗分析、留存分析、用户画像、实时事件流 Debug

详细管理端教程请参考项目文档。

## 类型支持

```typescript
import Monitor, { type MonitorConfig, type Properties } from '@js-monitor/sdk';
```

## 许可证

MIT
