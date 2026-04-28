/**
 * 本地缓存/队列
 *
 * 支持内存 FIFO 队列 + localStorage 持久化（页面刷新不丢失）。
 * 超限时优先丢弃非 P0 的旧数据。
 */

import type { CollectPayload } from '../types';

export interface StoreOptions {
  maxQueueSize: number;
}

const STORAGE_KEY = '__monitor_queue__';

export class Store {
  private queue: CollectPayload[] = [];
  private readonly maxQueueSize: number;
  private readonly useStorage: boolean;

  constructor(options: StoreOptions) {
    this.maxQueueSize = options.maxQueueSize;
    this.useStorage = typeof localStorage !== 'undefined';
    if (this.useStorage) {
      this.queue = this.loadFromStorage();
    }
  }

  /** 入队；超限时丢弃最旧的一条（优先丢弃非 P0） */
  enqueue(payload: CollectPayload): void {
    if (this.queue.length >= this.maxQueueSize) {
      // 优先丢弃最旧的非 P0 数据
      const idx = this.queue.findIndex((p) => p.priority !== 'P0');
      if (idx >= 0) {
        this.queue.splice(idx, 1);
      } else {
        this.queue.shift();
      }
    }
    this.queue.push(payload);
    this.saveToStorage();
  }

  /** 取出全部并清空队列 */
  drain(): CollectPayload[] {
    const items = this.queue;
    this.queue = [];
    this.saveToStorage();
    return items;
  }

  /** 当前队列长度 */
  size(): number {
    return this.queue.length;
  }

  private loadFromStorage(): CollectPayload[] {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (!raw) return [];
      const parsed = JSON.parse(raw);
      return Array.isArray(parsed) ? parsed : [];
    } catch {
      return [];
    }
  }

  private saveToStorage(): void {
    if (!this.useStorage) return;
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(this.queue));
    } catch {
      // 忽略存储失败（如隐私模式、超出配额）
    }
  }
}
