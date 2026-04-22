import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { listProjects, type Project } from '@/api/project';

const STORAGE_KEY = '__monitor_current_project_id';

export const useProjectStore = defineStore('project', () => {
  const list = ref<Project[]>([]);
  const currentId = ref<number | null>(null);
  const loading = ref(false);

  const current = computed<Project | null>(() => {
    if (currentId.value == null) return null;
    return list.value.find((p) => p.id === currentId.value) || null;
  });

  async function fetchAll(keyword?: string) {
    loading.value = true;
    try {
      const resp = await listProjects({ page: 1, page_size: 100, keyword });
      list.value = resp.data?.list ?? [];

      // 恢复 / 初始化 currentId
      if (currentId.value == null) {
        const saved = Number(localStorage.getItem(STORAGE_KEY) || 0);
        if (saved && list.value.some((p) => p.id === saved)) {
          currentId.value = saved;
        } else if (list.value[0]) {
          currentId.value = list.value[0].id;
        }
      }
    } finally {
      loading.value = false;
    }
  }

  function setCurrent(id: number | null) {
    currentId.value = id;
    if (id) {
      localStorage.setItem(STORAGE_KEY, String(id));
    } else {
      localStorage.removeItem(STORAGE_KEY);
    }
  }

  function upsert(p: Project) {
    const idx = list.value.findIndex((x) => x.id === p.id);
    if (idx >= 0) list.value.splice(idx, 1, p);
    else list.value.unshift(p);
  }

  function remove(id: number) {
    list.value = list.value.filter((p) => p.id !== id);
    if (currentId.value === id) {
      setCurrent(list.value[0]?.id ?? null);
    }
  }

  return { list, currentId, current, loading, fetchAll, setCurrent, upsert, remove };
});
