/**
 * 错误监控插件：JS 错误 / Promise 未捕获 / 资源加载失败
 *
 * - window.onerror → JS 运行时错误
 * - unhandledrejection → Promise 未捕获
 * - 捕获阶段监听 window error 事件 → 资源加载错误（img/script/link）
 *
 * 去重策略：1 分钟窗口内相同指纹最多上报 10 次；
 * P0 错误（SyntaxError / ReferenceError）实时上报，其他批量上报。
 */

import type { CollectPayload, ErrorData } from '../types';
import { errorFingerprint, now } from '../core/utils';

const DEDUP_WINDOW_MS = 60_000;
const DEDUP_MAX_COUNT = 10;

const P0_TYPES = ['SyntaxError', 'ReferenceError'];

interface DedupEntry {
  count: number;
  windowStart: number;
}

export interface ErrorPluginOptions {
  report: (payload: CollectPayload<ErrorData>) => void;
  onError?: (err: ErrorData) => void;
  debug?: boolean;
}

export function installErrorPlugin(options: ErrorPluginOptions): () => void {
  if (typeof window === 'undefined') {
    return () => {
      /* noop */
    };
  }

  const dedup = new Map<string, DedupEntry>();

  const shouldReport = (fingerprint: string): boolean => {
    const current = now();
    const entry = dedup.get(fingerprint);
    if (!entry) {
      dedup.set(fingerprint, { count: 1, windowStart: current });
      return true;
    }
    if (current - entry.windowStart > DEDUP_WINDOW_MS) {
      entry.count = 1;
      entry.windowStart = current;
      return true;
    }
    entry.count += 1;
    return entry.count <= DEDUP_MAX_COUNT;
  };

  const emit = (data: ErrorData): void => {
    const fingerprint =
      data.fingerprint ||
      errorFingerprint({
        type: data.type,
        message: data.message,
        sourceUrl: data.source_url,
        line: data.line,
        column: data.column,
      });
    data.fingerprint = fingerprint;
    if (!shouldReport(fingerprint)) return;
    const isP0 = P0_TYPES.some((t) => (data.message || '').indexOf(t) !== -1);
    options.report({ type: 'error', data, priority: isP0 ? 'P0' : 'P1' });
    options.onError?.(data);
  };

  const jsHandler = (event: ErrorEvent): void => {
    // 资源加载错误走 capture 阶段的 errorCaptureHandler；这里只处理 JS 运行时
    if (!(event.error instanceof Error) && !event.message) return;
    const err = event.error as Error | null;
    emit({
      type: 'js',
      message: event.message || (err && err.message) || 'Unknown error',
      stack: err?.stack,
      source_url: event.filename,
      line: event.lineno,
      column: event.colno,
    });
  };

  const rejectionHandler = (event: PromiseRejectionEvent): void => {
    const reason = event.reason;
    let message = 'Unhandled promise rejection';
    let stack: string | undefined;
    if (reason instanceof Error) {
      message = reason.message;
      stack = reason.stack;
    } else if (typeof reason === 'string') {
      message = reason;
    } else {
      try {
        message = JSON.stringify(reason);
      } catch {
        /* ignore */
      }
    }
    emit({ type: 'promise', message, stack });
  };

  const errorCaptureHandler = (event: Event): void => {
    const target = event.target as (HTMLImageElement | HTMLScriptElement | HTMLLinkElement) | null;
    if (!target || target === (window as unknown as EventTarget)) return;
    const tagName = (target.tagName || '').toLowerCase();
    if (tagName !== 'img' && tagName !== 'script' && tagName !== 'link') return;
    const src =
      (target as HTMLImageElement).src ||
      (target as HTMLLinkElement).href ||
      (target as HTMLScriptElement).src ||
      '';
    emit({
      type: 'resource',
      message: `Resource load failed: ${tagName} ${src}`,
      source_url: src,
      extra: { tagName },
    });
  };

  window.addEventListener('error', jsHandler);
  window.addEventListener('unhandledrejection', rejectionHandler);
  window.addEventListener('error', errorCaptureHandler, true);

  return () => {
    window.removeEventListener('error', jsHandler);
    window.removeEventListener('unhandledrejection', rejectionHandler);
    window.removeEventListener('error', errorCaptureHandler, true);
  };
}
