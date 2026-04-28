import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { Reporter, buildReporterFromConfig } from '../src/core/reporter';
import { Store } from '../src/core/store';
import type { CollectPayload, MonitorConfig, ReportContext } from '../src/types';

describe('Reporter', () => {
  let store: Store;

  beforeEach(() => {
    store = new Store({ maxQueueSize: 10 });
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  function createReporter(opts?: Partial<ConstructorParameters<typeof Reporter>[0]>) {
    return new Reporter({
      server: 'https://monitor.example.com',
      appId: 'app1',
      appKey: 'key1',
      flushInterval: 1000,
      retryMaxCount: 2,
      retryInterval: 100,
      store,
      getContext: () => ({ sdk_version: '1.0.0' }),
      ...opts,
    });
  }

  it('builds endpoint from server option', () => {
    const r = createReporter({ server: 'https://example.com/' });
    expect((r as unknown as Record<string, string>).endpoint).toBe('https://example.com/api/v1/collect');
  });

  it('report P0 immediately via sendImmediate', async () => {
    const fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValue({ ok: true } as Response);
    const r = createReporter();
    r.report({ type: 'error', data: { msg: 'oops' }, priority: 'P0' });
    // P0 uses sendImmediate which is async; wait for microtasks
    await vi.advanceTimersByTimeAsync(0);
    expect(fetchSpy).toHaveBeenCalled();
    fetchSpy.mockRestore();
  });

  it('report P1 enqueues to store', () => {
    const r = createReporter();
    r.report({ type: 'track', data: { event: 'click' }, priority: 'P1' });
    expect(store.size()).toBe(1);
  });

  it('flush sends drained items', async () => {
    const fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValue({ ok: true } as Response);
    const r = createReporter();
    r.report({ type: 'track', data: { event: 'a' }, priority: 'P1' });
    await r.flush();
    expect(fetchSpy).toHaveBeenCalled();
    expect(store.size()).toBe(0);
    fetchSpy.mockRestore();
  });

  it('flush does nothing when store is empty', async () => {
    const fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValue({ ok: true } as Response);
    const r = createReporter();
    await r.flush();
    expect(fetchSpy).not.toHaveBeenCalled();
    fetchSpy.mockRestore();
  });

  it('flush re-enqueues items on failure', async () => {
    const fetchSpy = vi.spyOn(globalThis, 'fetch').mockRejectedValue(new Error('network'));
    const r = createReporter({ retryMaxCount: 0 });
    r.report({ type: 'track', data: { event: 'a' }, priority: 'P1' });
    await r.flush();
    expect(store.size()).toBe(1);
    fetchSpy.mockRestore();
  });

  it('retry on non-ok response', async () => {
    const fetchSpy = vi.spyOn(globalThis, 'fetch')
      .mockResolvedValueOnce({ ok: false, status: 500 } as Response)
      .mockResolvedValueOnce({ ok: true } as Response);
    const r = createReporter({ retryMaxCount: 1, retryInterval: 50 });
    r.report({ type: 'track', data: { event: 'a' }, priority: 'P1' });
    const flushPromise = r.flush();
    await vi.advanceTimersByTimeAsync(60);
    await flushPromise;
    expect(fetchSpy).toHaveBeenCalledTimes(2);
    fetchSpy.mockRestore();
  });

  it('start/stop manages interval timer', () => {
    const r = createReporter();
    r.start();
    vi.advanceTimersByTime(1000);
    r.stop();
    // no error expected
  });

  it('flushSync sends with keepalive when supported', () => {
    const fetchSpy = vi.spyOn(globalThis, 'fetch').mockReturnValue(undefined as unknown as Promise<Response>);
    const r = createReporter();
    r.report({ type: 'track', data: { event: 'a' }, priority: 'P1' });
    r.flushSync();
    expect(fetchSpy).toHaveBeenCalled();
    const call = fetchSpy.mock.calls[0] as [string, RequestInit];
    expect(call[1].keepalive).toBe(true);
    fetchSpy.mockRestore();
  });

  it('wrapBatch returns single item for length 1', async () => {
    const fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValue({ ok: true } as Response);
    const r = createReporter();
    r.report({ type: 'track', data: { event: 'a' }, priority: 'P1' });
    await r.flush();
    const body = JSON.parse((fetchSpy.mock.calls[0][1] as RequestInit).body as string);
    expect(body.type).toBe('track');
    expect(body.data.__context).toBeDefined();
    fetchSpy.mockRestore();
  });

  it('wrapBatch returns batch for multiple items', async () => {
    const fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValue({ ok: true } as Response);
    const r = createReporter();
    r.report({ type: 'track', data: { event: 'a' }, priority: 'P1' });
    r.report({ type: 'track', data: { event: 'b' }, priority: 'P1' });
    await r.flush();
    const body = JSON.parse((fetchSpy.mock.calls[0][1] as RequestInit).body as string);
    expect(body.type).toBe('batch');
    expect(body.data.length).toBe(2);
    fetchSpy.mockRestore();
  });
});

describe('buildReporterFromConfig', () => {
  it('creates Reporter from MonitorConfig defaults', () => {
    const config: MonitorConfig = {
      appId: 'a',
      appKey: 'k',
      server: 'https://s.example.com',
    };
    const store = new Store({ maxQueueSize: 10 });
    const r = buildReporterFromConfig(config, store);
    expect(r).toBeInstanceOf(Reporter);
  });

  it('uses custom reporter config when provided', () => {
    const config: MonitorConfig = {
      appId: 'a',
      appKey: 'k',
      server: 'https://s.example.com',
      reporter: {
        flushInterval: 10_000,
        retryMaxCount: 5,
        retryInterval: 60_000,
      },
    };
    const store = new Store({ maxQueueSize: 10 });
    const r = buildReporterFromConfig(config, store);
    expect(r).toBeInstanceOf(Reporter);
  });
});
