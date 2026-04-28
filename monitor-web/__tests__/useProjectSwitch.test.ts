import { describe, it, expect, vi, beforeEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { useProjectStore } from '@/stores/project';
import { useProjectSwitch, useEnsureProject } from '@/composables/useProjectSwitch';
import { mount } from '@vue/test-utils';
import { defineComponent, h } from 'vue';

describe('useProjectSwitch', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    localStorage.clear();
  });

  it('returns currentProjectId computed', () => {
    const store = useProjectStore();
    store.list = [{ id: 1, name: 'P1' } as any];
    store.currentId = 1;

    const TestComp = defineComponent({
      setup() {
        const { currentProjectId } = useProjectSwitch();
        return { currentProjectId };
      },
      render() {
        return h('div');
      },
    });

    const wrapper = mount(TestComp);
    expect((wrapper.vm as any).currentProjectId).toBe(1);
  });

  it('calls onSwitch when project changes', async () => {
    const store = useProjectStore();
    store.list = [{ id: 1, name: 'P1' } as any, { id: 2, name: 'P2' } as any];
    store.currentId = 1;

    const onSwitch = vi.fn();
    const TestComp = defineComponent({
      setup() {
        useProjectSwitch({ onSwitch });
        return {};
      },
      render() {
        return h('div');
      },
    });

    mount(TestComp);
    store.setCurrent(2);
    await new Promise((r) => setTimeout(r, 10));
    expect(onSwitch).toHaveBeenCalledWith(2, expect.anything());
  });

  it('setCurrentProject updates store', () => {
    const store = useProjectStore();
    store.list = [{ id: 1, name: 'P1' } as any];

    const TestComp = defineComponent({
      setup() {
        const { setCurrentProject } = useProjectSwitch();
        return { setCurrentProject };
      },
      render() {
        return h('div');
      },
    });

    const wrapper = mount(TestComp);
    (wrapper.vm as any).setCurrentProject(1);
    expect(store.currentId).toBe(1);
  });

  it('stop removes watcher', () => {
    const store = useProjectStore();
    store.list = [{ id: 1, name: 'P1' } as any];
    store.currentId = 1;

    const onSwitch = vi.fn();
    const TestComp = defineComponent({
      setup() {
        const { stop } = useProjectSwitch({ onSwitch });
        return { stop };
      },
      render() {
        return h('div');
      },
    });

    const wrapper = mount(TestComp);
    (wrapper.vm as any).stop();
    store.setCurrent(2);
    expect(onSwitch).not.toHaveBeenCalled();
  });
});

describe('useEnsureProject', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    localStorage.clear();
  });

  it('returns ensureProjectLoaded function', () => {
    const TestComp = defineComponent({
      setup() {
        const { ensureProjectLoaded } = useEnsureProject();
        return { ensureProjectLoaded };
      },
      render() {
        return h('div');
      },
    });

    const wrapper = mount(TestComp);
    expect((wrapper.vm as any).ensureProjectLoaded).toBeInstanceOf(Function);
  });

  it('ensureProjectLoaded returns currentId when projects exist', async () => {
    const store = useProjectStore();
    store.list = [{ id: 1, name: 'P1' } as any];
    store.currentId = 1;

    const TestComp = defineComponent({
      setup() {
        const { ensureProjectLoaded } = useEnsureProject();
        return { ensureProjectLoaded };
      },
      render() {
        return h('div');
      },
    });

    const wrapper = mount(TestComp);
    const id = await (wrapper.vm as any).ensureProjectLoaded();
    expect(id).toBe(1);
  });
});
