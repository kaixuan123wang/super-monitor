/**
 * 数据上报模块
 *
 * Phase 2：
 * - P0 实时上报（fetch keepalive）
 * - P1 批量上报（定时器 + 队列满触发）
 * - 失败重试（简单指数退避，最多 retryMaxCount 次）
 * - 通过 ReportContext 注入公共字段（url / ua / breadcrumb 等）
 * - beforeunload / pagehide 时 flushSync（fetch keepalive + 同步 XHR 降级）
 */

import type { CollectPayload, MonitorConfig, ReportContext } from '../types';
import { Store } from './store';

/** 检测当前浏览器是否支持 fetch keepalive（Chrome 64+ / Firefox 131+ / Safari 18.2+） */
const supportsFetchKeepalive = ((): boolean => {
  try {
    return typeof Request !== 'undefined' && 'keepalive' in new Request('about:blank');
  } catch {
    return false;
  }
})();

export interface ReporterOptions {
  server: string;
  appId: string;
  appKey: string;
  flushInterval: number;
  retryMaxCount: number;
  retryInterval: number;
  store: Store;
  debug?: boolean;
  getContext?: () => ReportContext;
}

export class Reporter {
  private readonly endpoint: string;
  private readonly headers: Record<string, string>;
  private readonly flushInterval: number;
  private readonly retryMaxCount: number;
  private readonly retryInterval: number;
  private readonly store: Store;
  private timer: ReturnType<typeof setInterval> | null = null;
  private readonly debug: boolean;
  private readonly getContext: () => ReportContext;

  constructor(options: ReporterOptions) {
    const base = options.server.replace(/\/$/, '');
    this.endpoint = `${base}/api/v1/collect`;
    this.headers = {
      'Content-Type': 'application/json',
      'X-App-Id': options.appId,
      'X-App-Key': options.appKey,
    };
    this.flushInterval = options.flushInterval;
    this.retryMaxCount = options.retryMaxCount;
    this.retryInterval = options.retryInterval;
    this.store = options.store;
    this.debug = !!options.debug;
    this.getContext = options.getContext || (() => ({}));
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

  /**
   * 统一的上报入口：
   * - P0：立即发送（不入队）
   * - P1：入队，等批量 flush
   */
  report(payload: CollectPayload): void {
    if (payload.priority === 'P0') {
      this.sendImmediate(payload);
      return;
    }
    this.store.enqueue(payload);
  }

  /** 立即上报（通过 fetch） */
  async flush(): Promise<void> {
    const items = this.store.drain();
    if (items.length === 0) return;
    await this.sendWithRetry(this.wrapBatch(items), 0);
  }

  /** 页面关闭时尽量同步上报 */
  flushSync(): void {
    const items = this.store.drain();
    if (items.length === 0) return;
    const body = JSON.stringify(this.wrapBatch(items));
    // 仅当浏览器真正支持 keepalive 时才用 fetch；否则同步 XHR 更可靠
    if (typeof fetch === 'function' && supportsFetchKeepalive) {
      try {
        fetch(this.endpoint, {
          method: 'POST',
          headers: this.headers,
          body,
          keepalive: true,
        });
        return;
      } catch {
        /* ignore */
      }
    }
    // 降级：同步 XHR（兼容 IE7+ 及所有不支持 fetch keepalive 的浏览器）
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

  /** 将若干条数据包装为最终上报的 payload */
  private wrapBatch(items: CollectPayload[]): CollectPayload | { type: 'batch'; data: CollectPayload[]; context: ReportContext } {
    const context = this.getContext();
    if (items.length === 1) {
      const item = items[0];
      return {
        type: item.type,
        data: Object.assign({ __context: context }, item.data as object),
      } as CollectPayload;
    }
    return { type: 'batch', data: items, context };
  }

  /** 立即发送单条（P0 实时） */
  private async sendImmediate(payload: CollectPayload): Promise<void> {
    await this.sendWithRetry(
      {
        type: payload.type,
        data: Object.assign({ __context: this.getContext() }, payload.data as object),
      } as CollectPayload,
      0
    );
  }

  /** 带重试的发送 */
  private async sendWithRetry(body: unknown, attempt: number): Promise<void> {
    try {
      const resp = await fetch(this.endpoint, {
        method: 'POST',
        headers: this.headers,
        body: JSON.stringify(body),
        keepalive: true,
      });
      if (!resp.ok && attempt < this.retryMaxCount) {
        await this.delay(this.retryInterval * (attempt + 1));
        return this.sendWithRetry(body, attempt + 1);
      }
    } catch (e) {
      if (this.debug && typeof console !== 'undefined') {
        console.warn('[Monitor] send failed', e);
      }
      if (attempt < this.retryMaxCount) {
        await this.delay(this.retryInterval * (attempt + 1));
        return this.sendWithRetry(body, attempt + 1);
      }
    }
  }

  private delay(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }
}

export function buildReporterFromConfig(
  config: MonitorConfig,
  store: Store,
  getContext?: () => ReportContext
): Reporter {
  return new Reporter({
    server: config.server,
    appId: config.appId,
    appKey: config.appKey,
    flushInterval: config.reporter?.flushInterval ?? 5000,
    retryMaxCount: config.reporter?.retryMaxCount ?? 3,
    retryInterval: config.reporter?.retryInterval ?? 30000,
    store,
    debug: config.debug,
    getContext,
  });
}
