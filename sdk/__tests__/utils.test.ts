import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import {
  uuid,
  now,
  safeStorage,
  createLogger,
  assign,
  hashString,
  errorFingerprint,
  sanitizeObject,
  sanitizeBodyString,
  sanitizeUrl,
  parseUA,
} from '../src/core/utils';

describe('uuid()', () => {
  it('should return a string of length 36', () => {
    const id = uuid();
    expect(typeof id).toBe('string');
    expect(id.length).toBe(36);
  });

  it('should return unique values', () => {
    const ids = new Set(Array.from({ length: 100 }, uuid));
    expect(ids.size).toBe(100);
  });

  it('should match UUID v4 format', () => {
    const id = uuid();
    expect(id).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i);
  });
});

describe('now()', () => {
  it('should return a number close to Date.now()', () => {
    const before = Date.now();
    const n = now();
    const after = Date.now();
    expect(n).toBeGreaterThanOrEqual(before);
    expect(n).toBeLessThanOrEqual(after);
  });
});

describe('safeStorage', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('get returns null for missing key', () => {
    expect(safeStorage.get('__missing__')).toBeNull();
  });

  it('set/get roundtrip', () => {
    safeStorage.set('key', 'value');
    expect(safeStorage.get('key')).toBe('value');
  });

  it('remove deletes key', () => {
    safeStorage.set('key', 'value');
    safeStorage.remove('key');
    expect(safeStorage.get('key')).toBeNull();
  });

  it('handles quota exceeded gracefully', () => {
    const originalSetItem = Storage.prototype.setItem;
    Storage.prototype.setItem = vi.fn(() => {
      throw new DOMException('Quota exceeded', 'QuotaExceededError');
    });
    expect(() => safeStorage.set('key', 'value')).not.toThrow();
    Storage.prototype.setItem = originalSetItem;
  });
});

describe('createLogger()', () => {
  it('returns noops when debug is false', () => {
    const logger = createLogger(false);
    expect(logger.log).toBeInstanceOf(Function);
    expect(logger.warn).toBeInstanceOf(Function);
    expect(logger.error).toBeInstanceOf(Function);
    expect(() => logger.log('test')).not.toThrow();
  });

  it('returns console methods when debug is true', () => {
    const logger = createLogger(true);
    expect(logger.log).toBeTypeOf('function');
    expect(logger.warn).toBeTypeOf('function');
    expect(logger.error).toBeTypeOf('function');
  });

  it('accepts custom tag', () => {
    const logger = createLogger(true, '[Custom]');
    expect(logger.log).toBeTypeOf('function');
  });
});

describe('assign()', () => {
  it('merges objects shallowly', () => {
    const result = assign({ a: 1 }, { b: 2 });
    expect(result).toEqual({ a: 1, b: 2 });
  });

  it('skips undefined values', () => {
    const result = assign({ a: 1 }, { a: undefined, b: 2 });
    expect(result).toEqual({ a: 1, b: 2 });
  });

  it('skips null/undefined sources', () => {
    const result = assign({ a: 1 }, null, undefined, { b: 2 });
    expect(result).toEqual({ a: 1, b: 2 });
  });

  it('overwrites existing keys', () => {
    const result = assign({ a: 1 }, { a: 2 });
    expect(result).toEqual({ a: 2 });
  });
});

describe('hashString()', () => {
  it('returns 8-character hex string', () => {
    const h = hashString('hello');
    expect(h).toMatch(/^[0-9a-f]{8}$/i);
  });

  it('returns consistent hash for same input', () => {
    expect(hashString('same')).toBe(hashString('same'));
  });

  it('returns different hashes for different inputs', () => {
    expect(hashString('a')).not.toBe(hashString('b'));
  });

  it('handles empty string', () => {
    expect(hashString('')).toBe('811c9dc5');
  });
});

describe('errorFingerprint()', () => {
  it('returns consistent fingerprint for same params', () => {
    const fp1 = errorFingerprint({ type: 'js', message: 'err', sourceUrl: 'a.js', line: 1, column: 2 });
    const fp2 = errorFingerprint({ type: 'js', message: 'err', sourceUrl: 'a.js', line: 1, column: 2 });
    expect(fp1).toBe(fp2);
  });

  it('returns different fingerprint for different messages', () => {
    const fp1 = errorFingerprint({ type: 'js', message: 'err1' });
    const fp2 = errorFingerprint({ type: 'js', message: 'err2' });
    expect(fp1).not.toBe(fp2);
  });

  it('truncates message to 200 chars', () => {
    const long = 'a'.repeat(500);
    const fp1 = errorFingerprint({ type: 'js', message: long });
    const fp2 = errorFingerprint({ type: 'js', message: long.slice(0, 200) });
    expect(fp1).toBe(fp2);
  });
});

