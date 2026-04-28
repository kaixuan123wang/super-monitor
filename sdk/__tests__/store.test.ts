import { describe, it, expect, beforeEach } from 'vitest';
import { Store } from '../src/core/store';
import type { CollectPayload } from '../src/types';

function makePayload(type: string, priority: 'P0' | 'P1' = 'P1'): CollectPayload {
  return { type: type as CollectPayload['type'], data: {}, priority };
}

describe('Store', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('initializes with given maxQueueSize', () => {
    const store = new Store({ maxQueueSize: 5 });
    expect(store.size()).toBe(0);
  });

  it('enqueues items', () => {
    const store = new Store({ maxQueueSize: 10 });
    store.enqueue(makePayload('track'));
    store.enqueue(makePayload('error'));
    expect(store.size()).toBe(2);
  });

  it('drains returns all items and clears queue', () => {
    const store = new Store({ maxQueueSize: 10 });
    store.enqueue(makePayload('a'));
    store.enqueue(makePayload('b'));
    const items = store.drain();
    expect(items.length).toBe(2);
    expect(store.size()).toBe(0);
  });

  it('drops oldest non-P0 item when full', () => {
    const store = new Store({ maxQueueSize: 3 });
    store.enqueue(makePayload('a', 'P1'));
    store.enqueue(makePayload('b', 'P1'));
    store.enqueue(makePayload('c', 'P0'));
    store.enqueue(makePayload('d', 'P1'));
    // should drop the oldest P1 (a), keeping b, c(P0), d
    const items = store.drain();
    expect(items.map((i) => i.type)).toEqual(['b', 'c', 'd']);
  });

  it('drops oldest item when all are P0', () => {
    const store = new Store({ maxQueueSize: 2 });
    store.enqueue(makePayload('a', 'P0'));
    store.enqueue(makePayload('b', 'P0'));
    store.enqueue(makePayload('c', 'P0'));
    const items = store.drain();
    expect(items.map((i) => i.type)).toEqual(['b', 'c']);
  });

  it('respects maxQueueSize of 1', () => {
    const store = new Store({ maxQueueSize: 1 });
    store.enqueue(makePayload('a', 'P1'));
    store.enqueue(makePayload('b', 'P0'));
    const items = store.drain();
    expect(items.length).toBe(1);
    expect(items[0].type).toBe('b');
  });

  it('persists queue to localStorage', () => {
    const store = new Store({ maxQueueSize: 10 });
    store.enqueue(makePayload('persist_a'));
    store.enqueue(makePayload('persist_b'));

    const raw = localStorage.getItem('__monitor_queue__');
    expect(raw).toBeTruthy();
    const parsed = JSON.parse(raw!);
    expect(parsed.length).toBe(2);
    expect(parsed[0].type).toBe('persist_a');
  });

  it('restores queue from localStorage on init', () => {
    localStorage.setItem(
      '__monitor_queue__',
      JSON.stringify([makePayload('restored'), makePayload('restored2')])
    );
    const store = new Store({ maxQueueSize: 10 });
    expect(store.size()).toBe(2);
    const items = store.drain();
    expect(items.map((i) => i.type)).toEqual(['restored', 'restored2']);
  });

  it('clears localStorage on drain', () => {
    const store = new Store({ maxQueueSize: 10 });
    store.enqueue(makePayload('x'));
    store.drain();
    const raw = localStorage.getItem('__monitor_queue__');
    expect(raw).toBe('[]');
  });

  it('survives localStorage quota errors gracefully', () => {
    const store = new Store({ maxQueueSize: 10 });
    // Simulate quota exceeded by mocking setItem
    const originalSetItem = localStorage.setItem;
    localStorage.setItem = () => {
      throw new DOMException('Quota exceeded', 'QuotaExceededError');
    };
    expect(() => store.enqueue(makePayload('quota_test'))).not.toThrow();
    localStorage.setItem = originalSetItem;
  });
});
