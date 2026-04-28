<script setup lang="ts">
import { ref, reactive, onMounted, watch } from 'vue';
import { useProjectStore } from '@/stores/project';
import {
  listNetworkErrors,
  getNetworkStats,
  type NetworkErrorRow,
  type NetworkStats,
  type ListNetworkErrorsParams,
} from '@/api/network';
import { ElMessage } from 'element-plus';
import { truncate } from '@/utils/common';
import { useChart } from '@/composables/useChart';

const projectStore = useProjectStore();

const loading = ref(false);
const list = ref<NetworkErrorRow[]>([]);
const total = ref(0);

const filter = reactive<Omit<ListNetworkErrorsParams, 'project_id'>>({
  page: 1,
  page_size: 20,
  url: undefined,
  method: undefined,
  status: undefined,
  keyword: undefined,
});

const stats = ref<NetworkStats | null>(null);
const statsLoading = ref(false);
const days = ref(7);

const statusRef = ref<HTMLDivElement>();
const methodRef = ref<HTMLDivElement>();
const statusChart = useChart(statusRef);
const methodChart = useChart(methodRef);

async function reload() {
  if (!projectStore.currentId) {
    list.value = [];
    total.value = 0;
    return;
  }
  loading.value = true;
  try {
    const resp = await listNetworkErrors({
      project_id: projectStore.currentId,
      ...filter,
    });
    list.value = resp.data?.list ?? [];
    total.value = resp.data?.total ?? 0;
  } catch {
    // 全局拦截器已提示
  } finally {
    loading.value = false;
  }
}

async function fetchStats() {
  if (!projectStore.currentId) return;
  statsLoading.value = true;
  try {
    const res = await getNetworkStats(projectStore.currentId, days.value);
    stats.value = res.data ?? null;
    renderCharts();
  } finally {
    statsLoading.value = false;
  }
}

function renderCharts() {
  if (!stats.value) return;
  renderStatusChart();
  renderMethodChart();
}

function renderStatusChart() {
  if (!stats.value) return;
  const data = stats.value.status_distribution.map((d) => ({
    name: String(d.status),
    value: d.count,
  }));
  statusChart.setOption(
    {
      tooltip: { trigger: 'item' },
      legend: { bottom: 0, type: 'scroll' },
      series: [
        {
          type: 'pie',
          radius: ['40%', '70%'],
          data,
          label: { formatter: '{b}: {c} ({d}%)' },
        },
      ],
    },
    true
  );
}

function renderMethodChart() {
  if (!stats.value) return;
  const data = stats.value.method_distribution.slice(0, 8);
  methodChart.setOption(
    {
      tooltip: { trigger: 'axis' },
      grid: { left: 50, right: 20, top: 20, bottom: 30 },
      xAxis: { type: 'category', data: data.map((d) => d.method), axisLabel: { fontSize: 11 } },
      yAxis: { type: 'value', minInterval: 1 },
      series: [
        {
          type: 'bar',
          data: data.map((d) => d.count),
          itemStyle: { color: '#409eff' },
        },
      ],
    },
    true
  );
}

function onSearch() {
  filter.page = 1;
  reload();
}

function onReset() {
  filter.url = undefined;
  filter.method = undefined;
  filter.status = undefined;
  filter.keyword = undefined;
  filter.page = 1;
  reload();
}

function onPageChange(p: number) {
  filter.page = p;
  reload();
}

function onResize() {
  statusChart?.resize();
  methodChart?.resize();
}

function formatStatus(status?: number | null): string {
  if (status === undefined || status === null) return '-';
  if (status === 0) return '0 (网络错误)';
  return String(status);
}

function statusTagType(
  status?: number | null
): 'primary' | 'success' | 'warning' | 'info' | 'danger' {
  if (status === undefined || status === null) return 'info';
  if (status === 0) return 'danger';
  if (status >= 500) return 'danger';
  if (status >= 400) return 'warning';
  return 'success';
}

onMounted(() => {
  reload();
  fetchStats();
});

watch(
  () => projectStore.currentId,
  () => {
    reload();
    fetchStats();
  }
);
</script>

