/**
 * 通用列表 + 筛选 + 分页 composable。
 *
 * 用法：
 * ```ts
 * const { list, total, loading, filter, reload, onSearch, onReset, onPageChange } = useTableList({
 *   fetchApi: listErrors,
 *   defaultFilter: { error_type: undefined, browser: undefined },
 * });
 * ```
 */

import { onMounted, reactive, ref, watch, type Ref } from 'vue';
import { useProjectStore } from '@/stores/project';

export interface PaginatedResponse<T> {
  list: T[];
  total: number;
}

export interface ApiResponse<T> {
  code: number;
  message: string;
  data: T;
  pagination?: {
    page: number;
    page_size: number;
    total: number;
    total_pages: number;
  };
}

export interface UseTableListOptions<F extends Record<string, any>> {
  /** 获取列表数据的 API 函数 */
  fetchApi: (params: any) => Promise<ApiResponse<PaginatedResponse<any>>>;
  /** 默认筛选条件（不含 page、page_size） */
  defaultFilter?: F;
  /** 默认每页条数 */
  defaultPageSize?: number;
  /** 是否在挂载时自动加载 */
  autoLoad?: boolean;
  /** 是否监听项目切换 */
  watchProject?: boolean;
  /** 额外的依赖项，变化时自动刷新 */
  watchDeps?: () => any[];
}

export function useTableList<F extends Record<string, any>>(options: UseTableListOptions<F>) {
  const {
    fetchApi,
    defaultFilter = {} as F,
    defaultPageSize = 20,
    autoLoad = true,
    watchProject = true,
    watchDeps,
  } = options;

  const projectStore = useProjectStore();

  const loading = ref(false);
  const list = ref<any[]>([]) as Ref<any[]>;
  const total = ref(0);

  const filter = reactive<F & { page: number; page_size: number }>({
    ...defaultFilter,
    page: 1,
    page_size: defaultPageSize,
  } as F & { page: number; page_size: number });

  async function reload() {
    if (watchProject && !projectStore.currentId) {
      list.value = [];
      total.value = 0;
      return;
    }

    loading.value = true;
    try {
      const params = watchProject
        ? { project_id: projectStore.currentId, ...filter }
        : { ...filter };
      const resp = await fetchApi(params);
      list.value = resp.data?.list ?? [];
      total.value = resp.data?.total ?? 0;
    } catch {
      // 全局拦截器已提示
    } finally {
      loading.value = false;
    }
  }

  function onSearch() {
    filter.page = 1;
    reload();
  }

  function onReset() {
    Object.assign(filter, {
      ...defaultFilter,
      page: 1,
      page_size: defaultPageSize,
    });
    reload();
  }

  function onPageChange(p: number) {
    filter.page = p;
    reload();
  }

  function onPageSizeChange(size: number) {
    filter.page_size = size;
    filter.page = 1;
    reload();
  }

  onMounted(async () => {
    if (watchProject && !projectStore.list.length) {
      await projectStore.fetchAll();
    }
    if (autoLoad) {
      reload();
    }
  });

  // 监听项目切换
  if (watchProject) {
    watch(
      () => projectStore.currentId,
      () => {
        filter.page = 1;
        reload();
      }
    );
  }

  // 监听额外依赖
  if (watchDeps) {
    watch(watchDeps, () => {
      filter.page = 1;
      reload();
    });
  }

  return {
    list,
    total,
    loading,
    filter,
    reload,
    onSearch,
    onReset,
    onPageChange,
    onPageSizeChange,
  };
}
