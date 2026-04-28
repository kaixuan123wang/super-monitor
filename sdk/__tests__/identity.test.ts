import { describe, it, expect, beforeEach } from 'vitest';
import { Identity } from '../src/core/identity';
import { safeStorage } from '../src/core/utils';

describe('Identity', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('creates anonymous ID on first init', () => {
    const id = new Identity();
    expect(id.getAnonymousId()).toMatch(/^anon_/);
    expect(id.getDistinctId()).toBe(id.getAnonymousId());
    expect(id.isLoginId()).toBe(false);
  });

  it('reuses stored anonymous ID', () => {
    const id1 = new Identity();
    const anon = id1.getAnonymousId();
    const id2 = new Identity();
    expect(id2.getAnonymousId()).toBe(anon);
  });

  it('identify switches to login ID', () => {
    const id = new Identity();
    const original = id.getDistinctId();
    const result = id.identify('user_123');
    expect(result.originalId).toBe(original);
    expect(result.distinctId).toBe('user_123');
    expect(id.getDistinctId()).toBe('user_123');
    expect(id.isLoginId()).toBe(true);
    expect(id.getLoginId()).toBe('user_123');
  });

  it('logout reverts to anonymous ID', () => {
    const id = new Identity();
    const anon = id.getAnonymousId();
    id.identify('user_123');
    id.logout();
    expect(id.getDistinctId()).toBe(anon);
    expect(id.isLoginId()).toBe(false);
    expect(id.getLoginId()).toBeNull();
  });

  it('setAnonymousId overrides and persists', () => {
    const id = new Identity();
    id.setAnonymousId('custom_anon');
    expect(id.getAnonymousId()).toBe('custom_anon');
    expect(id.getDistinctId()).toBe('custom_anon');
    // persisted to localStorage
    const id2 = new Identity();
    expect(id2.getAnonymousId()).toBe('custom_anon');
  });

  it('uses custom prefix', () => {
    const id = new Identity('test_');
    expect(id.getAnonymousId()).toMatch(/^test_/);
  });

  it('restores login ID from localStorage', () => {
    const id1 = new Identity();
    id1.identify('user_123');
    const id2 = new Identity();
    expect(id2.getLoginId()).toBe('user_123');
    expect(id2.getDistinctId()).toBe('user_123');
  });
});
