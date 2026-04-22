/**
 * 数据上报模块
 *
 * Phase 1: 提供最小可用的 POST 上报能力 + 批量 flush。
 * 后续阶段扩展：失败重试、sendBeacon、gzip、脱敏等。
 */

import type { CollectPayload, MonitorConfig } from '../types';
import { Store } from './store';

export interface ReporterOptions {
  server: string;
  appId: string;
  appKey: string;
  flushInterval: number;
  store: Store;
  debug?: boolean;
}

export class Reporter {
  private readonly endpoint: string;
  private readonly headers: Record<string, string>;
  private readonly flushInterval: number;
  private readonly store: Store;
  private timer: ReturnType<typeof setInterval> | null = null;
  private readonly debug: boolean;

  constructor(options: ReporterOptions) {
    const base = options.server.replace(/\/$/, '');
    this.endpoint = `${base}/api/v1/collect`;
    this.headers = {
      'Content-Type': 'application/json',
      'X-App-Id': options.appId,
      'X-App-Key': options.appKey,
    };
    this.flushInterval = options.flushInterval;
    this.store = options.store;
    this.debug = !!options.debug;
  }

  /** 启动定时批量上报 */
  start(): void {
    if (this.timer) return;
    this.timer = setInterval(() => this.flush(), this.flushInterval);

    if (typeof window !== 'undefined') {
      window.addEventListener('beforeunload', () => this.flushSync());
      window.addEventListener('pagehide', () => this.flushSync());
    }
  }

  /** 停止定时器 */
  stop(): void {
    if (this.timer) {
      clearInterval(this.timer);
      this.timer = null;
    }
  }

  /** 加入队列 */
  report(payload: CollectPayload): void {
    this.store.enqueue(payload);
  }

  /** 立即上报（通过 fetch） */
  async flush(): Promise<void> {
    const items = this.store.drain();
    if (items.length === 0) return;
    try {
      await fetch(this.endpoint, {
        method: 'POST',
        headers: this.headers,
        body: JSON.stringify(items.length === 1 ? items[0] : { type: 'batch', data: items }),
        keepalive: true,
      });
    } catch (e) {
      if (this.debug && typeof console !== 'undefined') {
        console.warn('[Monitor] flush failed', e);
      }
    }
  }

  /** 页面关闭时同步上报（sendBeacon 降级） */
  flushSync(): void {
    const items = this.store.drain();
    if (items.length === 0) return;
    const body = JSON.stringify(items.length === 1 ? items[0] : { type: 'batch', data: items });
    try {
      if (typeof navigator !== 'undefined' && typeof navigator.sendBeacon === 'function') {
        const blob = new Blob([body], { type: 'application/json' });
        navigator.sendBeacon(this.endpoint, blob);
        return;
      }
    } catch {
      /* ignore */
    }
    // 降级：同步 XHR
    try {
      const xhr = new XMLHttpRequest();
      xhr.open('POST', this.endpoint, false);
      xhr.setRequestHeader('Content-Type', 'application/json');
      xhr.setRequestHeader('X-App-Id', this.headers['X-App-Id']);
      xhr.setRequestHeader('X-App-Key', this.headers['X-App-Key']);
      xhr.send(body);
    } catch {
      /* ignore */
    }
  }
}

export function buildReporterFromConfig(config: MonitorConfig, store: Store): Reporter {
  return new Reporter({
    server: config.server,
    appId: config.appId,
    appKey: config.appKey,
    flushInterval: config.reporter?.flushInterval ?? 5000,
    store,
    debug: config.debug,
  });
}
