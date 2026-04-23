<script setup lang="ts">
import { ref, onMounted, watch } from 'vue';
import { useProjectStore } from '@/stores/project';
import { listAnalyses, triggerAnalysis, type AiAnalysis } from '@/api/ai';
import { listErrors, type JsErrorRow } from '@/api/error';
import { ElMessage, ElMessageBox } from 'element-plus';

const projectStore = useProjectStore();

const loading = ref(false);
const list = ref<AiAnalysis[]>([]);
const total = ref(0);
const page = ref(1);
const pageSize = ref(20);

const errorList = ref<JsErrorRow[]>([]);
const errorLoading = ref(false);
const selectedErrorId = ref<number | null>(null);
const triggerLoading = ref(false);

const detailVisible = ref(false);
const selected = ref<AiAnalysis | null>(null);

async function fetchList() {
  if (!projectStore.currentId) return;
  loading.value = true;
  try {
    const res = await listAnalyses({ project_id: projectStore.currentId, page: page.value, page_size: pageSize.value });
    list.value = res.data?.list ?? [];
    total.value = res.data?.total ?? 0;
  } finally {
    loading.value = false;
  }
}

async function fetchErrors() {
  if (!projectStore.currentId) return;
  errorLoading.value = true;
  try {
    const res = await listErrors({ project_id: projectStore.currentId, page: 1, page_size: 50 });
    errorList.value = res.data?.list ?? [];
  } finally {
    errorLoading.value = false;
  }
}

async function doTrigger() {
  if (!selectedErrorId.value) {
    ElMessage.warning('请先选择一条错误');
    return;
  }
  triggerLoading.value = true;
  try {
    await triggerAnalysis(selectedErrorId.value);
    ElMessage.success('AI 分析已触发，请稍后刷新查看结果');
    setTimeout(() => fetchList(), 3000);
  } finally {
    triggerLoading.value = false;
  }
}

function showDetail(row: AiAnalysis) {
  selected.value = row;
  detailVisible.value = true;
}

function severityTag(score: number | null): 'danger' | 'warning' | 'info' | 'success' {
  if (!score) return 'info';
  if (score >= 4) return 'danger';
  if (score >= 3) return 'warning';
  return 'success';
}

function statusTag(status: string): 'success' | 'warning' | 'danger' | 'info' {
  if (status === 'success') return 'success';
  if (status === 'pending') return 'warning';
  if (status === 'failed') return 'danger';
  return 'info';
}

function truncate(s: string | null | undefined, n = 80) {
  if (!s) return '';
  return s.length > n ? s.slice(0, n) + '…' : s;
}

onMounted(async () => {
  if (!projectStore.list.length) await projectStore.fetchAll();
  fetchList();
  fetchErrors();
});

watch(() => projectStore.currentId, () => {
  page.value = 1;
  fetchList();
  fetchErrors();
});
</script>

