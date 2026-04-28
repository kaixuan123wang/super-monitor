import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { Timer } from '../src/core/timer';

describe('Timer', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('returns undefined for unknown event', () => {
    const timer = new Timer();
    expect(timer.end('unknown')).toBeUndefined();
  });

  it('measures duration in seconds with 3 decimals', () => {
    const timer = new Timer();
    timer.start('evt');
    vi.advanceTimersByTime(1500);
    const duration = timer.end('evt');
    expect(duration).toBe(1.5);
  });

  it('deletes timer after end', () => {
    const timer = new Timer();
    timer.start('evt');
    timer.end('evt');
    expect(timer.end('evt')).toBeUndefined();
  });

  it('clear single event', () => {
    const timer = new Timer();
    timer.start('evt');
    timer.clear('evt');
    expect(timer.end('evt')).toBeUndefined();
  });

  it('clear all events', () => {
    const timer = new Timer();
    timer.start('a');
    timer.start('b');
    timer.clear();
    expect(timer.end('a')).toBeUndefined();
    expect(timer.end('b')).toBeUndefined();
  });

  it('handles very short durations', () => {
    const timer = new Timer();
    timer.start('evt');
    vi.advanceTimersByTime(1);
    const duration = timer.end('evt');
    expect(duration).toBe(0.001);
  });

  it('handles zero duration', () => {
    const timer = new Timer();
    timer.start('evt');
    const duration = timer.end('evt');
    expect(duration).toBe(0);
  });
});
