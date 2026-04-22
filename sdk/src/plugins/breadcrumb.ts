/**
 * 面包屑（操作路径）采集：点击 / 路由跳转
 *
 * 点击事件通过 capture 阶段监听，记录目标元素的 selector 简描述。
 * 路由跳转通过 hashchange / popstate / pushState/replaceState hook。
 * Console / Network 面包屑分别由对应插件写入同一个 buffer。
 */

import type { BreadcrumbItem } from '../types';
import { BreadcrumbBuffer } from '../core/breadcrumb-buffer';

export interface BreadcrumbPluginOptions {
  buffer: BreadcrumbBuffer;
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

  const { buffer } = options;

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
    buffer.push({
      category: 'navigation',
      message: `hashchange → ${location.href}`,
      data: { url: location.href },
    });
  };
  const onPop = (): void => {
    buffer.push({
      category: 'navigation',
      message: `popstate → ${location.href}`,
      data: { url: location.href },
    });
  };
  window.addEventListener('hashchange', onHash);
  window.addEventListener('popstate', onPop);

  // pushState / replaceState
  const origPush = history.pushState;
  const origReplace = history.replaceState;
  history.pushState = function (
    data: unknown,
    unused: string,
    url?: string | URL | null
  ): void {
    buffer.push({
      category: 'navigation',
      message: `pushState → ${url ?? location.href}`,
    });
    return origPush.apply(this, [data, unused, url] as unknown as Parameters<typeof origPush>);
  };
  history.replaceState = function (
    data: unknown,
    unused: string,
    url?: string | URL | null
  ): void {
    buffer.push({
      category: 'navigation',
      message: `replaceState → ${url ?? location.href}`,
    });
    return origReplace.apply(this, [data, unused, url] as unknown as Parameters<typeof origReplace>);
  };

  return () => {
    document.removeEventListener('click', clickHandler, true);
    window.removeEventListener('hashchange', onHash);
    window.removeEventListener('popstate', onPop);
    history.pushState = origPush;
    history.replaceState = origReplace;
  };
}

export type { BreadcrumbItem };
