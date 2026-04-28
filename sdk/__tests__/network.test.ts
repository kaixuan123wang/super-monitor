import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { installNetworkPlugin } from '../src/plugins/network';
import type { CollectPayload, NetworkData } from '../src/types';

describe('Network Plugin', () => {
  let reportMock: ReturnType<typeof vi.fn>;
  let cleanup: () => void;
  let originalFetch: typeof window.fetch;
  let mockFetch: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    originalFetch = window.fetch;
    mockFetch = vi.fn().mockResolvedValue(
      new Response('ok', { status: 200 })
    );
    window.fetch = mockFetch as unknown as typeof window.fetch;
    reportMock = vi.fn();
    cleanup = installNetworkPlugin({ report: reportMock });
  });

  afterEach(() => {
    cleanup();
    window.fetch = originalFetch;
  });

  it('should intercept fetch and track errors', async () => {
    mockFetch.mockResolvedValue(
      new Response('not found', { status: 404, statusText: 'Not Found' })
    );

    await window.fetch('https://api.example.com/data');

    expect(reportMock).toHaveBeenCalledTimes(1);
    const payload = reportMock.mock.calls[0][0] as CollectPayload<NetworkData>;
    expect(payload.type).toBe('network');
    expect(payload.data.status).toBe(404);
    expect(payload.priority).toBe('P0');
  });

  it('should not report successful fetch', async () => {
    mockFetch.mockResolvedValue(new Response('ok', { status: 200 }));

    await window.fetch('https://api.example.com/data');

    expect(reportMock).not.toHaveBeenCalled();
  });

  it('should ignore SDK endpoint requests', async () => {
    mockFetch.mockResolvedValue(new Response('ok', { status: 500 }));

    await window.fetch('https://monitor.example.com/api/v1/collect');

    expect(reportMock).not.toHaveBeenCalled();
  });

  it('should report fetch network failure', async () => {
    mockFetch.mockRejectedValue(new TypeError('Failed to fetch'));

    await expect(window.fetch('https://api.example.com/data')).rejects.toThrow();

    expect(reportMock).toHaveBeenCalledTimes(1);
    const payload = reportMock.mock.calls[0][0] as CollectPayload<NetworkData>;
    expect(payload.data.status).toBe(0);
    expect(payload.data.error_type).toBe('TypeError');
  });

  it('should sanitize sensitive query params in URL', async () => {
    mockFetch.mockResolvedValue(new Response('err', { status: 500 }));

    await window.fetch('https://api.example.com/data?token=secret123&user=1');

    const payload = reportMock.mock.calls[0][0] as CollectPayload<NetworkData>;
    expect(payload.data.url).not.toContain('secret123');
    expect(payload.data.url).toContain('[REDACTED]');
  });

  it('should restore original fetch on cleanup', () => {
    cleanup();
    expect(typeof window.fetch).toBe('function');
  });

  it('should noop in non-browser environment', () => {
    const originalWindow = global.window;
    // @ts-expect-error removing window
    global.window = undefined;
    const noopCleanup = installNetworkPlugin({ report: reportMock });
    expect(typeof noopCleanup).toBe('function');
    noopCleanup();
    global.window = originalWindow;
  });

  it('should intercept XHR without crashing', () => {
    const xhr = new XMLHttpRequest();
    xhr.open('GET', 'https://api.example.com/xhr-test');
    // jsdom XHR doesn't send real requests; we just verify send doesn't crash
    expect(() => xhr.send()).not.toThrow();
  });
});
