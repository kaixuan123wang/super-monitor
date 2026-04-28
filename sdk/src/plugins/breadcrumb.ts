/**
 * 面包屑（操作路径）采集：点击 / 路由跳转
 *
 * 点击事件通过 capture 阶段监听，记录目标元素的 selector 简描述。
 * 路由跳转通过 hashchange / popstate / pushState/replaceState hook。
 * Console / Network 面包屑分别由对应插件写入同一个 buffer。
 */

import type { BreadcrumbItem, SanitizeConfig } from '../types';
import { BreadcrumbBuffer } from '../core/breadcrumb-buffer';
import { sanitizeUrl } from '../core/utils';

export interface BreadcrumbPluginOptions {
  buffer: BreadcrumbBuffer;
  sanitize?: SanitizeConfig;
}

function describeElement(el: Element | null): string {
  if (!el) return '';
  const tag = el.tagName.toLowerCase();
  const id = el.id ? `#${el.id}` : '';
  const cls =
    el.className && typeof el.className === 'string'
      ? '.' + el.className.trim().split(/\s+/).slice(0, 2).join('.')
      : '';
  const text = (el as HTMLElement).innerText?.trim().slice(0, 30) || '';
  return `${tag}${id}${cls}${text ? `("${text}")` : ''}`;
}

export function installBreadcrumbPlugin(options: BreadcrumbPluginOptions): () => void {
  if (typeof window === 'undefined') {
    return () => {
      /* noop */
    };
  }

  const { buffer, sanitize } = options;

  // 点击
  const clickHandler = (e: MouseEvent): void => {
    const target = e.target as Element | null;
    if (!target) return;
    buffer.push({
      category: 'click',
      message: describeElement(target),
    });
  };
  document.addEventListener('click', clickHandler, true);

  // 路由跳转 (hashchange / popstate)
  const onHash = (): void => {
    const url = sanitizeUrl(location.href, sanitize?.sensitiveQueryKeys);
    buffer.push({
      category: 'navigation',
      message: `hashchange -> ${url}`,
      data: { url },
    });
  };
  const onPop = (): void => {
    const url = sanitizeUrl(location.href, sanitize?.sensitiveQueryKeys);
    buffer.push({
      category: 'navigation',
      message: `popstate -> ${url}`,
      data: { url },
    });
  };
  window.addEventListener('hashchange', onHash);
  window.addEventListener('popstate', onPop);

  // pushState / replaceState — 保存当前引用以便链式调用
  const prevPush = history.pushState;
  const prevReplace = history.replaceState;
  history.pushState = function (
    data: unknown,
    unused: string,
    url?: string | URL | null
  ): void {
    const nextUrl = sanitizeUrl(String(url ?? location.href), sanitize?.sensitiveQueryKeys);
    buffer.push({
      category: 'navigation',
      message: `pushState -> ${nextUrl}`,
    });
    return prevPush.apply(this, [data, unused, url] as unknown as Parameters<typeof prevPush>);
  };
  history.replaceState = function (
    data: unknown,
    unused: string,
    url?: string | URL | null
  ): void {
    const nextUrl = sanitizeUrl(String(url ?? location.href), sanitize?.sensitiveQueryKeys);
    buffer.push({
      category: 'navigation',
      message: `replaceState -> ${nextUrl}`,
    });
    return prevReplace.apply(this, [data, unused, url] as unknown as Parameters<typeof prevReplace>);
  };

  return () => {
    document.removeEventListener('click', clickHandler, true);
    window.removeEventListener('hashchange', onHash);
    window.removeEventListener('popstate', onPop);
    history.pushState = prevPush;
    history.replaceState = prevReplace;
  };
}

export type { BreadcrumbItem };
