<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from 'vue';
import { useProjectStore } from '@/stores/project';
import {
  listFunnels,
  createFunnel,
  updateFunnel,
  deleteFunnel,
  analyzeFunnel,
  listTrackEvents,
  type TrackFunnel,
  type FunnelStep,
  type FunnelStepResult,
} from '@/api/tracking';
import { ElMessage, ElMessageBox } from 'element-plus';
import * as echarts from 'echarts/core';
import { FunnelChart } from 'echarts/charts';
import { TooltipComponent, LegendComponent } from 'echarts/components';
import { CanvasRenderer } from 'echarts/renderers';

echarts.use([FunnelChart, TooltipComponent, LegendComponent, CanvasRenderer]);

const projectStore = useProjectStore();

const funnels = ref<TrackFunnel[]>([]);
const loading = ref(false);
const selected = ref<TrackFunnel | null>(null);

const analyzeLoading = ref(false);
const analyzeResult = ref<FunnelStepResult[]>([]);
const overallConversion = ref(0);
const analyzeDays = ref(7);
const groupBy = ref('');
const breakdown = ref<
  Array<{ group: string; steps: FunnelStepResult[]; overall_conversion: number }>
>([]);

const chartRef = ref<HTMLDivElement>();
let chart: echarts.ECharts | null = null;

const dialogVisible = ref(false);
const editingId = ref<number | null>(null);
const formName = ref('');
const formSteps = ref<FunnelStep[]>([{ event: '', display_name: '' }]);
const formWindow = ref(1440);
const eventOptions = ref<string[]>([]);
const saving = ref(false);

async function fetchFunnels() {
  if (!projectStore.currentId) return;
  loading.value = true;
  try {
    const res = await listFunnels(projectStore.currentId);
    funnels.value = res.data?.list ?? [];
  } finally {
    loading.value = false;
  }
}

async function fetchEventOptions() {
  if (!projectStore.currentId) return;
  const res = await listTrackEvents({ project_id: projectStore.currentId, page_size: 200 });
  eventOptions.value = (res.data?.list ?? []).map((e) => e.event);
}

async function doAnalyze(funnel: TrackFunnel) {
  selected.value = funnel;
  analyzeLoading.value = true;
  analyzeResult.value = [];
  breakdown.value = [];
  try {
    const res = await analyzeFunnel(funnel.id, {
      time_range: { days: analyzeDays.value },
      group_by: groupBy.value || undefined,
    });
    analyzeResult.value = res.data?.steps ?? [];
    overallConversion.value = res.data?.overall_conversion ?? 0;
    breakdown.value = res.data?.breakdown ?? [];
    await nextTick();
    renderChart();
  } finally {
    analyzeLoading.value = false;
  }
}

