/**
 * 超级属性管理（全局属性，每次事件自动附加）
 *
 * 对标 Mixpanel.register / 神策 registerSuperProperties。
 */

import type { Properties } from '../types';
import { safeStorage } from './utils';

const STORAGE_KEY = '__monitor_super_props';

export class SuperProperties {
  private props: Properties = {};

  constructor() {
    const raw = safeStorage.get(STORAGE_KEY);
    if (raw) {
      try {
        this.props = JSON.parse(raw) as Properties;
      } catch {
        this.props = {};
      }
    }
  }

  register(properties: Properties): void {
    Object.assign(this.props, properties);
    this.persist();
  }

  unregister(propertyName: string): void {
    delete this.props[propertyName];
    this.persist();
  }

  clear(): void {
    this.props = {};
    safeStorage.remove(STORAGE_KEY);
  }

  getAll(): Properties {
    return { ...this.props };
  }

  private persist(): void {
    try {
      safeStorage.set(STORAGE_KEY, JSON.stringify(this.props));
    } catch {
      /* ignore */
    }
  }
}