<template>
  <div class="ai-analysis-page">
    <!-- 触发区 -->
    <el-card shadow="never" style="margin-bottom:16px">
      <template #header>
        <span>触发 AI 分析</span>
      </template>
      <el-form inline>
        <el-form-item label="选择错误">
          <el-select
            v-model="selectedErrorId"
            placeholder="从错误列表选择"
            filterable
            :loading="errorLoading"
            style="width:400px"
          >
            <el-option
              v-for="e in errorList"
              :key="e.id"
              :label="`#${e.id} ${truncate(e.message, 60)}`"
              :value="e.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" :loading="triggerLoading" @click="doTrigger">
            触发 AI 分析
          </el-button>
          <el-button @click="fetchList">刷新列表</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <!-- 分析历史列表 -->
    <el-card shadow="never">
      <template #header>
        <span>AI 分析历史</span>
        <span style="float:right;color:#999;font-size:13px">共 {{ total }} 条</span>
      </template>

      <el-table :data="list" v-loading="loading" border>
        <el-table-column prop="error_id" label="错误 ID" width="90" />
        <el-table-column label="状态" width="90">
          <template #default="{ row }">
            <el-tag :type="statusTag(row.status)" size="small">{{ row.status }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="严重度" width="80">
          <template #default="{ row }">
            <el-tag v-if="row.severity_score" :type="severityTag(row.severity_score)" size="small">
              P{{ row.severity_score }}
            </el-tag>
            <span v-else>-</span>
          </template>
        </el-table-column>
        <el-table-column label="置信度" width="80">
          <template #default="{ row }">
            {{ row.confidence != null ? (row.confidence * 100).toFixed(0) + '%' : '-' }}
          </template>
        </el-table-column>
        <el-table-column label="AI 建议摘要" min-width="200">
          <template #default="{ row }">
            {{ truncate(row.ai_suggestion, 100) || '-' }}
          </template>
        </el-table-column>
        <el-table-column label="模型" prop="model_used" width="130" />
        <el-table-column label="耗时(ms)" prop="cost_ms" width="90" />
        <el-table-column label="缓存" width="70">
          <template #default="{ row }">
            <el-icon v-if="row.is_cached" color="#67c23a"><Check /></el-icon>
          </template>
        </el-table-column>
        <el-table-column label="时间" width="170">
          <template #default="{ row }">{{ row.created_at?.slice(0, 19).replace('T', ' ') }}</template>
        </el-table-column>
        <el-table-column label="操作" width="80" fixed="right">
          <template #default="{ row }">
            <el-button link type="primary" @click="showDetail(row)">详情</el-button>
          </template>
        </el-table-column>
      </el-table>

      <div style="margin-top:12px;text-align:right">
        <el-pagination
          v-model:current-page="page"
          :page-size="pageSize"
          :total="total"
          layout="total, prev, pager, next"
          @current-change="fetchList"
        />
      </div>
    </el-card>

    <!-- 详情抽屉 -->
    <el-drawer v-model="detailVisible" title="AI 分析详情" size="50%">
      <template v-if="selected">
        <el-descriptions :column="2" border size="small" style="margin-bottom:16px">
          <el-descriptions-item label="错误 ID">{{ selected.error_id }}</el-descriptions-item>
          <el-descriptions-item label="状态">
            <el-tag :type="statusTag(selected.status)" size="small">{{ selected.status }}</el-tag>
          </el-descriptions-item>
          <el-descriptions-item label="严重度">P{{ selected.severity_score ?? '-' }}</el-descriptions-item>
          <el-descriptions-item label="置信度">
            {{ selected.confidence != null ? (selected.confidence * 100).toFixed(0) + '%' : '-' }}
          </el-descriptions-item>
          <el-descriptions-item label="可能文件">{{ selected.probable_file ?? '-' }}</el-descriptions-item>
          <el-descriptions-item label="可能行号">{{ selected.probable_line ?? '-' }}</el-descriptions-item>
          <el-descriptions-item label="模型">{{ selected.model_used ?? '-' }}</el-descriptions-item>
          <el-descriptions-item label="耗时">{{ selected.cost_ms ?? '-' }} ms</el-descriptions-item>
        </el-descriptions>

        <div v-if="selected.tags?.length" style="margin-bottom:12px">
          <el-tag v-for="tag in selected.tags" :key="tag" style="margin-right:6px">{{ tag }}</el-tag>
        </div>

        <el-collapse accordion>
          <el-collapse-item title="AI 分析建议" name="suggestion">
            <pre style="white-space:pre-wrap;font-size:13px;line-height:1.6">{{ selected.ai_suggestion || '暂无' }}</pre>
          </el-collapse-item>
          <el-collapse-item v-if="selected.analyzed_stack" title="Source Map 还原堆栈" name="stack">
            <pre style="white-space:pre-wrap;font-size:12px;font-family:monospace">{{ selected.analyzed_stack }}</pre>
          </el-collapse-item>
        </el-collapse>
      </template>
    </el-drawer>
  </div>
</template>

<style scoped>
.ai-analysis-page { padding: 0; }
</style>
