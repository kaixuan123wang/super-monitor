<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, nextTick } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useProjectStore } from '@/stores/project';
import { getEventDetail, type EventDetail } from '@/api/tracking';
import * as echarts from 'echarts/core';
import { LineChart } from 'echarts/charts';
import { GridComponent, TooltipComponent } from 'echarts/components';
import { CanvasRenderer } from 'echarts/renderers';

echarts.use([LineChart, GridComponent, TooltipComponent, CanvasRenderer]);

const route = useRoute();
const router = useRouter();
const projectStore = useProjectStore();

const loading = ref(false);
const detail = ref<EventDetail | null>(null);
const trendRef = ref<HTMLDivElement>();
let chart: echarts.ECharts | null = null;

function getEventName(): string {
  return decodeURIComponent(route.params.eventName as string);
}

async function fetchDetail() {
  if (!projectStore.currentId) return;
  loading.value = true;
  try {
    const res = await getEventDetail(getEventName(), projectStore.currentId);
    detail.value = res.data;
    await nextTick();
    renderChart();
  } finally {
    loading.value = false;
  }
}

function renderChart() {
  if (!trendRef.value || !detail.value) return;
  if (!chart) chart = echarts.init(trendRef.value);
  chart.setOption({
    tooltip: { trigger: 'axis' },
    grid: { left: 40, right: 20, top: 10, bottom: 30 },
    xAxis: {
      type: 'category',
      data: detail.value.trend.map((t) => t.date),
      axisLabel: { fontSize: 11 },
    },
    yAxis: { type: 'value', minInterval: 1 },
    series: [
      {
        type: 'line',
        data: detail.value.trend.map((t) => t.count),
        smooth: true,
        areaStyle: { opacity: 0.15 },
        itemStyle: { color: '#409eff' },
      },
    ],
  });
}

onMounted(() => {
  fetchDetail();
  window.addEventListener('resize', onResize);
});
onUnmounted(() => {
  chart?.dispose();
  chart = null;
  window.removeEventListener('resize', onResize);
});

watch(() => projectStore.currentId, fetchDetail);
watch(() => route.params.eventName, fetchDetail);

function onResize() {
  chart?.resize();
}
</script>

<template>
  <div v-loading="loading">
    <div style="margin-bottom: 16px">
      <el-button link @click="router.push('/tracking/events')">← 返回事件列表</el-button>
    </div>

    <el-descriptions v-if="detail" :title="detail.event" :column="3" border>
      <el-descriptions-item label="事件名">{{ detail.event }}</el-descriptions-item>
      <el-descriptions-item label="总次数">{{ detail.total_count }}</el-descriptions-item>
      <el-descriptions-item label="独立用户">{{ detail.unique_users }}</el-descriptions-item>
    </el-descriptions>

    <el-card shadow="never" style="margin-top: 16px">
      <template #header><span>近 7 天趋势</span></template>
      <div ref="trendRef" style="height: 200px" />
    </el-card>

    <el-card shadow="never" style="margin-top: 16px">
      <template #header><span>属性列表</span></template>
      <template v-if="detail?.properties?.length">
        <el-tag v-for="prop in detail.properties" :key="prop" style="margin: 4px" size="small">
          {{ prop }}
        </el-tag>
      </template>
      <el-empty v-else description="暂无属性数据" />
    </el-card>
  </div>
</template>
