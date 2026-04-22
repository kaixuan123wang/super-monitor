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

/**
 * 简单字符串哈希（FNV-1a 32bit），返回 8 位十六进制
 * 用于错误指纹去重，不用于安全场景
 */
export function hashString(input: string): string {
  let h = 0x811c9dc5;
  for (let i = 0; i < input.length; i++) {
    h ^= input.charCodeAt(i);
    h = (h + ((h << 1) + (h << 4) + (h << 7) + (h << 8) + (h << 24))) >>> 0;
  }
  return ('00000000' + h.toString(16)).slice(-8);
}

/** 生成错误指纹 */
export function errorFingerprint(opts: {
  type: string;
  message: string;
  sourceUrl?: string;
  line?: number;
  column?: number;
}): string {
  const key = [
    opts.type,
    (opts.message || '').slice(0, 200),
    opts.sourceUrl || '',
    opts.line ?? '',
    opts.column ?? '',
  ].join(':');
  return hashString(key);
}

const DEFAULT_SENSITIVE_FIELDS = [
  'password',
  'passwd',
  'pwd',
  'token',
  'access_token',
  'refresh_token',
  'secret',
  'apikey',
  'api_key',
  'authorization',
  'auth',
];

const DEFAULT_SENSITIVE_QUERY_KEYS = ['token', 'auth', 'key', 'secret', 'access_token'];

const REDACTED = '[REDACTED]';

/** 递归替换对象中敏感字段的值 */
export function sanitizeObject(
  value: unknown,
  sensitive: string[] = DEFAULT_SENSITIVE_FIELDS,
  depth = 0
): unknown {
  if (depth > 6) return value;
  if (value === null || value === undefined) return value;
  if (Array.isArray(value)) {
    return value.map((v) => sanitizeObject(v, sensitive, depth + 1));
  }
  if (typeof value === 'object') {
    const out: Record<string, unknown> = {};
    for (const k of Object.keys(value as Record<string, unknown>)) {
      const lower = k.toLowerCase();
      if (sensitive.some((s) => lower.includes(s))) {
        out[k] = REDACTED;
      } else {
        out[k] = sanitizeObject((value as Record<string, unknown>)[k], sensitive, depth + 1);
      }
    }
    return out;
  }
  return value;
}

/** 脱敏字符串形式的 body（尝试 JSON.parse 后脱敏，失败则按关键字正则替换） */
export function sanitizeBodyString(
  body: string,
  sensitive: string[] = DEFAULT_SENSITIVE_FIELDS,
  maxSize = 10 * 1024
): string {
  if (!body) return body;
  let truncated = body.length > maxSize ? body.slice(0, maxSize) + '...[TRUNCATED]' : body;
  try {
    const parsed = JSON.parse(truncated.replace(/\.\.\.\[TRUNCATED\]$/, ''));
    return JSON.stringify(sanitizeObject(parsed, sensitive));
  } catch {
    // 非 JSON，使用正则粗糙替换
    for (const field of sensitive) {
      const reg = new RegExp(`(["']?${field}["']?\\s*[:=]\\s*["']?)[^"'&,\\s}]+`, 'gi');
      truncated = truncated.replace(reg, `$1${REDACTED}`);
    }
    return truncated;
  }
}

/** 从 URL 中去掉敏感 query key */
export function sanitizeUrl(
  url: string,
  sensitiveKeys: string[] = DEFAULT_SENSITIVE_QUERY_KEYS
): string {
  if (!url || url.indexOf('?') === -1) return url;
  try {
    const [base, query] = url.split('?');
    const params = query.split('&').map((pair) => {
      const [k, ...rest] = pair.split('=');
      const v = rest.join('=');
      if (sensitiveKeys.some((s) => k.toLowerCase() === s.toLowerCase())) {
        return `${k}=${REDACTED}`;
      }
      return v === undefined ? k : `${k}=${v}`;
    });
    return `${base}?${params.join('&')}`;
  } catch {
    return url;
  }
}

/** 轻量的 UA 解析（只解析 browser / os / device 大类） */
export function parseUA(ua: string): {
  browser: string;
  browser_version: string;
  os: string;
  os_version: string;
  device_type: 'mobile' | 'tablet' | 'desktop';
} {
  const u = ua || '';
  let browser = 'Unknown';
  let browserVersion = '';
  if (/Edg\/([\d.]+)/.test(u)) {
    browser = 'Edge';
    browserVersion = RegExp.$1;
  } else if (/Chrome\/([\d.]+)/.test(u) && !/OPR/.test(u)) {
    browser = 'Chrome';
    browserVersion = RegExp.$1;
  } else if (/Firefox\/([\d.]+)/.test(u)) {
    browser = 'Firefox';
    browserVersion = RegExp.$1;
  } else if (/Safari\/([\d.]+)/.test(u) && /Version\/([\d.]+)/.test(u)) {
    browser = 'Safari';
    browserVersion = RegExp.$1;
  } else if (/MSIE ([\d.]+)/.test(u) || /Trident.*rv:([\d.]+)/.test(u)) {
    browser = 'IE';
    browserVersion = RegExp.$1;
  }

  let os = 'Unknown';
  let osVersion = '';
  if (/Windows NT ([\d.]+)/.test(u)) {
    os = 'Windows';
    osVersion = RegExp.$1;
  } else if (/Mac OS X ([\d_\.]+)/.test(u)) {
    os = 'macOS';
    osVersion = RegExp.$1.replace(/_/g, '.');
  } else if (/Android ([\d.]+)/.test(u)) {
    os = 'Android';
    osVersion = RegExp.$1;
  } else if (/iPhone OS ([\d_]+)/.test(u) || /OS ([\d_]+) like Mac OS X/.test(u)) {
    os = 'iOS';
    osVersion = RegExp.$1.replace(/_/g, '.');
  } else if (/Linux/.test(u)) {
    os = 'Linux';
  }

  let deviceType: 'mobile' | 'tablet' | 'desktop' = 'desktop';
  if (/iPad|Tablet/i.test(u)) deviceType = 'tablet';
  else if (/Mobile|iPhone|Android/i.test(u)) deviceType = 'mobile';

  return {
    browser,
    browser_version: browserVersion,
    os,
    os_version: osVersion,
    device_type: deviceType,
  };
}
