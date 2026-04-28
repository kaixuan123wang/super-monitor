import { describe, it, expect, beforeEach, vi } from 'vitest';
import { Monitor } from '../src/core/monitor';

describe('Monitor', () => {
  beforeEach(() => {
    Monitor.destroy();
  });

  it('should initialize with required config', () => {
    Monitor.init({
      appId: 'test-app',
      appKey: 'test-key',
      server: 'https://test.example.com',
    });
    expect(() => Monitor.track('test_event')).not.toThrow();
  });

  it('should throw when missing required fields', () => {
    expect(() => Monitor.init({ appId: '', appKey: '', server: '' } as any)).toThrow(
      'init requires appId, appKey and server'
    );
  });

  it('should ignore duplicate init calls', () => {
    Monitor.init({ appId: 'test', appKey: 'key', server: 'https://test.com' });
    Monitor.init({ appId: 'test2', appKey: 'key2', server: 'https://test2.com' });
    // Should not throw and should keep first config behavior (we verify by checking no error)
    expect(() => Monitor.track('event')).not.toThrow();
  });

  it('should report track events', () => {
    Monitor.init({ appId: 'test', appKey: 'key', server: 'https://test.com' });
    expect(() => Monitor.track('custom_event', { foo: 'bar' })).not.toThrow();
  });

  it('should identify user', () => {
    Monitor.init({ appId: 'test', appKey: 'key', server: 'https://test.com' });
    expect(() => Monitor.identify('user_123')).not.toThrow();
  });

  it('should set user properties', () => {
    Monitor.init({ appId: 'test', appKey: 'key', server: 'https://test.com' });
    expect(() => Monitor.setUserProperties({ plan: 'pro' })).not.toThrow();
  });

  it('should register super properties', () => {
    Monitor.init({ appId: 'test', appKey: 'key', server: 'https://test.com' });
    expect(() => Monitor.registerSuperProperties({ channel: 'web' })).not.toThrow();
  });

  it('should add breadcrumb', () => {
    Monitor.init({ appId: 'test', appKey: 'key', server: 'https://test.com' });
    expect(() =>
      Monitor.addBreadcrumb({ category: 'custom', message: 'test breadcrumb' })
    ).not.toThrow();
  });

  it('should warn when calling methods before init', () => {
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
    Monitor.init({ appId: 'test', appKey: 'key', server: 'https://test.com', debug: true });
    Monitor.destroy();
    Monitor.track('event');
    expect(warnSpy).toHaveBeenCalledWith(
      '[Monitor]',
      expect.stringContaining('not initialized')
    );
    warnSpy.mockRestore();
  });

  it('should destroy and reset state', () => {
    Monitor.init({ appId: 'test', appKey: 'key', server: 'https://test.com', debug: true });
    Monitor.destroy();
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
    Monitor.track('event');
    expect(warnSpy).toHaveBeenCalledWith(
      '[Monitor]',
      expect.stringContaining('not initialized')
    );
    warnSpy.mockRestore();
  });

  it('should re-init after destroy', () => {
    Monitor.init({ appId: 'test', appKey: 'key', server: 'https://test.com' });
    Monitor.destroy();
    expect(() =>
      Monitor.init({ appId: 'test2', appKey: 'key2', server: 'https://test2.com' })
    ).not.toThrow();
  });
});
