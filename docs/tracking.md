# 附录：埋点模块设计

## 埋点能力对比与选型

| 能力 | 神策数据 | GrowingIO | Mixpanel/Amplitude | 本平台实现 |
|------|---------|-----------|-------------------|-----------|
| 代码埋点 | ✅ | ✅ | ✅ | ✅ Phase 1 |
| 全埋点（无埋点） | ✅ | ✅（核心强项） | ✅ | ✅ Phase 2 |
| 用户识别（匿名→登录） | ✅ IDM 2.0/3.0 | ✅ | ✅ | ✅ Phase 1 |
| 用户属性（Profile） | ✅ | ✅ | ✅ People API | ✅ Phase 1 |
| 超级属性（全局附加） | ✅ | ✅ | ✅ Super Properties | ✅ Phase 1 |
| 事件时长统计 | ✅ | ✅ | ✅ | ✅ Phase 1 |
| 元素曝光追踪 | ✅ | ✅（半自动 imp） | ❌ | ✅ Phase 2 |
| 漏斗分析 | ✅ | ✅ | ✅ | ✅ Phase 2 |
| 留存分析 | ✅ | ✅ | ✅ | ✅ Phase 3 |
| 埋点管理平台 | ✅ | ✅ | ✅ | ✅ Phase 2 |
| 可视化圈选 | ✅ | ✅ | ❌ | ⬜ Phase 4（预留） |

---

## 埋点 API 风格

参考 Mixpanel/Amplitude 现代设计，兼容 Sensors Data 用户关联体系：

```typescript
// 追踪自定义事件（对标 Mixpanel.track / 神策 track）
Monitor.track(eventName: string, properties?: Record<string, any>): void

// 用户识别（对标 Mixpanel.identify / 神策 login）
Monitor.identify(userId: string): void

// 设置用户属性（对标 Mixpanel.people.set / Amplitude setUserProperties）
Monitor.setUserProperties(properties: Record<string, any>): void

// 用户属性追加（对标神策 profileAppend）
Monitor.appendUserProperties(properties: Record<string, string[]>): void

// 设置全局超级属性（对标 Mixpanel.register）
Monitor.registerSuperProperties(properties: Record<string, any>): void

// 清除超级属性
Monitor.unregisterSuperProperty(propertyName: string): void
Monitor.clearSuperProperties(): void

// 事件时长统计（对标神策 trackTimerStart / trackTimerEnd）
Monitor.trackTimerStart(eventName: string): void
Monitor.trackTimerEnd(eventName: string, properties?: Record<string, any>): void

// 用户注销（对标 Mixpanel.reset / 神策 logout）
Monitor.logout(): void

// 设置匿名 ID（高级用法）
Monitor.identify_anonymous(anonymousId: string): void
```

---

## 用户身份体系设计

借鉴神策 IDM 2.0 的简易用户关联方案：

```
┌─────────────────────────────────────────────────────┐
│                  用户身份生命周期                      │
│                                                     │
│  访问网站                                            │
│    ↓                                                │
│  分配匿名 ID（UUID，存 localStorage）                 │
│    ↓ 用户登录                                        │
│  调用 identify(userId)                               │
│    ↓                                                │
│  后端关联：匿名 ID → 登录 ID（profile_merge）          │
│    ↓ 用户登出                                        │
│  调用 logout()，重置为新匿名 ID                       │
└─────────────────────────────────────────────────────┘
```

### 匿名 ID 生成规则

```typescript
function generateAnonymousId(): string {
  // 优先读取 localStorage 已有的 anonymous_id
  const stored = localStorage.getItem('__monitor_anon_id');
  if (stored) return stored;
  // 否则生成新的 UUID
  const id = 'anon_' + crypto.randomUUID();
  localStorage.setItem('__monitor_anon_id', id);
  return id;
}
```

### 事件上报 Payload

```typescript
interface TrackPayload {
  distinct_id: string;         // 当前标识：登录前=匿名ID，登录后=用户ID
  anonymous_id: string;        // 始终为设备匿名 ID
  is_login_id: boolean;        // distinct_id 是否为登录 ID
  event: string;
  properties: Record<string, any>;
  time: number;                // 事件发生时间戳（ms）
}
```

