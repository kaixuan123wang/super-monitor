/**
 * 项目切换监听 composable。
 *
 * 用法：
 * ```ts
 * // 基础用法
 * const { currentProjectId, currentProject } = useProjectSwitch();
 *
 * // 带回调
 * useProjectSwitch({
 *   onSwitch: (projectId) => {
 *     // 项目切换时的处理
 *     fetchData(projectId);
 *   },
 * });
 *
 * // 立即执行一次
 * useProjectSwitch({
 *   onSwitch: (projectId) => fetchData(projectId),
 *   immediate: true,
 * });
 * ```
 */

import { computed, onMounted, watch, type WatchStopHandle } from 'vue';
import { useProjectStore } from '@/stores/project';

export interface UseProjectSwitchOptions {
  /** 项目切换时的回调 */
  onSwitch?: (projectId: number | null, project: any) => void;
  /** 是否在挂载时立即执行一次回调 */
  immediate?: boolean;
  /** 是否自动加载项目列表 */
  autoLoad?: boolean;
}

export function useProjectSwitch(options: UseProjectSwitchOptions = {}) {
  const { onSwitch, immediate = false, autoLoad = true } = options;

  const projectStore = useProjectStore();

  const currentProjectId = computed(() => projectStore.currentId);
  const currentProject = computed(() => projectStore.current);

  // 监听项目切换
  let stopWatch: WatchStopHandle | undefined;
  if (onSwitch) {
    stopWatch = watch(
      () => projectStore.currentId,
      (newId) => {
        onSwitch(newId, projectStore.current);
      }
    );
  }

  // 挂载时加载项目列表
  onMounted(async () => {
    if (autoLoad && !projectStore.list.length) {
      await projectStore.fetchAll();
    }
    if (immediate && onSwitch) {
      onSwitch(projectStore.currentId, projectStore.current);
    }
  });

  // 清理
  const stop = () => {
    stopWatch?.();
  };

  return {
    currentProjectId,
    currentProject,
    projectList: computed(() => projectStore.list),
    setCurrentProject: (id: number) => projectStore.setCurrent(id),
    stop,
  };
}

/**
 * 确保项目已加载的 composable。
 * 如果项目列表未加载，会自动加载并等待完成。
 */
export function useEnsureProject() {
  const projectStore = useProjectStore();

  async function ensureProjectLoaded() {
    if (!projectStore.list.length) {
      await projectStore.fetchAll();
    }
    return projectStore.currentId;
  }

  return {
    ensureProjectLoaded,
    currentProjectId: computed(() => projectStore.currentId),
    currentProject: computed(() => projectStore.current),
  };
}
