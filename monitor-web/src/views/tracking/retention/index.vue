<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from 'vue';
import { useProjectStore } from '@/stores/project';
import {
  listRetentions,
  createRetention,
  analyzeRetention,
  listTrackEvents,
  type TrackRetentionConfig,
  type RetentionTableRow,
} from '@/api/tracking';
import { ElMessage } from 'element-plus';
import * as echarts from 'echarts/core';
import { LineChart } from 'echarts/charts';
import { GridComponent, TooltipComponent, LegendComponent } from 'echarts/components';
import { CanvasRenderer } from 'echarts/renderers';

echarts.use([LineChart, GridComponent, TooltipComponent, LegendComponent, CanvasRenderer]);

const projectStore = useProjectStore();

const configs = ref<TrackRetentionConfig[]>([]);
const loading = ref(false);
const selected = ref<TrackRetentionConfig | null>(null);

const analyzeLoading = ref(false);
const tableData = ref<RetentionTableRow[]>([]);
const avgRetention = ref<number[]>([]);
const analyzeDays = ref(14);
const retentionType = ref<'day' | 'week'>('day');

const chartRef = ref<HTMLDivElement>();
let chart: echarts.ECharts | null = null;

const dialogVisible = ref(false);
const formName = ref('');
const formInitial = ref('');
const formReturn = ref('');
const formDays = ref(7);
const eventOptions = ref<string[]>([]);
const saving = ref(false);

async function fetchConfigs() {
  if (!projectStore.currentId) return;
  loading.value = true;
  try {
    const res = await listRetentions(projectStore.currentId);
    configs.value = res.data?.list ?? [];
  } finally {
    loading.value = false;
  }
}

async function fetchEventOptions() {
  if (!projectStore.currentId) return;
  const res = await listTrackEvents({ project_id: projectStore.currentId, page_size: 200 });
  eventOptions.value = (res.data?.list ?? []).map((e) => e.event);
}

async function doAnalyze(cfg: TrackRetentionConfig) {
  selected.value = cfg;
  analyzeLoading.value = true;
  tableData.value = [];
  avgRetention.value = [];
  try {
    const res = await analyzeRetention(cfg.id, {
      time_range: { days: analyzeDays.value },
      retention_type: retentionType.value,
    });
    tableData.value = res.data?.retention_table ?? [];
    avgRetention.value = res.data?.avg_retention ?? [];
    await nextTick();
    renderChart();
  } finally {
    analyzeLoading.value = false;
  }
}

function renderChart() {
  if (!chartRef.value || !avgRetention.value.length) return;
  if (!chart) chart = echarts.init(chartRef.value);

  const days = avgRetention.value.map((_, i) =>
    retentionType.value === 'week' ? `Week ${i + 1}` : `Day ${i + 1}`
  );
  chart.setOption(
    {
      tooltip: { trigger: 'axis' },
      xAxis: { type: 'category', data: days },
      yAxis: {
        type: 'value',
        min: 0,
        max: 1,
        axisLabel: { formatter: (v: number) => `${(v * 100).toFixed(0)}%` },
      },
      series: [
        {
          type: 'line',
          data: avgRetention.value,
          smooth: true,
          areaStyle: { opacity: 0.2 },
          label: {
            show: true,
            formatter: (p: { value: number }) => `${(p.value * 100).toFixed(1)}%`,
          },
        },
      ],
    },
    true
  );
}

onUnmounted(() => {
  chart?.dispose();
  chart = null;
});

function heatColor(rate: number): string {
  if (rate >= 0.5) return '#1a5c1a';
  if (rate >= 0.3) return '#4caf50';
  if (rate >= 0.15) return '#8bc34a';
  if (rate >= 0.08) return '#cddc39';
  if (rate >= 0.04) return '#ffeb3b';
  if (rate >= 0.02) return '#ffc107';
  return '#ff5722';
}

function openCreate() {
  formName.value = '';
  formInitial.value = '';
  formReturn.value = '';
  formDays.value = 7;
  dialogVisible.value = true;
}

async function saveRetention() {
  if (!formName.value || !formInitial.value || !formReturn.value) {
    ElMessage.warning('请填写完整');
    return;
  }
  saving.value = true;
  try {
    await createRetention({
      project_id: projectStore.currentId!,
      name: formName.value,
      initial_event: formInitial.value,
      return_event: formReturn.value,
      retention_days: formDays.value,
    });
    ElMessage.success('保存成功');
    dialogVisible.value = false;
    fetchConfigs();
  } finally {
    saving.value = false;
  }
}

const tableColumns = computed(() => {
  if (!tableData.value.length) return [];
  const keys = Object.keys(tableData.value[0]).filter(
    (k) => k.startsWith('day_') || k.startsWith('week_')
  );
  return keys;
});

onMounted(async () => {
  if (!projectStore.list.length) await projectStore.fetchAll();
  fetchConfigs();
  fetchEventOptions();
});

watch(
  () => projectStore.currentId,
  () => {
    fetchConfigs();
    fetchEventOptions();
  }
);
</script>