---

## 全埋点（无埋点）自动采集

SDK 内部自动采集的预置事件（无需手动调用）：

### $page_view - 页面浏览事件

```typescript
{
  event: '$page_view',
  properties: {
    $page_url: 'https://example.com/product/123',
    $page_title: '商品详情页',
    $referrer: 'https://example.com/home',
    $referrer_title: '首页',
    $viewport_width: 1920,
    $viewport_height: 1080,
    $is_first_visit: false,
    $is_first_day: false,
  }
}
```

### $element_click - 元素点击事件

```typescript
{
  event: '$element_click',
  properties: {
    $element_id: 'btn-submit',
    $element_class: 'btn btn-primary',
    $element_type: 'button',
    $element_name: '立即购买',
    $element_content: '立即购买',
    $element_path: 'body > div > button#btn-submit',
    $element_xpath: '/html/body/div/button',
    $page_url: 'https://example.com/product/123',
    $page_x: 120,
    $page_y: 350,
  }
}
```

### $page_leave - 页面离开事件

```typescript
{
  event: '$page_leave',
  properties: {
    $page_url: 'https://example.com/product/123',
    $page_title: '商品详情页',
    $stay_duration: 45.2,
    $leave_reason: 'navigation',
  }
}
```

---

## 元素曝光半自动采集

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

SDK 内部使用 IntersectionObserver 监听元素可见性，当元素从不可见→可见时，自动发送 data-track-event 指定的埋点事件。支持 once 模式（只触发一次）和 always 模式（每次出现都触发）。

---

## 埋点管理平台前端页面

```
tracking/
├── events/               # 事件管理
│   ├── index.vue         # 事件列表（事件名、分类、数据量、状态）
│   └── detail.vue        # 事件详情（属性列表、近期趋势迷你图）
├── analysis/
│   ├── event-analysis.vue    # 事件分析（折线/柱状图，支持分组、筛选）
│   ├── funnel.vue            # 漏斗分析（可视化漏斗图 + 明细数据）
│   └── retention.vue         # 留存分析（热力表格 / 曲线图）
├── users/
│   ├── index.vue             # 用户列表（条件筛选、分页）
│   └── profile.vue           # 用户详情（属性面板 + 事件流水线）
└── debug/
    └── live-events.vue       # 实时事件流（Debug 模式，SSE 展示）
```

### 核心页面交互设计

**事件分析页（参考 Mixpanel Events）**
- 左侧：选择事件、添加筛选条件、选择指标（UV/PV/Sum/Avg）
- 右侧：时间序列折线图 + 数据表格
- 支持「按维度分组」（如按浏览器、城市、设备类型）
- 时间范围选择器（最近7天/30天/自定义）

**漏斗分析页（参考神策漏斗）**
- 拖拽排序的步骤配置区
- 可视化漏斗图（各步骤转化率 + 流失率）
- 下方明细：各步骤转化人数、平均转化时长
- 支持分组对比（如 A/B 实验对比不同渠道的转化）

**留存分析页（参考 Amplitude Retention）**
- 上方：初始事件 + 回访事件 + 时间范围配置
- 中间：热力矩阵（cohort 表格，颜色深浅表示留存率高低）
- 下方：平均留存曲线图

**用户画像页（参考神策用户分群）**
- 左侧筛选面板：按用户属性筛选（城市=北京，membership=premium 等）
- 右侧用户列表：显示 distinct_id、最后访问时间、事件数等
- 点击进入用户详情：属性面板 + 时间线（事件流水）

---

## 埋点数据处理流程（后端）

```
SDK 上报 /api/v1/collect (type=track)
              ↓
    接收、鉴权（app_id + app_key）
              ↓
    IP 解析地理信息（异步，不阻塞上报）
              ↓
    写入 Redis Stream（track_events_stream）
              ↓ 异步消费
    Consumer Worker（并发多个）
        ↓               ↓
   写入 PostgreSQL   更新用户 Profile
   track_events      track_user_profiles
        ↓
   触发实时事件流推送（SSE → Debug 页面）
        ↓
   触发预聚合任务（每分钟 → track_event_stats_hourly）
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
