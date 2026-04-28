<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue';
import { useProjectStore } from '@/stores/project';
import { getDashboardOverview, type OverviewData } from '@/api/dashboard';
import * as echarts from 'echarts/core';
import { LineChart, PieChart, BarChart } from 'echarts/charts';
import {
  TitleComponent,
  TooltipComponent,
  GridComponent,
  LegendComponent,
} from 'echarts/components';
import { CanvasRenderer } from 'echarts/renderers';

echarts.use([
  LineChart,
  PieChart,
  BarChart,
  TitleComponent,
  TooltipComponent,
  GridComponent,
  LegendComponent,
  CanvasRenderer,
]);

const projectStore = useProjectStore();
const days = ref(7);
const loading = ref(false);
const data = ref<OverviewData | null>(null);

// ECharts 实例引用
const trendRef = ref<HTMLDivElement>();
const browserRef = ref<HTMLDivElement>();
const osRef = ref<HTMLDivElement>();
const deviceRef = ref<HTMLDivElement>();

let trendChart: echarts.ECharts | null = null;
let browserChart: echarts.ECharts | null = null;
let osChart: echarts.ECharts | null = null;
let deviceChart: echarts.ECharts | null = null;

async function fetchData() {
  if (!projectStore.currentId) return;
  loading.value = true;
  try {
    const res = await getDashboardOverview({
      project_id: projectStore.currentId,
      days: days.value,
    });
    data.value = res.data;
    await nextTick();
    renderCharts();
  } catch {
    // error message shown by request interceptor
  } finally {
    loading.value = false;
  }
}

function renderCharts() {
  if (!data.value) return;
  renderTrend();
  renderBrowser();
  renderOs();
  renderDevice();
}

function renderTrend() {
  if (!trendRef.value || !data.value) return;
  if (!trendChart) trendChart = echarts.init(trendRef.value);
  const trend = data.value.error_trend;
  trendChart.setOption(
    {
      tooltip: { trigger: 'axis' },
      grid: { left: 40, right: 20, top: 20, bottom: 30 },
      xAxis: { type: 'category', data: trend.map((t) => t.date), axisLabel: { fontSize: 11 } },
      yAxis: { type: 'value', minInterval: 1 },
      series: [
        {
          name: '错误数',
          type: 'line',
          data: trend.map((t) => t.count),
          smooth: true,
          areaStyle: { opacity: 0.15 },
          itemStyle: { color: '#f56c6c' },
        },
      ],
    },
    true
  );
}

function renderBrowser() {
  if (!browserRef.value || !data.value) return;
  if (!browserChart) browserChart = echarts.init(browserRef.value);
  browserChart.setOption(
    {
      tooltip: { trigger: 'item', formatter: '{b}: {c} ({d}%)' },
      legend: { orient: 'vertical', right: 10, top: 'center', textStyle: { fontSize: 11 } },
      series: [
        {
          type: 'pie',
          radius: ['40%', '70%'],
          center: ['38%', '50%'],
          data: data.value.browser_distribution,
          label: { show: false },
        },
      ],
    },
    true
  );
}

function renderOs() {
  if (!osRef.value || !data.value) return;
  if (!osChart) osChart = echarts.init(osRef.value);
  osChart.setOption(
    {
      tooltip: { trigger: 'item', formatter: '{b}: {c} ({d}%)' },
      legend: { orient: 'vertical', right: 10, top: 'center', textStyle: { fontSize: 11 } },
      series: [
        {
          type: 'pie',
          radius: ['40%', '70%'],
          center: ['38%', '50%'],
          data: data.value.os_distribution,
          label: { show: false },
        },
      ],
    },
    true
  );
}

function renderDevice() {
  if (!deviceRef.value || !data.value) return;
  if (!deviceChart) deviceChart = echarts.init(deviceRef.value);
  deviceChart.setOption(
    {
      tooltip: { trigger: 'axis' },
      grid: { left: 60, right: 20, top: 20, bottom: 30 },
      xAxis: {
        type: 'category',
        data: data.value.device_distribution.map((d) => d.name),
      },
      yAxis: { type: 'value', minInterval: 1 },
      series: [
        {
          type: 'bar',
          data: data.value.device_distribution.map((d) => d.value),
          itemStyle: { color: '#409eff' },
          barMaxWidth: 60,
        },
      ],
    },
    true
  );
}

function disposeCharts() {
  [trendChart, browserChart, osChart, deviceChart].forEach((c) => c?.dispose());
  trendChart = browserChart = osChart = deviceChart = null;
}

function onResize() {
  [trendChart, browserChart, osChart, deviceChart].forEach((c) => c?.resize());
}

onMounted(() => {
  fetchData();
  window.addEventListener('resize', onResize);
});

onUnmounted(() => {
  disposeCharts();
  window.removeEventListener('resize', onResize);
});

watch(() => projectStore.currentId, fetchData);
watch(days, fetchData);

const perf = computed(() => data.value?.avg_performance);
const fmt = (v: number | null | undefined, unit = 'ms') =>
  v == null ? '-' : `${Math.round(v)} ${unit}`;

