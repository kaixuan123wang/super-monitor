# Phase 5：前端埋点信息采集

## 目标

本阶段不扩展用户/分组/权限体系，重点收敛到前端埋点数据的采集、入库与查询闭环。

1. **SDK 自动采集增强**：完善 `$page_view` / `$element_click` / `$page_leave` 的上下文信息。
2. **埋点曝光追踪**：实现 `exposure.ts`，支持 `IntersectionObserver` 半自动曝光采集。
3. **用户身份与属性采集**：继续支持 `identify` / `track_signup` / `setUserProperties` / `appendUserProperties`。
4. **后端埋点入库**：确保 `track` / `track_batch` / `profile` / `track_signup` 写入对应表。
5. **用户画像查询**：支持按属性筛选用户，并查看用户事件时间线。

---

## 5.1 SDK 采集能力

### 5.1.1 自动事件

SDK 初始化后可根据 `tracking.autoTrack` 开关自动采集：

```typescript
Monitor.init({
  appId: 'app_xxx',
  appKey: 'key_xxx',
  server: 'https://monitor.example.com',
  tracking: {
    enableTracking: true,
    autoTrack: {
      pageView: true,
      click: true,
      pageLeave: true,
      exposure: true,
    },
  },
});
```

自动事件：

- `$page_view`：页面浏览。
- `$element_click`：元素点击。
- `$page_leave`：页面离开与停留时长。
- 自定义曝光事件：由元素上的 `data-track-event` 决定。

### 5.1.2 通用上下文

每条埋点事件都应携带：

- `distinct_id` / `anonymous_id` / `is_login_id`
- 页面 URL、标题、referrer
- viewport、screen、language、timezone
- browser、browser_version、os、os_version、device_type
- release、environment、sdk_version
- 已注册的超级属性

---

## 5.2 埋点曝光追踪

### 5.2.1 半自动曝光采集

```html
<div
  data-track-imp="true"
  data-track-event="product_exposure"
  data-track-attrs='{"product_id": "sku_001", "position": 3, "list": "home_recommend"}'
  data-track-mode="once"
>
  商品卡片内容
</div>
```

字段约定：

| 属性 | 说明 |
|------|------|
| `data-track-imp` | 标记元素需要采集曝光，值为 `true` 时启用 |
| `data-track-event` | 曝光事件名，未提供时默认 `$element_exposure` |
| `data-track-attrs` | JSON 字符串，作为业务属性合入事件属性 |
| `data-track-mode` | `once` 只触发一次，`always` 每次重新进入视口都触发 |

### 5.2.2 SDK 实现要求

- 使用 `IntersectionObserver` 监听进入/离开视口。
- 默认阈值为 0.5，可由 SDK 内部配置扩展。
- 触发事件时补充元素基础信息：
  - `$element_id`
  - `$element_class`
  - `$element_type`
  - `$element_content`
  - `$element_path`
  - `$page_url`
  - `$viewport_width`
  - `$viewport_height`
  - `$exposure_ratio`
- 支持 DOM 动态新增元素，使用 `MutationObserver` 追加监听。

---

## 5.3 上报与入库

SDK 继续使用统一上报接口：

```http
POST /api/v1/collect
Headers:
  X-App-Id: {app_id}
  X-App-Key: {app_key}
Body:
  { "type": "track", "data": { ... } }
```

后端分发规则：

| type | 目标 |
|------|------|
| `track` | 写入 `track_events`，并更新 `track_user_profiles.total_events` |
| `track_batch` | 批量写入 `track_events` |
| `profile` | 更新 `track_user_profiles.properties` |
| `track_signup` | 写入 `track_id_mapping`，关联匿名 ID 与登录 ID |

---

## 5.4 用户画像查询

### 5.4.1 API

```http
GET /api/tracking/users
Query: project_id, page, page_size, keyword, filters

GET /api/tracking/users/:distinct_id
Query: project_id

GET /api/tracking/users/:distinct_id/events
Query: project_id, page, page_size, event_name, start_time, end_time
```

`filters` 使用 JSON 序列化数组：

```json
[
  { "property": "city", "operator": "eq", "value": "Beijing" },
  { "property": "membership", "operator": "eq", "value": "premium" }
]
```

### 5.4.2 管理端页面

用户画像页面提供：

- 项目选择。
- 关键字搜索：`distinct_id` / `user_id` / `name` / `email`。
- 属性筛选：按用户属性做等值或包含筛选。
- 用户列表：展示 `distinct_id`、最后访问时间、事件数、核心属性。
- 用户详情：展示用户属性和最近事件时间线。

---

## 5.5 验收标准

- [x] SDK 初始化后能自动采集 `$page_view`。
- [x] 点击页面元素能产生 `$element_click`。
- [x] 页面离开时能产生 `$page_leave`。
- [x] 曝光元素进入视口后能自动上报 `data-track-event` 指定事件。
- [x] `Monitor.track` 能写入 `track_events`。
- [x] `Monitor.identify` 能写入 `track_id_mapping`。
- [x] `Monitor.setUserProperties` 能更新 `track_user_profiles.properties`。
- [x] 管理端能筛选用户画像并查看事件时间线。
