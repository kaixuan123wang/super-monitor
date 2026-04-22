/**
 * 本地缓存/队列
 *
 * Phase 2: 内存 FIFO 队列，超限时优先丢弃非 P0 的旧数据。
 * 后续阶段会替换为 IndexedDB 持久化队列。
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
