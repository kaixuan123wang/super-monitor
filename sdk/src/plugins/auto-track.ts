/**
 * 全埋点插件：自动采集 $page_view / $element_click / $page_leave
 *
 * 调用方式：
 *   const cleanup = installAutoTrackPlugin({ track, config })
 *   // 卸载时调用 cleanup()
 */

import type { TrackingConfig } from '../types';

export interface AutoTrackOptions {
  /** 触发埋点事件的回调（由 Monitor 注入） */
  track: (event: string, properties: Record<string, unknown>, priority?: 'P0' | 'P1') => void;
  config?: TrackingConfig;
}

export function installAutoTrackPlugin(options: AutoTrackOptions): () => void {
  const { track, config } = options;
  const autoTrack = config?.autoTrack ?? { pageView: true, click: true, pageLeave: true };
  const cleanups: Array<() => void> = [];

  if (autoTrack.pageView !== false) {
    cleanups.push(installPageView(track));
  }
  if (autoTrack.click !== false) {
    cleanups.push(installElementClick(track));
  }
  if (autoTrack.pageLeave !== false) {
    cleanups.push(installPageLeave(track));
  }

  return () => cleanups.forEach((fn) => fn());
}

// ── $page_view ────────────────────────────────────────────────────────────────

function installPageView(track: AutoTrackOptions['track']): () => void {
  // 立即上报当前页面
  firePageView(track);

  // SPA：监听 popstate + hashchange；同时用 MutationObserver 检测 URL 变化
  let lastUrl = location.href;

  const handleNav = () => {
    if (location.href !== lastUrl) {
      lastUrl = location.href;
      firePageView(track);
    }
  };

  window.addEventListener('popstate', handleNav);
  window.addEventListener('hashchange', handleNav);

  // 拦截 history.pushState / replaceState
  const origPush = history.pushState.bind(history);
  const origReplace = history.replaceState.bind(history);

  history.pushState = function (...args) {
    origPush(...args);
    handleNav();
  };
  history.replaceState = function (...args) {
    origReplace(...args);
    handleNav();
  };

  return () => {
    window.removeEventListener('popstate', handleNav);
    window.removeEventListener('hashchange', handleNav);
    history.pushState = origPush;
    history.replaceState = origReplace;
  };
}

function firePageView(track: AutoTrackOptions['track']): void {
  let isFirstVisit = false;
  let isFirstDay = false;
  try {
    isFirstVisit = !sessionStorage.getItem('__monitor_visited');
    isFirstDay = !localStorage.getItem('__monitor_first_day');
    if (isFirstVisit) sessionStorage.setItem('__monitor_visited', '1');
    if (isFirstDay) localStorage.setItem('__monitor_first_day', new Date().toDateString());
  } catch {
    // 隐私模式或禁用 storage 时静默降级
  }

  track('$page_view', {
    $page_url: location.href,
    $page_title: document.title,
    $referrer: document.referrer || undefined,
    $viewport_width: window.innerWidth,
    $viewport_height: window.innerHeight,
    $is_first_visit: isFirstVisit,
    $is_first_day: isFirstDay,
  });
}

// ── $element_click ────────────────────────────────────────────────────────────

const INTERACTIVE_TAGS = new Set(['A', 'BUTTON', 'INPUT', 'SELECT', 'TEXTAREA', 'LABEL']);

function installElementClick(track: AutoTrackOptions['track']): () => void {
  const handler = (e: MouseEvent) => {
    const target = findInteractiveTarget(e.target as Element | null);
    if (!target) return;

    track('$element_click', {
      $element_id: target.id || undefined,
      $element_class: target.className || undefined,
      $element_type: (target as HTMLInputElement).type || target.tagName.toLowerCase(),
      $element_name: (target as HTMLElement).getAttribute('name') || undefined,
      $element_content: getTextContent(target),
      $element_path: getElementPath(target),
      $page_url: location.href,
      $page_x: e.pageX,
      $page_y: e.pageY,
    });
  };

  document.addEventListener('click', handler, true);
  return () => document.removeEventListener('click', handler, true);
}

function findInteractiveTarget(el: Element | null): Element | null {
  let cur: Element | null = el;
  for (let i = 0; i < 5 && cur && cur !== document.body; i++) {
    if (
      INTERACTIVE_TAGS.has(cur.tagName) ||
      cur.hasAttribute('onclick') ||
      cur.getAttribute('role') === 'button' ||
      (cur as HTMLElement).style?.cursor === 'pointer'
    ) {
      return cur;
    }
    cur = cur.parentElement;
  }
  return el; // 兜底：返回原始目标
}

function getTextContent(el: Element): string {
  const text = (el as HTMLElement).innerText || el.textContent || '';
  return text.trim().slice(0, 50);
}

function getElementPath(el: Element): string {
  const parts: string[] = [];
  let cur: Element | null = el;
  for (let i = 0; i < 5 && cur && cur !== document.documentElement; i++) {
    let part = cur.tagName.toLowerCase();
    if (cur.id) part += `#${cur.id}`;
    else if (cur.className) part += `.${String(cur.className).split(' ')[0]}`;
    parts.unshift(part);
    cur = cur.parentElement;
  }
  return parts.join(' > ');
}

// ── $page_leave ───────────────────────────────────────────────────────────────

function installPageLeave(track: AutoTrackOptions['track']): () => void {
  const enterTime = Date.now();
  let fired = false;

  const fire = (reason: string) => {
    if (fired) return;
    fired = true;
    const duration = (Date.now() - enterTime) / 1000;
    track('$page_leave', {
      $page_url: location.href,
      $page_title: document.title,
      $stay_duration: Math.round(duration * 10) / 10,
      $leave_reason: reason,
    }, 'P0');
  };

  const handleBeforeUnload = () => fire('unload');
  const handlePageHide = () => fire('pagehide');

  window.addEventListener('beforeunload', handleBeforeUnload);
  window.addEventListener('pagehide', handlePageHide);

  return () => {
    window.removeEventListener('beforeunload', handleBeforeUnload);
    window.removeEventListener('pagehide', handlePageHide);
  };
}