function renderChart() {
  if (!chartRef.value || !analyzeResult.value.length) return;
  if (!chart) chart = echarts.init(chartRef.value);

  const data = analyzeResult.value.map((s) => ({
    name: s.display_name || s.event,
    value: s.user_count,
  }));

  chart.setOption(
    {
      tooltip: { trigger: 'item', formatter: '{b}: {c} 人 ({d}%)' },
      series: [
        {
          type: 'funnel',
          left: '10%',
          width: '80%',
          data,
          label: {
            formatter: (p: { name: string; value: number }) => `${p.name}\n${p.value} 人`,
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

const breakdownStepLabels = computed(() =>
  analyzeResult.value.map((s) => s.display_name || s.event)
);

function addStep() {
  formSteps.value.push({ event: '', display_name: '' });
}

function removeStep(idx: number) {
  formSteps.value.splice(idx, 1);
}

function openCreate() {
  editingId.value = null;
  formName.value = '';
  formSteps.value = [{ event: '' }, { event: '' }];
  formWindow.value = 1440;
  dialogVisible.value = true;
}

function openEdit(f: TrackFunnel) {
  editingId.value = f.id;
  formName.value = f.name;
  formSteps.value = f.steps.map((s) => ({ ...s }));
  formWindow.value = f.window_minutes;
  dialogVisible.value = true;
}

async function saveFunnel() {
  if (!formName.value) {
    ElMessage.warning('请填写漏斗名称');
    return;
  }
  const steps = formSteps.value.filter((s) => s.event);
  if (steps.length < 2) {
    ElMessage.warning('漏斗至少需要 2 个步骤');
    return;
  }

  saving.value = true;
  try {
    if (editingId.value) {
      await updateFunnel(editingId.value, {
        name: formName.value,
        steps,
        window_minutes: formWindow.value,
      });
    } else {
      await createFunnel({
        project_id: projectStore.currentId!,
        name: formName.value,
        steps,
        window_minutes: formWindow.value,
      });
    }
    ElMessage.success('保存成功');
    dialogVisible.value = false;
    fetchFunnels();
  } finally {
    saving.value = false;
  }
}

async function handleDelete(f: TrackFunnel) {
  try {
    await ElMessageBox.confirm(`确认删除漏斗「${f.name}」？`, '确认', { type: 'warning' });
    await deleteFunnel(f.id);
    ElMessage.success('已删除');
    if (selected.value?.id === f.id) {
      selected.value = null;
      analyzeResult.value = [];
    }
    fetchFunnels();
  } catch {
    // 用户取消或请求失败
  }
}

onMounted(async () => {
  if (!projectStore.list.length) await projectStore.fetchAll();
  fetchFunnels();
  fetchEventOptions();
});

watch(
  () => projectStore.currentId,
  () => {
    fetchFunnels();
    fetchEventOptions();
  }
);
</script>

<template>
  <div style="display: flex; gap: 16px">
    <!-- 左侧：漏斗列表 -->
    <div style="width: 280px; flex-shrink: 0">
      <el-card shadow="never">
        <template #header>
          <div style="display: flex; justify-content: space-between; align-items: center">
            <span>漏斗列表</span>
            <el-button size="small" type="primary" @click="openCreate">新建</el-button>
          </div>
        </template>
        <div v-loading="loading">
          <el-empty v-if="!funnels.length" description="暂无漏斗" />
          <div
            v-for="f in funnels"
            :key="f.id"
            :class="['funnel-item', { active: selected?.id === f.id }]"
            @click="doAnalyze(f)"
          >
            <div class="funnel-name">{{ f.name }}</div>
            <div class="funnel-meta">
              {{ f.steps.length }} 步骤 · {{ f.window_minutes }} 分钟窗口
            </div>
            <div class="funnel-actions">
              <el-button link size="small" @click.stop="openEdit(f)">编辑</el-button>
              <el-button link size="small" type="danger" @click.stop="handleDelete(f)"
                >删除</el-button
              >
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
            <span>{{ selected ? selected.name + ' — 漏斗分析' : '选择左侧漏斗查看分析' }}</span>
            <div v-if="selected" style="display: flex; align-items: center; gap: 8px">
              <el-select
                v-model="groupBy"
                clearable
                placeholder="分组"
                style="width: 120px"
                @change="doAnalyze(selected!)"
              >
                <el-option label="浏览器" value="browser" />
                <el-option label="系统" value="os" />
                <el-option label="设备" value="device_type" />
                <el-option label="环境" value="environment" />
                <el-option label="版本" value="release" />
              </el-select>
              <el-select v-model="analyzeDays" style="width: 120px" @change="doAnalyze(selected!)">
                <el-option label="近 7 天" :value="7" />
                <el-option label="近 14 天" :value="14" />
                <el-option label="近 30 天" :value="30" />
              </el-select>
              <el-button type="primary" :loading="analyzeLoading" @click="doAnalyze(selected!)"
                >重新分析</el-button
              >
            </div>
          </div>
        </template>

        <el-empty v-if="!selected" description="从左侧选择漏斗" />
        <div v-else v-loading="analyzeLoading">
          <div v-if="analyzeResult.length">
            <!-- 整体转化率 -->
            <el-alert
              :title="`整体转化率：${(overallConversion * 100).toFixed(1)}%`"
              type="info"
              show-icon
              :closable="false"
              style="margin-bottom: 16px"
            />

            <!-- 漏斗图 -->
            <div ref="chartRef" style="width: 100%; height: 300px; margin-bottom: 16px" />

            <!-- 步骤明细表 -->
            <el-table :data="analyzeResult" border size="small">
              <el-table-column type="index" label="步骤" width="60" />
              <el-table-column label="事件" min-width="150">
                <template #default="{ row }">{{ row.display_name || row.event }}</template>
              </el-table-column>
              <el-table-column prop="user_count" label="用户数" width="100" />
              <el-table-column label="转化率" width="100">
                <template #default="{ row }">
                  <span :style="{ color: row.conversion_rate < 0.3 ? '#f56c6c' : '#67c23a' }">
                    {{ (row.conversion_rate * 100).toFixed(1) }}%
                  </span>
                </template>
              </el-table-column>
              <el-table-column label="平均转化时长" width="130">
                <template #default="{ row }">
                  {{
                    row.avg_time_to_next_ms
                      ? (row.avg_time_to_next_ms / 1000).toFixed(1) + ' 秒'
                      : '-'
                  }}
                </template>
              </el-table-column>
            </el-table>

            <el-table
              v-if="breakdown.length"
              :data="breakdown"
              border
              size="small"
              style="margin-top: 16px"
            >
              <el-table-column prop="group" label="分组" width="120" />
              <el-table-column label="整体转化率" width="120">
                <template #default="{ row }"
                  >{{ (row.overall_conversion * 100).toFixed(1) }}%</template
                >
              </el-table-column>
              <el-table-column
                v-for="(_, idx) in breakdownStepLabels"
                :key="idx"
                :label="breakdownStepLabels[idx]"
                min-width="100"
              >
                <template #default="{ row }">
                  {{ row.steps[idx] ? `${row.steps[idx].user_count} 人` : '-' }}
                </template>
              </el-table-column>
            </el-table>
          </div>
          <el-empty v-else description="点击「重新分析」查看漏斗数据" />
        </div>
      </el-card>
    </div>

    <!-- 创建/编辑 dialog -->
    <el-dialog v-model="dialogVisible" :title="editingId ? '编辑漏斗' : '新建漏斗'" width="560px">
      <el-form label-width="100px">
        <el-form-item label="漏斗名称" required>
          <el-input v-model="formName" />
        </el-form-item>
        <el-form-item label="转化窗口">
          <el-input-number v-model="formWindow" :min="1" :max="43200" />
          <span style="margin-left: 8px; color: #999">分钟</span>
        </el-form-item>
        <el-form-item label="步骤">
          <div
            v-for="(step, idx) in formSteps"
            :key="idx"
            style="display: flex; align-items: center; gap: 8px; margin-bottom: 8px"
          >
            <el-tag type="info" size="small" style="flex-shrink: 0">步骤 {{ idx + 1 }}</el-tag>
            <el-select
              v-model="step.event"
              filterable
              allow-create
              placeholder="选择或输入事件名"
              style="flex: 1"
            >
              <el-option v-for="ev in eventOptions" :key="ev" :label="ev" :value="ev" />
            </el-select>
            <el-input
              v-model="step.display_name"
              placeholder="展示名（可选）"
              style="width: 120px"
            />
            <el-button :disabled="formSteps.length <= 2" link type="danger" @click="removeStep(idx)"
              >删除</el-button
            >
          </div>
          <el-button size="small" @click="addStep">+ 添加步骤</el-button>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="saving" @click="saveFunnel">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped>
.funnel-item {
  padding: 10px 12px;
  border-radius: 6px;
  margin-bottom: 8px;
  cursor: pointer;
  border: 1px solid transparent;
  transition: all 0.2s;
}
.funnel-item:hover {
  background: #f5f7fa;
}
.funnel-item.active {
  background: #ecf5ff;
  border-color: #b3d8ff;
}
.funnel-name {
  font-weight: 500;
  margin-bottom: 4px;
}
.funnel-meta {
  font-size: 12px;
  color: #999;
}
.funnel-actions {
  margin-top: 6px;
}
</style>