<template>
  <div>
    <!-- 统计卡片 -->
    <el-row :gutter="16" class="stats-row" v-loading="statsLoading">
      <el-col :span="6">
        <el-statistic title="总报错数" :value="stats?.total ?? 0" />
      </el-col>
      <el-col :span="6">
        <el-statistic title="平均耗时 (ms)" :value="stats?.avg_duration ?? 0" />
      </el-col>
      <el-col :span="6">
        <el-statistic title="Top URL 报错数" :value="stats?.top_urls?.[0]?.count ?? 0" />
      </el-col>
      <el-col :span="6">
        <el-statistic title="时间范围 (天)" :value="days" />
      </el-col>
    </el-row>

    <!-- 图表 -->
    <el-row :gutter="16" class="chart-row">
      <el-col :span="12">
        <el-card shadow="never">
          <template #header><span>状态码分布</span></template>
          <div ref="statusRef" class="chart-box" />
        </el-card>
      </el-col>
      <el-col :span="12">
        <el-card shadow="never">
          <template #header><span>请求方法分布</span></template>
          <div ref="methodRef" class="chart-box" />
        </el-card>
      </el-col>
    </el-row>

    <!-- 筛选 -->
    <el-card shadow="never" class="filter-card">
      <template #header><span>接口报错列表</span></template>
      <div class="toolbar">
        <el-input
          v-model="filter.keyword"
          placeholder="搜索 URL / 错误类型 / Body"
          clearable
          style="width: 260px"
          @keyup.enter="onSearch"
          @clear="onSearch"
        />
        <el-input
          v-model="filter.url"
          placeholder="URL 筛选"
          clearable
          style="width: 200px"
          @keyup.enter="onSearch"
          @clear="onSearch"
        />
        <el-select
          v-model="filter.method"
          placeholder="方法"
          clearable
          style="width: 100px"
          @change="onSearch"
        >
          <el-option label="GET" value="GET" />
          <el-option label="POST" value="POST" />
          <el-option label="PUT" value="PUT" />
          <el-option label="DELETE" value="DELETE" />
          <el-option label="PATCH" value="PATCH" />
        </el-select>
        <el-input-number
          v-model="filter.status"
          placeholder="状态码"
          :min="0"
          :max="599"
          style="width: 110px"
          @change="onSearch"
        />
        <el-button type="primary" @click="onSearch">搜索</el-button>
        <el-button @click="onReset">重置</el-button>
      </div>

      <el-table v-loading="loading" :data="list" style="width: 100%">
        <el-table-column label="URL" min-width="260" show-overflow-tooltip>
          <template #default="{ row }">
            <div class="url-cell">
              <el-tag size="small" :type="statusTagType(row.status)">{{ row.method }}</el-tag>
              <span class="url-text">{{ truncate(row.url, 80) }}</span>
            </div>
          </template>
        </el-table-column>
        <el-table-column label="状态码" width="100" align="center">
          <template #default="{ row }">
            <el-tag size="small" :type="statusTagType(row.status)">{{
              formatStatus(row.status)
            }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="耗时 (ms)" prop="duration" width="100" align="right" />
        <el-table-column label="错误类型" prop="error_type" width="120" show-overflow-tooltip />
        <el-table-column label="页面 URL" prop="page_url" min-width="180" show-overflow-tooltip />
        <el-table-column label="时间" prop="created_at" width="170">
          <template #default="{ row }">{{ new Date(row.created_at).toLocaleString() }}</template>
        </el-table-column>
      </el-table>

      <el-pagination
        v-if="total > filter.page_size!"
        v-model:current-page="filter.page"
        :page-size="filter.page_size"
        :total="total"
        layout="prev, pager, next, total"
        class="pagination"
        @current-change="onPageChange"
      />
    </el-card>
  </div>
</template>

<style scoped lang="scss">
.stats-row {
  margin-bottom: 16px;
}
.chart-row {
  margin-bottom: 16px;
}
.chart-box {
  width: 100%;
  height: 280px;
}
.filter-card {
  .toolbar {
    display: flex;
    gap: 8px;
    margin-bottom: 16px;
    flex-wrap: wrap;
  }
}
.url-cell {
  display: flex;
  align-items: center;
  gap: 8px;
}
.url-text {
  color: #606266;
  font-size: 13px;
}
.pagination {
  margin-top: 16px;
  justify-content: flex-end;
}
</style>