<template>
  <div style="display: flex; gap: 16px">
    <!-- 左侧：留存配置列表 -->
    <div style="width: 280px; flex-shrink: 0">
      <el-card shadow="never">
        <template #header>
          <div style="display: flex; justify-content: space-between; align-items: center">
            <span>留存配置</span>
            <el-button size="small" type="primary" @click="openCreate">新建</el-button>
          </div>
        </template>
        <div v-loading="loading">
          <el-empty v-if="!configs.length" description="暂无留存配置" />
          <div
            v-for="c in configs"
            :key="c.id"
            :class="['retention-item', { active: selected?.id === c.id }]"
            @click="doAnalyze(c)"
          >
            <div class="retention-name">{{ c.name }}</div>
            <div class="retention-meta">
              初始: {{ c.initial_event }}<br />
              回访: {{ c.return_event }} · {{ c.retention_days }}天
            </div>
          </div>
        </div>
      </el-card>
    </div>

    <!-- 右侧：分析结果 -->
    <div style="flex: 1; min-width: 0">
      <el-card shadow="never">
        <template #header>
          <div style="display: flex; justify-content: space-between; align-items: center">
            <span>{{ selected ? selected.name + ' — 留存分析' : '选择左侧配置查看留存' }}</span>
            <div v-if="selected" style="display: flex; align-items: center; gap: 8px">
              <el-select
                v-model="retentionType"
                style="width: 100px"
                @change="doAnalyze(selected!)"
              >
                <el-option label="按天" value="day" />
                <el-option label="按周" value="week" />
              </el-select>
              <el-select v-model="analyzeDays" style="width: 120px" @change="doAnalyze(selected!)">
                <el-option label="近 14 天" :value="14" />
                <el-option label="近 30 天" :value="30" />
              </el-select>
              <el-button type="primary" :loading="analyzeLoading" @click="doAnalyze(selected!)"
                >重新分析</el-button
              >
            </div>
          </div>
        </template>

        <el-empty v-if="!selected" description="从左侧选择留存配置" />
        <div v-else v-loading="analyzeLoading">
          <div v-if="tableData.length">
            <!-- 平均留存曲线 -->
            <div ref="chartRef" style="width: 100%; height: 280px; margin-bottom: 16px" />

            <!-- 热力矩阵表 -->
            <el-table :data="tableData" border size="small" max-height="400">
              <el-table-column prop="cohort_date" label="Cohort 日期" width="120" fixed />
              <el-table-column prop="cohort_size" label="人数" width="80" />
              <el-table-column
                v-for="col in tableColumns"
                :key="col"
                :label="
                  col.startsWith('week_')
                    ? col.replace('week_', 'Week ')
                    : col.replace('day_', 'Day ')
                "
                width="70"
              >
                <template #default="{ row }">
                  <div
                    v-if="row[col] != null"
                    :style="{
                      backgroundColor: heatColor(row[col] as number),
                      color: (row[col] as number) > 0.2 ? '#fff' : '#333',
                      textAlign: 'center',
                      padding: '4px 0',
                      borderRadius: '3px',
                      fontSize: '12px',
                    }"
                  >
                    {{ ((row[col] as number) * 100).toFixed(1) }}%
                  </div>
                  <span v-else style="color: #999">-</span>
                </template>
              </el-table-column>
            </el-table>
          </div>
          <el-empty v-else description="点击「重新分析」查看留存数据" />
        </div>
      </el-card>
    </div>

    <!-- 新建 dialog -->
    <el-dialog v-model="dialogVisible" title="新建留存配置" width="480px">
      <el-form label-width="100px">
        <el-form-item label="配置名称" required>
          <el-input v-model="formName" placeholder="例如：新用户 7 日留存" />
        </el-form-item>
        <el-form-item label="初始事件" required>
          <el-select
            v-model="formInitial"
            filterable
            allow-create
            placeholder="选择或输入事件名"
            style="width: 100%"
          >
            <el-option v-for="ev in eventOptions" :key="ev" :label="ev" :value="ev" />
          </el-select>
        </el-form-item>
        <el-form-item label="回访事件" required>
          <el-select
            v-model="formReturn"
            filterable
            allow-create
            placeholder="选择或输入事件名"
            style="width: 100%"
          >
            <el-option v-for="ev in eventOptions" :key="ev" :label="ev" :value="ev" />
          </el-select>
        </el-form-item>
        <el-form-item label="留存天数">
          <el-input-number v-model="formDays" :min="1" :max="90" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="saving" @click="saveRetention">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped>
.retention-item {
  padding: 10px 12px;
  border-radius: 6px;
  margin-bottom: 8px;
  cursor: pointer;
  border: 1px solid transparent;
  transition: all 0.2s;
}
.retention-item:hover {
  background: #f5f7fa;
}
.retention-item.active {
  background: #ecf5ff;
  border-color: #b3d8ff;
}
.retention-name {
  font-weight: 500;
  margin-bottom: 4px;
}
.retention-meta {
  font-size: 12px;
  color: #999;
  line-height: 1.5;
}
</style>
