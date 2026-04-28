import { describe, it, expect, beforeEach } from 'vitest';
import { SuperProperties } from '../src/core/super-props';

describe('SuperProperties', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('starts empty when no stored props', () => {
    const sp = new SuperProperties();
    expect(sp.getAll()).toEqual({});
  });

  it('registers properties', () => {
    const sp = new SuperProperties();
    sp.register({ platform: 'web', version: '1.0.0' });
    expect(sp.getAll()).toEqual({ platform: 'web', version: '1.0.0' });
  });

  it('merges new properties with existing', () => {
    const sp = new SuperProperties();
    sp.register({ a: 1 });
    sp.register({ b: 2 });
    expect(sp.getAll()).toEqual({ a: 1, b: 2 });
  });

  it('overwrites existing properties', () => {
    const sp = new SuperProperties();
    sp.register({ a: 1 });
    sp.register({ a: 2 });
    expect(sp.getAll()).toEqual({ a: 2 });
  });

  it('unregisters a property', () => {
    const sp = new SuperProperties();
    sp.register({ a: 1, b: 2 });
    sp.unregister('a');
    expect(sp.getAll()).toEqual({ b: 2 });
  });

  it('clear removes all properties', () => {
    const sp = new SuperProperties();
    sp.register({ a: 1 });
    sp.clear();
    expect(sp.getAll()).toEqual({});
  });

  it('persists to localStorage', () => {
    const sp1 = new SuperProperties();
    sp1.register({ key: 'value' });
    const sp2 = new SuperProperties();
    expect(sp2.getAll()).toEqual({ key: 'value' });
  });

  it('getAll returns a copy', () => {
    const sp = new SuperProperties();
    sp.register({ a: 1 });
    const all = sp.getAll();
    (all as Record<string, number>).a = 2;
    expect(sp.getAll()).toEqual({ a: 1 });
  });
});
