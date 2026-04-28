import { describe, it, expect, beforeEach, vi } from 'vitest';
import { BreadcrumbBuffer } from '../src/core/breadcrumb-buffer';

describe('BreadcrumbBuffer', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  it('initializes empty', () => {
    const buf = new BreadcrumbBuffer(5);
    expect(buf.getAll()).toEqual([]);
  });

  it('pushes items with auto timestamp', () => {
    const buf = new BreadcrumbBuffer(5);
    vi.setSystemTime(1000);
    buf.push({ category: 'click', message: 'btn' });
    const items = buf.getAll();
    expect(items.length).toBe(1);
    expect(items[0].category).toBe('click');
    expect(items[0].message).toBe('btn');
    expect(items[0].timestamp).toBe(1000);
  });

  it('respects custom timestamp', () => {
    const buf = new BreadcrumbBuffer(5);
    buf.push({ category: 'click', message: 'btn', timestamp: 500 });
    expect(buf.getAll()[0].timestamp).toBe(500);
  });

  it('drops oldest items when exceeding maxSize', () => {
    const buf = new BreadcrumbBuffer(3);
    buf.push({ category: 'a', message: '1' });
    buf.push({ category: 'b', message: '2' });
    buf.push({ category: 'c', message: '3' });
    buf.push({ category: 'd', message: '4' });
    const items = buf.getAll();
    expect(items.map((i) => i.message)).toEqual(['2', '3', '4']);
  });

  it('clear removes all items', () => {
    const buf = new BreadcrumbBuffer(5);
    buf.push({ category: 'click', message: 'btn' });
    buf.clear();
    expect(buf.getAll()).toEqual([]);
  });

  it('getAll returns a shallow copy', () => {
    const buf = new BreadcrumbBuffer(5);
    buf.push({ category: 'click', message: 'btn' });
    const a = buf.getAll();
    const b = buf.getAll();
    expect(a).not.toBe(b);
    expect(a).toEqual(b);
  });
});
