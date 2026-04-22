/**
 * 本地缓存/队列
 *
 * Phase 1: 内存队列 + localStorage 降级。
 * Phase 2 起会扩展为 IndexedDB 持久化队列（失败补发 / FIFO）。
 */

import type { CollectPayload } from '../types';

export interface StoreOptions {
  maxQueueSize: number;
}

export class Store {
  private queue: CollectPayload[] = [];
  private readonly maxQueueSize: number;

  constructor(options: StoreOptions) {
    this.maxQueueSize = options.maxQueueSize;
  }

  /** 入队；超限时丢弃最旧的一条 */
  enqueue(payload: CollectPayload): void {
    if (this.queue.length >= this.maxQueueSize) {
      this.queue.shift();
    }
    this.queue.push(payload);
  }

  /** 取出全部并清空队列 */
  drain(): CollectPayload[] {
    const items = this.queue;
    this.queue = [];
    return items;
  }

  /** 当前队列长度 */
  size(): number {
    return this.queue.length;
  }
}
