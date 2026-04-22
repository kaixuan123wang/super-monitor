# Phase 3：数据可视化

## 目标

1. 仪表盘图表（ECharts）
2. 错误趋势、浏览器分布、设备分布
3. 实时数据推送（SSE）
4. **埋点全埋点**：SDK 实现 `auto-track.ts`（$page_view / $element_click / $page_leave 自动采集）
5. **埋点管理平台**：前端事件定义管理页、事件属性管理页
6. **埋点分析**：事件分析页（折线图 + 分组维度）

---

## 3.1 仪表盘设计

### 3.1.1 概览数据 API

```
GET  /api/dashboard/overview      # 仪表盘概览数据
  Query: project_id, days: 7|30
  Response: {
    total_errors, total_network_errors,
    error_trend: [...],
    browser_distribution: [...],
    os_distribution: [...],
    top_errors: [...],
    avg_performance: { fp, fcp, lcp, cls }
  }
```

### 3.1.2 图表组件

| 图表 | 类型 | 数据 |
|------|------|------|
| 错误趋势 | 折线图 | 按天/小时聚合的错误数 |
| 浏览器分布 | 饼图/环形图 | 各浏览器占比 |
| 操作系统分布 | 饼图 | 各 OS 占比 |
| 设备类型分布 | 柱状图 | desktop / mobile / tablet |
| 性能指标 | 仪表盘/数字卡片 | FP/FCP/LCP/CLS 平均值 |
| Top 错误 | 表格 | 按出现次数排序 |

---

## 3.2 实时数据推送（SSE）

### 3.2.1 SSE 接口

```
GET  /api/dashboard/realtime      # 实时数据（SSE 连接）
  Headers: Accept: text/event-stream
  Events:
    - init: { project_ids, connection_id }
    - error: { id, message, error_type, created_at, project_id }
    - alert: { rule_id, alert_content, severity }
    - heartbeat: { timestamp }
```

### 3.2.2 SSE 实现细节

- 连接建立时，后端记录用户 ID 与连接映射
- 用户登录后，推送其所有有权限项目的实时告警
- 心跳间隔：30 秒
- 断线重连：前端自动重连，指数退避（1s → 2s → 4s → 8s，最大 30s）
- 消息格式：`event: error\ndata: {...}\n\n`

### 3.2.3 前端 SSE 封装

```typescript
class SSEClient {
  private eventSource: EventSource | null = null;
  private reconnectDelay = 1000;
  private maxReconnectDelay = 30000;

  connect(url: string) {
    this.eventSource = new EventSource(url);
    this.eventSource.onmessage = (e) => {
      const data = JSON.parse(e.data);
      this.handleEvent(data);
    };
    this.eventSource.onerror = () => {
      this.reconnect(url);
    };
  }

  private reconnect(url: string) {
    setTimeout(() => {
      this.reconnectDelay = Math.min(this.reconnectDelay * 2, this.maxReconnectDelay);
      this.connect(url);
    }, this.reconnectDelay);
  }
}
```

---

## 3.3 全埋点自动采集（SDK）

### 3.3.1 自动采集事件

```typescript
// $page_view - 页面浏览事件（SPA 路由切换时自动发送）
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

// $element_click - 元素点击事件
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

// $page_leave - 页面离开事件
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

### 3.3.2 实现要点

- **$page_view**：监听 `popstate` / `hashchange`，SPA 框架通过 MutationObserver 检测 URL 变化
- **$element_click**：document 级别事件委托，过滤可交互元素（a/button/input/带点击事件的元素）
- **$page_leave**：`beforeunload` / `pagehide` 事件，计算停留时长

---

## 3.4 埋点管理平台前端

### 3.4.1 事件定义管理页

功能：
- 事件列表（事件名、展示名、分类、数据量、状态）
- 创建/编辑事件定义
- 事件详情（属性列表、近期趋势迷你图）

### 3.4.2 事件属性管理页

功能：
- 属性列表（属性名、类型、说明）
- 创建/编辑属性定义
- 通用属性 vs 事件专属属性

### 3.4.3 事件分析页

交互设计（参考 Mixpanel Events）：
- 左侧：选择事件、添加筛选条件、选择指标（UV/PV/Sum/Avg）
- 右侧：时间序列折线图 + 数据表格
- 支持「按维度分组」（如按浏览器、城市、设备类型）
- 时间范围选择器（最近7天/30天/自定义）

---

## 3.5 埋点预聚合统计

### 3.5.1 预聚合表

```sql
-- 事件按小时预聚合（加速事件分析查询）
CREATE TABLE track_event_stats_hourly (
    id              SERIAL PRIMARY KEY,
    project_id      INTEGER NOT NULL REFERENCES projects(id),
    event           VARCHAR(128) NOT NULL,
    hour            TIMESTAMPTZ NOT NULL,
    total_count     INTEGER DEFAULT 0,
    unique_users    INTEGER DEFAULT 0,
    properties_summary JSONB,
    UNIQUE(project_id, event, hour)
);
```

### 3.5.2 聚合任务

- 每分钟执行一次预聚合
- 从 `track_events` 读取最近一小时数据
- 按 `project_id + event + hour` 聚合
- 更新 `track_event_stats_hourly`

---

## 3.6 本阶段验收标准

- [ ] 仪表盘能显示错误趋势、浏览器分布、设备分布等图表
- [ ] SSE 实时推送能正常工作，新错误实时显示
- [ ] SDK 全埋点能自动采集 $page_view / $element_click / $page_leave
- [ ] 埋点管理平台能创建/管理事件定义
- [ ] 事件分析页能查询事件数据并显示折线图
- [ ] 预聚合统计表能正常更新
