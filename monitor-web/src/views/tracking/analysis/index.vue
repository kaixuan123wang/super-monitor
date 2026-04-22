<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, nextTick } from 'vue';
import { useProjectStore } from '@/stores/project';
import { getEventAnalysis, listTrackEvents, type AnalysisSeries } from '@/api/tracking';
import * as echarts from 'echarts/core';
import { LineChart } from 'echarts/charts';
import {
  GridComponent,
  TooltipComponent,
  LegendComponent,
} from 'echarts/components';
import { CanvasRenderer } from 'echarts/renderers';

echarts.use([LineChart, GridComponent, TooltipComponent, LegendComponent, CanvasRenderer]);

const projectStore = useProjectStore();

const loading = ref(false);
const eventOptions = ref<string[]>([]);
const selectedEvents = ref<string[]>([]);
const days = ref(7);
const metric = ref<'pv' | 'uv'>('pv');
const groupBy = ref('');

const chartRef = ref<HTMLDivElement>();
let chart: echarts.ECharts | null = null;

const tableData = ref<{ name: string; data: number[]; total: number }[]>([]);
const dates = ref<string[]>([]);

const GROUP_OPTIONS = [
  { label: '不分组', value: '' },
  { label: '浏览器', value: 'browser' },
  { label: '操作系统', value: 'os' },
  { label: '设备类型', value: 'device_type' },
  { label: '环境', value: 'environment' },
];

async function loadEventOptions() {
  if (!projectStore.currentId) return;
  try {
    const res = await listTrackEvents({ project_id: projectStore.currentId, page_size: 100 });
    eventOptions.value = (res.data?.list ?? []).map((e) => e.event);
    if (eventOptions.value.length && !selectedEvents.value.length) {
      selectedEvents.value = [eventOptions.value[0]];
    }
  } catch { /* ignore */ }
}

async function query() {
  if (!projectStore.currentId || !selectedEvents.value.length) return;
  loading.value = true;
  try {
    const res = await getEventAnalysis({
      project_id: projectStore.currentId,
      events: selectedEvents.value.join(','),
      days: days.value,
      metric: metric.value,
      group_by: groupBy.value || undefined,
    });
    dates.value = res.data?.dates ?? [];
    const series: AnalysisSeries[] = res.data?.series ?? [];
    tableData.value = series.map((s) => ({
      name: s.name,
      data: s.data,
      total: s.data.reduce((a, b) => a + b, 0),
    }));
    await nextTick();
    renderChart(series);
  } finally {
    loading.value = false;
  }
}

function renderChart(series: AnalysisSeries[]) {
  if (!chartRef.value) return;
  if (!chart) chart = echarts.init(chartRef.value);
  chart.setOption({
    tooltip: { trigger: 'axis' },
    legend: { bottom: 0, type: 'scroll' },
    grid: { left: 50, right: 20, top: 20, bottom: 50 },
    xAxis: { type: 'category', data: dates.value, axisLabel: { fontSize: 11 } },
    yAxis: { type: 'value', minInterval: 1 },
    series: series.map((s) => ({
      name: s.name,
      type: 'line',
      data: s.data,
      smooth: true,
    })),
  });
}

onMounted(async () => {
  await loadEventOptions();
  if (selectedEvents.value.length) query();
});
onUnmounted(() => chart?.dispose());
watch(() => projectStore.currentId, async () => {
  await loadEventOptions();
  query();
});
</script>

<template>
  <div>
    <!-- 查询面板 -->
    <el-card shadow="never" style="margin-bottom: 16px">
      <el-form inline>
        <el-form-item label="选择事件">
          <el-select
            v-model="selectedEvents"
            multiple
            collapse-tags
            placeholder="请选择事件"
            style="width: 300px"
            filterable
          >
            <el-option v-for="e in eventOptions" :key="e" :label="e" :value="e" />
          </el-select>
        </el-form-item>
        <el-form-item label="时间范围">
          <el-radio-group v-model="days">
            <el-radio-button :label="7">近 7 天</el-radio-button>
            <el-radio-button :label="14">近 14 天</el-radio-button>
            <el-radio-button :label="30">近 30 天</el-radio-button>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="指标">
          <el-radio-group v-model="metric">
            <el-radio-button label="pv">PV（总次数）</el-radio-button>
            <el-radio-button label="uv">UV（独立用户）</el-radio-button>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="分组维度">
          <el-select v-model="groupBy" style="width: 130px">
            <el-option
              v-for="o in GROUP_OPTIONS"
              :key="o.value"
              :label="o.label"
              :value="o.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" :loading="loading" @click="query">查询</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <!-- 折线图 -->
    <el-card shadow="never" style="margin-bottom: 16px">
      <template #header><span>趋势图</span></template>
      <div ref="chartRef" style="height: 300px" />
    </el-card>

    <!-- 数据表格 -->
    <el-card shadow="never">
      <template #header><span>数据明细</span></template>
      <el-table :data="tableData" size="small" style="width: 100%">
        <el-table-column label="事件/维度" prop="name" min-width="160" />
        <el-table-column label="合计" prop="total" width="100" align="right" />
        <el-table-column
          v-for="(date, i) in dates"
          :key="date"
          :label="date.slice(5)"
          :prop="`data[${i}]`"
          width="80"
          align="right"
        >
          <template #default="{ row }">{{ row.data[i] ?? 0 }}</template>
        </el-table-column>
      </el-table>
    </el-card>
  </div>
</template>
