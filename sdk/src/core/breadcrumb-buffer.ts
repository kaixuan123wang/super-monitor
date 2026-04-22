/**
 * 面包屑环形缓冲区
 *
 * 维护最近 N 条用户操作路径，错误上报时一并附带。
 */

import type { BreadcrumbItem } from '../types';
import { now } from './utils';

export class BreadcrumbBuffer {
  private items: BreadcrumbItem[] = [];
  private readonly maxSize: number;

  constructor(maxSize = 30) {
    this.maxSize = maxSize;
  }

  push(item: Omit<BreadcrumbItem, 'timestamp'> & { timestamp?: number }): void {
    const full: BreadcrumbItem = {
      timestamp: item.timestamp ?? now(),
      ...item,
    };
    this.items.push(full);
    if (this.items.length > this.maxSize) {
      this.items.shift();
    }
  }

  getAll(): BreadcrumbItem[] {
    return this.items.slice();
  }

  clear(): void {
    this.items = [];
  }
}
