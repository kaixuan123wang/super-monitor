/**
 * 性能指标采集
 *
 * 指标：FP / FCP / LCP / CLS / TTFB / FID + 页面加载时间、DNS 时间等
 *
 * 策略：
 * - 采样上报（默认 10%）
 * - LCP 在页面隐藏（pagehide/visibilitychange hidden）时取最终值
 * - CLS 累加 layout-shift 直到页面隐藏
 */

import type { CollectPayload, PerformanceData } from '../types';

export interface PerformancePluginOptions {
  report: (payload: CollectPayload<PerformanceData>) => void;
  sampleRate?: number;
}

interface LayoutShiftEntry extends PerformanceEntry {
  value: number;
  hadRecentInput: boolean;
}

interface FirstInputEntry extends PerformanceEntry {
  processingStart: number;
  startTime: number;
}

export function installPerformancePlugin(options: PerformancePluginOptions): () => void {
  if (typeof window === 'undefined' || typeof performance === 'undefined') {
    return () => {
      /* noop */
    };
  }
  const sampleRate = options.sampleRate ?? 0.1;
  if (Math.random() > sampleRate) {
    return () => {
      /* not sampled */
    };
  }

  const data: PerformanceData = { url: window.location?.href };
  const observers: PerformanceObserver[] = [];
  let reported = false;

  // 1. Navigation timing → TTFB / 页面加载时间 / DNS / TCP / SSL / DOM parse
  const readNavigationTiming = (): void => {
    try {
      const [nav] = performance.getEntriesByType('navigation') as PerformanceNavigationTiming[];
      if (!nav) return;
      data.ttfb = Math.round(nav.responseStart - nav.startTime);
      data.load_time = Math.round(nav.loadEventEnd - nav.startTime);
      data.dns_time = Math.round(nav.domainLookupEnd - nav.domainLookupStart);
      data.tcp_time = Math.round(nav.connectEnd - nav.connectStart);
      data.ssl_time =
        nav.secureConnectionStart > 0 ? Math.round(nav.connectEnd - nav.secureConnectionStart) : 0;
      data.dom_parse_time = Math.round(nav.domContentLoadedEventEnd - nav.domInteractive);
    } catch {
      /* ignore */
    }
  };

  // 2. Paint timing → FP / FCP
  const readPaint = (): void => {
    try {
      const entries = performance.getEntriesByType('paint');
      for (const entry of entries) {
        if (entry.name === 'first-paint') data.fp = Math.round(entry.startTime);
        if (entry.name === 'first-contentful-paint') data.fcp = Math.round(entry.startTime);
      }
    } catch {
      /* ignore */
    }
  };

  // 3. Resource count / size
  const readResource = (): void => {
    try {
      const list = performance.getEntriesByType('resource') as PerformanceResourceTiming[];
      data.resource_count = list.length;
      data.resource_size = list.reduce((sum, r) => sum + (r.transferSize || 0), 0);
    } catch {
      /* ignore */
    }
  };

  // 4. LCP
  try {
    const po = new PerformanceObserver((list) => {
      const entries = list.getEntries();
      const last = entries[entries.length - 1];
      if (last) data.lcp = Math.round(last.startTime);
    });
    po.observe({ type: 'largest-contentful-paint', buffered: true });
    observers.push(po);
  } catch {
    /* LCP not supported */
  }

  // 5. CLS
  let clsValue = 0;
  try {
    const po = new PerformanceObserver((list) => {
      for (const entry of list.getEntries() as LayoutShiftEntry[]) {
        if (!entry.hadRecentInput) {
          clsValue += entry.value;
        }
      }
      data.cls = Math.round(clsValue * 10000) / 10000;
    });
    po.observe({ type: 'layout-shift', buffered: true });
    observers.push(po);
  } catch {
    /* CLS not supported */
  }

  // 6. FID
  try {
    const po = new PerformanceObserver((list) => {
      const entries = list.getEntries() as FirstInputEntry[];
      if (entries[0]) {
        data.fid = Math.round(entries[0].processingStart - entries[0].startTime);
      }
    });
    po.observe({ type: 'first-input', buffered: true });
    observers.push(po);
  } catch {
    /* FID not supported */
  }

  const finalize = (): void => {
    if (reported) return;
    reported = true;
    readNavigationTiming();
    readPaint();
    readResource();
    for (const ob of observers) {
      try {
        ob.disconnect();
      } catch {
        /* ignore */
      }
    }
    options.report({ type: 'performance', data, priority: 'P1' });
  };

  // 页面加载完成后延迟 1s 上报一次基础指标
  const onLoad = (): void => {
    setTimeout(finalize, 1000);
  };

  if (document.readyState === 'complete') {
    onLoad();
  } else {
    window.addEventListener('load', onLoad, { once: true });
  }

  // 页面隐藏时强制上报（避免错过 LCP/CLS 终值）
  const onHide = (): void => {
    if (document.visibilityState === 'hidden') {
      finalize();
    }
  };
  document.addEventListener('visibilitychange', onHide);
  window.addEventListener('pagehide', finalize);

  return () => {
    window.removeEventListener('load', onLoad);
    document.removeEventListener('visibilitychange', onHide);
    window.removeEventListener('pagehide', finalize);
    for (const ob of observers) {
      try {
        ob.disconnect();
      } catch {
        /* ignore */
      }
    }
  };
}
