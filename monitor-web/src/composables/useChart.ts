/**
 * ECharts 图表 composable。
 *
 * 用法：
 * ```ts
 * const chartRef = ref<HTMLElement>();
 * const { setOption, dispose } = useChart(chartRef);
 *
 * // 设置图表选项
 * setOption({
 *   title: { text: '示例图表' },
 *   xAxis: { type: 'category', data: ['Mon', 'Tue', 'Wed'] },
 *   yAxis: { type: 'value' },
 *   series: [{ type: 'line', data: [150, 230, 224] }],
 * });
 * ```
 */

import { onBeforeUnmount, onMounted, ref, watch, type Ref, shallowRef } from 'vue';
import * as echarts from 'echarts/core';
import { CanvasRenderer } from 'echarts/renderers';

// 注册渲染器
echarts.use([CanvasRenderer]);

export interface UseChartOptions {
  /** 主题 */
  theme?: string;
  /** 是否在窗口 resize 时自动调整 */
  autoResize?: boolean;
  /** 是否在组件卸载时自动销毁 */
  autoDispose?: boolean;
  /** 渲染器 */
  renderer?: 'canvas' | 'svg';
}

export function useChart(
  containerRef: Ref<HTMLElement | undefined>,
  options: UseChartOptions = {}
) {
  const { theme, autoResize = true, autoDispose = true } = options;

  const chartInstance = shallowRef<echarts.ECharts>();

  function init() {
    if (!containerRef.value) return;
    if (chartInstance.value) {
      chartInstance.value.dispose();
    }
    chartInstance.value = echarts.init(containerRef.value, theme);
  }

  function setOption(option: echarts.EChartsCoreOption, notMerge = false) {
    if (!chartInstance.value) {
      init();
    }
    chartInstance.value?.setOption(option, notMerge);
  }

  function resize() {
    chartInstance.value?.resize();
  }

  function dispose() {
    chartInstance.value?.dispose();
    chartInstance.value = undefined;
  }

  function getInstance() {
    return chartInstance.value;
  }

  // 监听容器变化
  watch(containerRef, (el) => {
    if (el) {
      init();
    } else {
      dispose();
    }
  });

  // 窗口 resize 监听
  let resizeHandler: (() => void) | undefined;

  onMounted(() => {
    if (autoResize) {
      resizeHandler = () => resize();
      window.addEventListener('resize', resizeHandler);
    }
  });

  onBeforeUnmount(() => {
    if (resizeHandler) {
      window.removeEventListener('resize', resizeHandler);
    }
    if (autoDispose) {
      dispose();
    }
  });

  return {
    chartInstance,
    setOption,
    resize,
    dispose,
    getInstance,
    init,
  };
}

/**
 * 快捷方法：创建带有自动更新能力的图表。
 *
 * 用法：
 * ```ts
 * const chartRef = ref<HTMLElement>();
 * const data = ref({ x: [], y: [] });
 *
 * useChartWithAutoUpdate(chartRef, data, (d) => ({
 *   xAxis: { data: d.x },
 *   series: [{ data: d.y }],
 * }));
 * ```
 */
export function useChartWithAutoUpdate<T>(
  containerRef: Ref<HTMLElement | undefined>,
  data: Ref<T>,
  optionGetter: (data: T) => echarts.EChartsCoreOption,
  options: UseChartOptions = {}
) {
  const chart = useChart(containerRef, options);

  watch(
    data,
    (newData) => {
      if (newData) {
        chart.setOption(optionGetter(newData), true);
      }
    },
    { deep: true, immediate: true }
  );

  return chart;
}
