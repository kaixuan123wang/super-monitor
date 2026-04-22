/**
 * SDK 入口文件
 *
 * 使用方式：
 *   import Monitor from '@js-monitor/sdk';
 *   Monitor.init({ appId, appKey, server });
 *
 * 或浏览器直接引入 IIFE 构建产物，全局变量 `Monitor`：
 *   <script src="/sdk.iife.js"></script>
 *   <script>Monitor.init({ ... })</script>
 */

import { Monitor, SDK_VERSION } from './core/monitor';

export { Monitor, SDK_VERSION };
export * from './types';

export default Monitor;
