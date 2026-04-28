import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { SSEClient } from '@/utils/sse';

describe('SSEClient', () => {
  let instances: any[] = [];

  beforeEach(() => {
    instances = [];
    class MockEventSource {
      url: string;
      onopen: ((this: EventSource, ev: Event) => void) | null = null;
      onerror: ((this: EventSource, ev: Event) => void) | null = null;
      private listeners = new Map<string, any[]>();
      close = vi.fn(() => {
        this.onopen = null;
        this.onerror = null;
      });
      addEventListener = vi.fn((evt: string, fn: any) => {
        if (!this.listeners.has(evt)) this.listeners.set(evt, []);
        this.listeners.get(evt)!.push(fn);
      });
      getListeners(evt: string) {
        return this.listeners.get(evt) ?? [];
      }
      constructor(url: string) {
        this.url = url;
        instances.push(this);
      }
    }
    vi.stubGlobal('EventSource', MockEventSource as any);
    vi.stubGlobal('fetch', vi.fn(() =>
      Promise.resolve({ ok: true, json: () => Promise.resolve({ data: { token: 'sse_token_123' } }) })
    ));
    localStorage.setItem('__monitor_access_token', 'access_token_xyz');
  });

  afterEach(() => {
    localStorage.clear();
    vi.unstubAllGlobals();
    vi.clearAllTimers();
    instances = [];
  });

  it('constructs with url', () => {
    const client = new SSEClient('/api/events');
    expect(client).toBeInstanceOf(SSEClient);
  });

  it('connect opens EventSource', async () => {
    const client = new SSEClient('/api/events');
    client.connect();
    await new Promise((r) => setTimeout(r, 10));
    expect(instances.length).toBe(1);
  });

  it('on registers handler and receives data', async () => {
    const client = new SSEClient('/api/events');
    const handler = vi.fn();
    client.on('message', handler);
    client.connect();
    await new Promise((r) => setTimeout(r, 10));

    const es = instances[0];
    const msgListeners = es.getListeners('message');
    expect(msgListeners.length).toBeGreaterThan(0);

    msgListeners[0]({ data: JSON.stringify({ type: 'test' }) });
    expect(handler).toHaveBeenCalledWith({ type: 'test' });
  });

  it('off removes specific handler', async () => {
    const client = new SSEClient('/api/events');
    const handler = vi.fn();
    client.on('message', handler);
    client.off('message', handler);
    client.connect();
    await new Promise((r) => setTimeout(r, 10));

    const es = instances[0];
    const msgListeners = es.getListeners('message');
    if (msgListeners.length > 0) {
      msgListeners[0]({ data: JSON.stringify({ type: 'test' }) });
    }
    expect(handler).not.toHaveBeenCalled();
  });

  it('off removes all handlers for event', async () => {
    const client = new SSEClient('/api/events');
    const handler = vi.fn();
    client.on('message', handler);
    client.off('message');
    client.connect();
    await new Promise((r) => setTimeout(r, 10));
    const es = instances[0];
    expect(es.getListeners('message').length).toBe(0);
  });

  it('disconnect closes connection', async () => {
    const client = new SSEClient('/api/events');
    client.connect();
    await new Promise((r) => setTimeout(r, 10));
    const es = instances[0];
    client.disconnect();
    expect(es.close).toHaveBeenCalled();
  });

  it('reconnects on error with backoff', async () => {
    vi.useFakeTimers();
    const client = new SSEClient('/api/events');
    client.connect();
    await vi.advanceTimersByTimeAsync(10);

    const es = instances[0];
    es.onerror?.({} as any);
    expect(es.close).toHaveBeenCalled();

    await vi.advanceTimersByTimeAsync(1100);
    expect(instances.length).toBe(2);

    vi.useRealTimers();
  });

  it('does not reconnect after disconnect', async () => {
    vi.useFakeTimers();
    const client = new SSEClient('/api/events');
    client.connect();
    await vi.advanceTimersByTimeAsync(10);
    client.disconnect();

    const es = instances[0];
    es.onerror?.({} as any);
    await vi.advanceTimersByTimeAsync(2000);
    expect(instances.length).toBe(1);
    vi.useRealTimers();
  });

  it('fetches sse token before connecting', async () => {
    localStorage.setItem('__monitor_access_token', 'access_token_xyz');
    const client = new SSEClient('/api/events');
    client.connect();
    await new Promise((r) => setTimeout(r, 10));

    const fetchCalls = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls;
    const sseTokenCall = fetchCalls.find((c: any[]) => c[0] === '/api/auth/sse-token');
    expect(sseTokenCall).toBeDefined();
    localStorage.removeItem('__monitor_access_token');
  });

  it('does not connect without an SSE token', async () => {
    vi.stubGlobal('fetch', vi.fn(() => Promise.reject(new Error('network'))));
    const client = new SSEClient('/api/events');
    client.connect();
    await new Promise((r) => setTimeout(r, 10));
    expect(instances.length).toBe(0);
  });

  it('handles non-JSON data gracefully', async () => {
    const client = new SSEClient('/api/events');
    const handler = vi.fn();
    client.on('message', handler);
    client.connect();
    await new Promise((r) => setTimeout(r, 10));

    const es = instances[0];
    const msgListeners = es.getListeners('message');
    expect(msgListeners.length).toBeGreaterThan(0);
    msgListeners[0]({ data: 'plain text' });
    expect(handler).toHaveBeenCalledWith('plain text');
  });
});
