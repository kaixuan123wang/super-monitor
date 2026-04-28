/**
 * 元素曝光追踪（IntersectionObserver + data-track-imp）
 * Phase 5 实现。
 */

import type { SanitizeConfig } from '../types';
import { sanitizeUrl } from '../core/utils';

export interface ExposureOptions {
  track: (event: string, properties: Record<string, unknown>) => void;
  threshold?: number;
  sanitize?: SanitizeConfig;
}

const DEFAULT_EVENT = '$element_exposure';
const DEFAULT_THRESHOLD = 0.5;

export function installExposurePlugin(options: ExposureOptions): () => void {
  if (
    typeof window === 'undefined' ||
    typeof document === 'undefined' ||
    typeof IntersectionObserver === 'undefined'
  ) {
    return () => undefined;
  }

  const { track, threshold = DEFAULT_THRESHOLD, sanitize } = options;
  const observed = new WeakSet<Element>();
  const triggered = new WeakSet<Element>();
  const visible = new WeakSet<Element>();

  const observer = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        const el = entry.target;
        const mode = (el.getAttribute('data-track-mode') || 'once').toLowerCase();
        const shouldTrack = entry.isIntersecting;

        if (!shouldTrack) {
          visible.delete(el);
          continue;
        }

        if (mode !== 'always' && triggered.has(el)) {
          observer.unobserve(el);
          continue;
        }

        if (mode === 'always' && visible.has(el)) {
          continue;
        }

        visible.add(el);
        triggered.add(el);
        fireExposure(track, el, entry.intersectionRatio, sanitize);

        if (mode !== 'always') {
          observer.unobserve(el);
        }
      }
    },
    { threshold: [threshold] }
  );

  const scan = (root: ParentNode): void => {
    const nodes: Element[] = [];
    if (root instanceof Element && isExposureElement(root)) {
      nodes.push(root);
    }
    root.querySelectorAll?.('[data-track-imp="true"]').forEach((el) => nodes.push(el));

    for (const el of nodes) {
      if (observed.has(el)) continue;
      observed.add(el);
      observer.observe(el);
    }
  };

  const mutationObserver =
    typeof MutationObserver !== 'undefined'
      ? new MutationObserver((mutations) => {
          for (const mutation of mutations) {
            mutation.addedNodes.forEach((node) => {
              if (node.nodeType === Node.ELEMENT_NODE) {
                scan(node as Element);
              }
            });
          }
        })
      : null;

  const handleReady = () => scan(document.documentElement);

  scan(document.documentElement);
  document.addEventListener('DOMContentLoaded', handleReady);
  mutationObserver?.observe(document.documentElement, {
    childList: true,
    subtree: true,
  });

  return () => {
    observer.disconnect();
    mutationObserver?.disconnect();
    document.removeEventListener('DOMContentLoaded', handleReady);
  };
}

function isExposureElement(el: Element): boolean {
  return el.getAttribute('data-track-imp') === 'true';
}

function fireExposure(
  track: ExposureOptions['track'],
  el: Element,
  exposureRatio: number,
  sanitize?: SanitizeConfig
): void {
  const event = el.getAttribute('data-track-event') || DEFAULT_EVENT;
  const customAttrs = parseAttrs(el.getAttribute('data-track-attrs'));

  track(event, {
    ...customAttrs,
    $element_id: el.id || undefined,
    $element_class: getClassName(el) || undefined,
    $element_type: (el as HTMLInputElement).type || el.tagName.toLowerCase(),
    $element_content: getTextContent(el),
    $element_path: getElementPath(el),
    $page_url: sanitizeUrl(location.href, sanitize?.sensitiveQueryKeys),
    $viewport_width: window.innerWidth,
    $viewport_height: window.innerHeight,
    $exposure_ratio: Math.round(exposureRatio * 100) / 100,
  });
}

function parseAttrs(raw: string | null): Record<string, unknown> {
  if (!raw) return {};
  try {
    const parsed = JSON.parse(raw);
    return parsed && typeof parsed === 'object' && !Array.isArray(parsed) ? parsed : {};
  } catch {
    return {};
  }
}

function getClassName(el: Element): string {
  const className = el.className;
  if (typeof className === 'string') return className;
  return el.getAttribute('class') || '';
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
    if (cur.id) {
      part += `#${cur.id}`;
    } else {
      const firstClass = getClassName(cur).split(/\s+/).filter(Boolean)[0];
      if (firstClass) part += `.${firstClass}`;
    }
    parts.unshift(part);
    cur = cur.parentElement;
  }
  return parts.join(' > ');
}