const perfItems = [
  { key: 'fp', label: 'FP（首次绘制）' },
  { key: 'fcp', label: 'FCP（首次内容绘制）' },
  { key: 'lcp', label: 'LCP（最大内容绘制）' },
  { key: 'ttfb', label: 'TTFB（首字节时间）' },
];
</script>

<template>
  <div v-loading="loading" class="dashboard">
    <!-- 工具栏 -->
    <div class="dashboard__toolbar">
      <span class="dashboard__title">概览仪表盘</span>
      <el-radio-group v-model="days" size="small">
        <el-radio-button :label="7">近 7 天</el-radio-button>
        <el-radio-button :label="30">近 30 天</el-radio-button>
      </el-radio-group>
    </div>

    <!-- 数字卡片 -->
    <el-row :gutter="16" class="dashboard__stats">
      <el-col :span="6">
        <el-card shadow="never" class="stat-card stat-card--error">
          <div class="stat-card__value">{{ data?.total_errors ?? '-' }}</div>
          <div class="stat-card__label">JS 错误总数</div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="never" class="stat-card stat-card--network">
          <div class="stat-card__value">{{ data?.total_network_errors ?? '-' }}</div>
          <div class="stat-card__label">接口错误总数</div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="never" class="stat-card stat-card--perf">
          <div class="stat-card__value">{{ fmt(perf?.fcp) }}</div>
          <div class="stat-card__label">平均 FCP</div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="never" class="stat-card stat-card--lcp">
          <div class="stat-card__value">{{ fmt(perf?.lcp) }}</div>
          <div class="stat-card__label">平均 LCP</div>
        </el-card>
      </el-col>
    </el-row>

    <!-- 错误趋势 -->
    <el-card shadow="never" class="dashboard__chart-card">
      <template #header><span>错误趋势</span></template>
      <div ref="trendRef" class="chart chart--tall" />
    </el-card>

    <!-- 分布图 -->
    <el-row :gutter="16" class="dashboard__row">
      <el-col :span="12">
        <el-card shadow="never" class="dashboard__chart-card">
          <template #header><span>浏览器分布</span></template>
          <div ref="browserRef" class="chart" />
        </el-card>
      </el-col>
      <el-col :span="12">
        <el-card shadow="never" class="dashboard__chart-card">
          <template #header><span>操作系统分布</span></template>
          <div ref="osRef" class="chart" />
        </el-card>
      </el-col>
    </el-row>

    <el-row :gutter="16" class="dashboard__row">
      <el-col :span="10">
        <el-card shadow="never" class="dashboard__chart-card">
          <template #header><span>设备类型分布</span></template>
          <div ref="deviceRef" class="chart" />
        </el-card>
      </el-col>
      <el-col :span="14">
        <el-card shadow="never" class="dashboard__chart-card">
          <template #header><span>Top 错误</span></template>
          <el-table
            :data="data?.top_errors ?? []"
            size="small"
            :max-height="220"
            style="width: 100%"
          >
            <el-table-column label="错误信息" prop="message" show-overflow-tooltip />
            <el-table-column label="次数" prop="count" width="80" align="right" />
          </el-table>
        </el-card>
      </el-col>
    </el-row>

    <!-- 性能指标卡片 -->
    <el-card shadow="never" class="dashboard__chart-card">
      <template #header
        ><span>性能指标（近 {{ days }} 天平均）</span></template
      >
      <el-row :gutter="16">
        <el-col v-for="item in perfItems" :key="item.key" :span="6">
          <div class="perf-item">
            <div class="perf-item__value">{{ fmt(perf?.[item.key as keyof typeof perf]) }}</div>
            <div class="perf-item__label">{{ item.label }}</div>
          </div>
        </el-col>
      </el-row>
    </el-card>
  </div>
</template>

<style scoped lang="scss">
.dashboard {
  display: flex;
  flex-direction: column;
  gap: 16px;

  &__toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  &__title {
    font-size: 16px;
    font-weight: 600;
  }

  &__stats {
    margin-bottom: 0 !important;
  }

  &__chart-card {
    width: 100%;
  }

  &__row {
    margin-bottom: 0 !important;
  }
}

.stat-card {
  text-align: center;
  padding: 8px 0;

  &__value {
    font-size: 28px;
    font-weight: 700;
    line-height: 1.2;
  }

  &__label {
    font-size: 13px;
    color: var(--el-text-color-secondary);
    margin-top: 4px;
  }

  &--error .stat-card__value {
    color: #f56c6c;
  }
  &--network .stat-card__value {
    color: #e6a23c;
  }
  &--perf .stat-card__value {
    color: #409eff;
  }
  &--lcp .stat-card__value {
    color: #67c23a;
  }
}

.chart {
  height: 200px;

  &--tall {
    height: 240px;
  }
}

.perf-item {
  text-align: center;
  padding: 12px 0;

  &__value {
    font-size: 22px;
    font-weight: 600;
    color: #409eff;
  }

  &__label {
    font-size: 12px;
    color: var(--el-text-color-secondary);
    margin-top: 4px;
  }
}
</style>