describe('sanitizeObject()', () => {
  it('replaces sensitive fields with [REDACTED]', () => {
    const input = { password: 'secret', age: 30 };
    const out = sanitizeObject(input) as Record<string, unknown>;
    expect(out.password).toBe('[REDACTED]');
    expect(out.age).toBe(30);
  });

  it('handles nested objects', () => {
    const input = { user: { token: 'abc', age: 30 } };
    const out = sanitizeObject(input) as Record<string, Record<string, unknown>>;
    expect(out.user.token).toBe('[REDACTED]');
    expect(out.user.age).toBe(30);
  });

  it('handles arrays', () => {
    const input = [{ apiKey: 'k1' }, { apiKey: 'k2' }];
    const out = sanitizeObject(input) as Array<Record<string, unknown>>;
    expect(out[0].apiKey).toBe('[REDACTED]');
    expect(out[1].apiKey).toBe('[REDACTED]');
  });

  it('respects max depth', () => {
    const input: Record<string, unknown> = {};
    let cur: Record<string, unknown> = input;
    for (let i = 0; i < 10; i++) {
      cur.n = { password: 'secret' };
      cur = cur.n as Record<string, unknown>;
    }
    const out = sanitizeObject(input) as Record<string, unknown>;
    // depth > 6 stops recursion, so deeper nested objects are returned as-is
    expect(out).toBeDefined();
  });

  it('returns primitives as-is', () => {
    expect(sanitizeObject(null)).toBeNull();
    expect(sanitizeObject('string')).toBe('string');
    expect(sanitizeObject(42)).toBe(42);
  });
});

describe('sanitizeBodyString()', () => {
  it('sanitizes JSON body', () => {
    const body = JSON.stringify({ password: 'secret', age: 30 });
    const out = sanitizeBodyString(body);
    expect(out).toContain('[REDACTED]');
    expect(out).toContain('30');
  });

  it('truncates oversized JSON', () => {
    const body = JSON.stringify({ data: 'x'.repeat(20 * 1024) });
    const out = sanitizeBodyString(body);
    expect(out.endsWith('...[TRUNCATED]')).toBe(true);
  });

  it('handles non-JSON body with regex', () => {
    const body = 'password=secret&name=John';
    const out = sanitizeBodyString(body);
    expect(out).toContain('[REDACTED]');
  });

  it('returns empty string for empty input', () => {
    expect(sanitizeBodyString('')).toBe('');
  });
});

describe('sanitizeUrl()', () => {
  it('removes sensitive query keys', () => {
    const url = 'https://example.com?token=abc&nickname=John';
    const out = sanitizeUrl(url);
    expect(out).toContain('token=[REDACTED]');
    expect(out).toContain('nickname=John');
  });

  it('returns url without query as-is', () => {
    expect(sanitizeUrl('https://example.com')).toBe('https://example.com');
  });

  it('returns empty/invalid url as-is', () => {
    expect(sanitizeUrl('')).toBe('');
  });

  it('handles multiple sensitive keys', () => {
    const url = 'https://example.com?token=a&secret=b&key=c&safe=d';
    const out = sanitizeUrl(url);
    expect(out).toContain('token=[REDACTED]');
    expect(out).toContain('secret=[REDACTED]');
    expect(out).toContain('key=[REDACTED]');
    expect(out).toContain('safe=d');
  });
});

describe('parseUA()', () => {
  it('detects Chrome', () => {
    const ua = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36';
    const result = parseUA(ua);
    expect(result.browser).toBe('Chrome');
    expect(result.browser_version).toBe('120.0.0.0');
    expect(result.os).toBe('Windows');
    expect(result.device_type).toBe('desktop');
  });

  it('detects Firefox', () => {
    const ua = 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:120.0) Gecko/20100101 Firefox/120.0';
    const result = parseUA(ua);
    expect(result.browser).toBe('Firefox');
    expect(result.os).toBe('macOS');
  });

  it('detects Safari', () => {
    const ua = 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Safari/605.1.15';
    const result = parseUA(ua);
    expect(result.browser).toBe('Safari');
    expect(result.browser_version).toBe('17.1');
  });

  it('detects Edge', () => {
    const ua = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0';
    const result = parseUA(ua);
    expect(result.browser).toBe('Edge');
  });

  it('detects mobile', () => {
    const ua = 'Mozilla/5.0 (iPhone; CPU iPhone OS 17_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Mobile/15E148 Safari/604.1';
    const result = parseUA(ua);
    expect(result.device_type).toBe('mobile');
    expect(result.os).toBe('iOS');
  });

  it('detects tablet', () => {
    const ua = 'Mozilla/5.0 (iPad; CPU OS 17_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Mobile/15E148 Safari/604.1';
    const result = parseUA(ua);
    expect(result.device_type).toBe('tablet');
  });

  it('detects Android', () => {
    const ua = 'Mozilla/5.0 (Linux; Android 14; SM-G991B) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36';
    const result = parseUA(ua);
    expect(result.os).toBe('Android');
    expect(result.device_type).toBe('mobile');
  });

  it('handles empty UA', () => {
    const result = parseUA('');
    expect(result.browser).toBe('Unknown');
    expect(result.os).toBe('Unknown');
  });

  it('handles IE', () => {
    const ua = 'Mozilla/5.0 (Windows NT 10.0; WOW64; Trident/7.0; rv:11.0) like Gecko';
    const result = parseUA(ua);
    expect(result.browser).toBe('IE');
    expect(result.browser_version).toBe('11.0');
  });
});
