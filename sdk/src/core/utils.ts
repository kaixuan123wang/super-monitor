/**
 * 通用工具函数
 */

/** 生成 UUID v4（降级实现，兼容老浏览器） */
export function uuid(): string {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return crypto.randomUUID();
  }
  // 降级：基于时间戳 + 随机数
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
    const r = (Math.random() * 16) | 0;
    const v = c === 'x' ? r : (r & 0x3) | 0x8;
    return v.toString(16);
  });
}

/** 当前时间戳（毫秒） */
export function now(): number {
  return Date.now();
}

/** 安全的 localStorage 读写（SSR / 隐私模式降级） */
export const safeStorage = {
  get(key: string): string | null {
    try {
      return typeof localStorage !== 'undefined' ? localStorage.getItem(key) : null;
    } catch {
      return null;
    }
  },
  set(key: string, value: string): void {
    try {
      if (typeof localStorage !== 'undefined') {
        localStorage.setItem(key, value);
      }
    } catch {
      /* ignore */
    }
  },
  remove(key: string): void {
    try {
      if (typeof localStorage !== 'undefined') {
        localStorage.removeItem(key);
      }
    } catch {
      /* ignore */
    }
  },
};

/** 简单日志打印（受 debug 开关控制） */
export function createLogger(debug: boolean, tag = '[Monitor]') {
  const noop = () => { /* noop */ };
  if (!debug || typeof console === 'undefined') {
    return { log: noop, warn: noop, error: noop };
  }
  return {
    log: (...args: unknown[]) => console.log(tag, ...args),
    warn: (...args: unknown[]) => console.warn(tag, ...args),
    error: (...args: unknown[]) => console.error(tag, ...args),
  };
}

/** 浅合并（跳过 undefined） */
export function assign<T extends object>(target: T, ...sources: Array<Partial<T> | undefined>): T {
  for (const src of sources) {
    if (!src) continue;
    for (const key in src) {
      if (Object.prototype.hasOwnProperty.call(src, key) && src[key] !== undefined) {
        (target as Record<string, unknown>)[key] = src[key] as unknown;
      }
    }
  }
  return target;
}
