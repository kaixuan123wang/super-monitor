/**
 * fetch / XHR 接口监控
 *
 * 行为：
 * - 所有请求都推入面包屑（保留最近 N 条用于错误关联）
 * - 接口错误（status >= 400 或网络失败）实时（P0）上报为 `network` 类型
 * - 成功请求不单独上报，仅作为面包屑
 *
 * 脱敏：
 * - URL 的 query 中 token/auth/key/secret 被替换
 * - 请求/响应 body 中的敏感字段被替换为 [REDACTED]
 * - 响应体截断至 10KB
 */

import type { CollectPayload, NetworkData, SanitizeConfig } from '../types';
import { now, sanitizeBodyString, sanitizeUrl } from '../core/utils';
import { BreadcrumbBuffer } from '../core/breadcrumb-buffer';

const SDK_ENDPOINT_MARK = '/api/v1/collect';

export interface NetworkPluginOptions {
  report: (payload: CollectPayload<NetworkData>) => void;
  breadcrumb?: BreadcrumbBuffer;
  sanitize?: SanitizeConfig;
  /** 忽略的 URL 子串（默认自动忽略 SDK 自身上报地址） */
  ignoreUrls?: string[];
}

export function installNetworkPlugin(options: NetworkPluginOptions): () => void {
  if (typeof window === 'undefined') {
    return () => {
      /* noop */
    };
  }

  const maxBody = options.sanitize?.maxBodySize ?? 10 * 1024;
  const sensitiveFields = options.sanitize?.sensitiveFields;
  const sensitiveQueryKeys = options.sanitize?.sensitiveQueryKeys;
  const ignoreUrls = options.ignoreUrls ?? [];

  const shouldIgnore = (url: string): boolean => {
    if (!url) return true;
    if (url.indexOf(SDK_ENDPOINT_MARK) !== -1) return true;
    return ignoreUrls.some((p) => url.indexOf(p) !== -1);
  };

  const track = (data: NetworkData): void => {
    options.breadcrumb?.push({
      category: 'xhr',
      message: `${data.method} ${data.url} → ${data.status}`,
      level: data.status >= 400 ? 'error' : 'info',
      data: { duration: data.duration, status: data.status },
    });

    // 只有错误才单独上报
    if (data.status === 0 || data.status >= 400) {
      options.report({ type: 'network', data, priority: 'P0' });
    }
  };

  // ========= fetch 劫持 =========
  const originalFetch = window.fetch?.bind(window);
  let fetchPatched = false;
  if (typeof originalFetch === 'function') {
    window.fetch = async (input: RequestInfo | URL, init?: RequestInit): Promise<Response> => {
      const method = (init?.method || (input instanceof Request ? input.method : 'GET') || 'GET').toUpperCase();
      const rawUrl =
        typeof input === 'string'
          ? input
          : input instanceof URL
          ? input.toString()
          : input.url;

      if (shouldIgnore(rawUrl)) {
        return originalFetch(input as RequestInfo, init);
      }

      const start = now();
      const sanitizedUrl = sanitizeUrl(rawUrl, sensitiveQueryKeys);

      let reqBody: string | undefined;
      if (init?.body && typeof init.body === 'string') {
        reqBody = sanitizeBodyString(init.body, sensitiveFields, maxBody);
      }

      try {
        const resp = await originalFetch(input as RequestInfo, init);
        const duration = now() - start;

        let responseText: string | undefined;
        if (resp.status >= 400) {
          try {
            // 克隆后读取，避免消费原始流
            responseText = await resp.clone().text();
            responseText = sanitizeBodyString(responseText, sensitiveFields, maxBody);
          } catch {
            /* ignore */
          }
        }

        track({
          url: sanitizedUrl,
          method,
          status: resp.status,
          duration,
          request_body: reqBody,
          response_text: responseText,
        });
        return resp;
      } catch (e) {
        const duration = now() - start;
        track({
          url: sanitizedUrl,
          method,
          status: 0,
          duration,
          request_body: reqBody,
          error_type: (e as Error)?.name || 'NetworkError',
        });
        throw e;
      }
    };
    fetchPatched = true;
  }

  // ========= XHR 劫持 =========
  interface MonitorXHR extends XMLHttpRequest {
    __monitor?: {
      method: string;
      url: string;
      start: number;
      body?: string;
    };
  }

  const originalOpen = XMLHttpRequest.prototype.open;
  const originalSend = XMLHttpRequest.prototype.send;

  XMLHttpRequest.prototype.open = function (
    this: MonitorXHR,
    method: string,
    url: string | URL,
    async?: boolean,
    username?: string | null,
    password?: string | null
  ): void {
    const urlStr = url.toString();
    this.__monitor = {
      method: (method || 'GET').toUpperCase(),
      url: urlStr,
      start: 0,
    };
    // 需要全部参数以兼容老浏览器
    return originalOpen.apply(this, [method, urlStr, async ?? true, username ?? null, password ?? null] as unknown as Parameters<typeof originalOpen>);
  };

  XMLHttpRequest.prototype.send = function (
    this: MonitorXHR,
    body?: Document | XMLHttpRequestBodyInit | null
  ): void {
    const info = this.__monitor;
    if (info && !shouldIgnore(info.url)) {
      info.start = now();
      if (typeof body === 'string') {
        info.body = sanitizeBodyString(body, sensitiveFields, maxBody);
      }

      const onLoadEnd = (): void => {
        this.removeEventListener('loadend', onLoadEnd);
        const duration = now() - info.start;
        const status = this.status;
        let responseText: string | undefined;
        if (status >= 400) {
          try {
            // 仅在 responseType 是默认或 text 时才能读取
            if (!this.responseType || this.responseType === 'text') {
              responseText = sanitizeBodyString(this.responseText || '', sensitiveFields, maxBody);
            }
          } catch {
            /* ignore */
          }
        }
        track({
          url: sanitizeUrl(info.url, sensitiveQueryKeys),
          method: info.method,
          status: status || 0,
          duration,
          request_body: info.body,
          response_text: responseText,
          error_type: status === 0 ? 'NetworkError' : undefined,
        });
      };

      this.addEventListener('loadend', onLoadEnd);
    }
    return originalSend.apply(this, [body as XMLHttpRequestBodyInit] as unknown as Parameters<typeof originalSend>);
  };

  return () => {
    if (fetchPatched && originalFetch) {
      window.fetch = originalFetch;
    }
    XMLHttpRequest.prototype.open = originalOpen;
    XMLHttpRequest.prototype.send = originalSend;
  };
}
