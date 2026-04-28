/**
 * 控制台日志劫持：把 console.warn / error / log 写入面包屑
 *
 * 仅写面包屑，不单独上报。错误上报时会携带上下文。
 */

import { BreadcrumbBuffer } from '../core/breadcrumb-buffer';
import { sanitizeBodyString, sanitizeObject } from '../core/utils';
import type { SanitizeConfig } from '../types';

export interface ConsolePluginOptions {
  buffer: BreadcrumbBuffer;
  levels?: Array<'log' | 'info' | 'warn' | 'error' | 'debug'>;
  sanitize?: SanitizeConfig;
}

function safeStringify(args: unknown[], sanitize?: SanitizeConfig): string {
  const maxSize = sanitize?.maxBodySize ?? 10 * 1024;
  try {
    return args
      .map((a) => {
        if (typeof a === 'string') {
          return sanitizeBodyString(a, sanitize?.sensitiveFields, maxSize);
        }
        if (a instanceof Error) return a.message;
        return JSON.stringify(sanitizeObject(a, sanitize?.sensitiveFields));
      })
      .join(' ')
      .slice(0, 300);
  } catch {
    return '[unserializable]';
  }
}

export function installConsolePlugin(options: ConsolePluginOptions): () => void {
  if (typeof console === 'undefined') {
    return () => {
      /* noop */
    };
  }
  const levels = options.levels ?? ['warn', 'error'];
  const originals: Record<string, (...args: unknown[]) => void> = {};

  for (const level of levels) {
    const orig = (console as unknown as Record<string, (...args: unknown[]) => void>)[level];
    if (typeof orig !== 'function') continue;
    originals[level] = orig;
    (console as unknown as Record<string, (...args: unknown[]) => void>)[level] = (
      ...args: unknown[]
    ): void => {
      options.buffer.push({
        category: 'console',
        level:
          level === 'log' || level === 'info'
            ? 'info'
            : (level as 'warn' | 'error' | 'debug'),
        message: safeStringify(args, options.sanitize),
      });
      try {
        orig.apply(console, args);
      } catch {
        /* ignore */
      }
    };
  }

  return () => {
    for (const level of Object.keys(originals)) {
      (console as unknown as Record<string, (...args: unknown[]) => void>)[level] =
        originals[level];
    }
  };
}
