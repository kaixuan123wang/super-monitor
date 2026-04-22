# @js-monitor/sdk

零依赖的浏览器端 JS 监控 / 埋点 SDK。

## 构建

```bash
pnpm install    # 或 npm i / yarn
pnpm build      # 输出 build/sdk.umd.js / sdk.esm.js / sdk.iife.js
```

## 使用

```ts
import Monitor from '@js-monitor/sdk';

Monitor.init({
  appId: 'your-app-id',
  appKey: 'your-app-key',
  server: 'https://monitor.example.com',
  environment: 'production',
  release: 'git-commit-hash',
});

Monitor.track('purchase', { product_id: 'sku_001', price: 99.9 });
Monitor.identify('user_123');
Monitor.setUserProperties({ $name: '张三', membership: 'premium' });
```

> Phase 1 仅搭建骨架：核心初始化 / 上报 / 身份 / 超级属性 / 计时器。
> Phase 2 起接入 error / network / performance / breadcrumb / auto-track 等插件。

## 目录

```
src/
├── core/                # 核心模块
│   ├── monitor.ts       # SDK 单例
│   ├── reporter.ts      # 数据上报
│   ├── store.ts         # 队列
│   ├── identity.ts      # 用户身份
│   ├── super-props.ts   # 超级属性
│   ├── timer.ts         # 事件时长
│   └── utils.ts
├── plugins/             # 监控/埋点插件（Phase 2+）
├── types/
└── index.ts
```
