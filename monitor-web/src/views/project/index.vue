<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import { useProjectStore } from '@/stores/project';
import {
  createProject,
  deleteProject,
  updateProject,
  type CreateProjectBody,
  type Project,
  type UpdateProjectBody,
} from '@/api/project';

const projectStore = useProjectStore();
const keyword = ref('');
const loading = ref(false);

// ============ 新建 / 编辑 对话框 ============
const dialogVisible = ref(false);
const editing = ref<Project | null>(null);
const form = reactive({
  name: '',
  description: '',
  environment: 'production',
  alert_threshold: 10,
  data_retention_days: 30,
});

function resetForm() {
  form.name = '';
  form.description = '';
  form.environment = 'production';
  form.alert_threshold = 10;
  form.data_retention_days = 30;
  editing.value = null;
}

function openCreate() {
  resetForm();
  dialogVisible.value = true;
}

function openEdit(row: Project) {
  editing.value = row;
  form.name = row.name;
  form.description = row.description || '';
  form.environment = row.environment;
  form.alert_threshold = row.alert_threshold;
  form.data_retention_days = row.data_retention_days;
  dialogVisible.value = true;
}

async function submitForm() {
  if (!form.name.trim()) {
    ElMessage.warning('项目名称不能为空');
    return;
  }
  try {
    if (editing.value) {
      const body: UpdateProjectBody = {
        name: form.name,
        description: form.description,
        environment: form.environment,
        alert_threshold: form.alert_threshold,
        data_retention_days: form.data_retention_days,
      };
      const resp = await updateProject(editing.value.id, body);
      if (resp.data) projectStore.upsert(resp.data);
      ElMessage.success('项目已更新');
    } else {
      const body: CreateProjectBody = {
        name: form.name,
        description: form.description,
        environment: form.environment,
        alert_threshold: form.alert_threshold,
        data_retention_days: form.data_retention_days,
      };
      const resp = await createProject(body);
      if (resp.data) projectStore.upsert(resp.data);
      ElMessage.success('项目已创建');
    }
    dialogVisible.value = false;
  } catch (e) {
    // 全局拦截器已提示
  }
}

// ============ 删除 ============
async function handleDelete(row: Project) {
  try {
    await ElMessageBox.confirm(`确定删除项目 "${row.name}" 吗？`, '删除确认', {
      type: 'warning',
      confirmButtonText: '删除',
      cancelButtonText: '取消',
    });
    await deleteProject(row.id);
    projectStore.remove(row.id);
    ElMessage.success('已删除');
  } catch {
    /* cancel */
  }
}

// ============ SDK 接入代码 ============
const snippetDialogVisible = ref(false);
const snippetProject = ref<Project | null>(null);

function showSnippet(row: Project) {
  snippetProject.value = row;
  snippetDialogVisible.value = true;
}

function buildSnippet(p: Project): string {
  const origin = window.location.origin;
  return (
    `<!-- 监控 SDK 接入示例 -->
<script src="${origin}/sdk.iife.js"></` +
    `script>
<script>
  Monitor.init({
    appId: '${p.app_id}',
    appKey: '${p.app_key}',
    server: '${origin}',
    environment: '${p.environment}',
    debug: false,
  });
</` +
    `script>`
  );
}

function copySnippet() {
  if (!snippetProject.value) return;
  const text = buildSnippet(snippetProject.value);
  if (navigator.clipboard?.writeText) {
    navigator.clipboard
      .writeText(text)
      .then(() => {
        ElMessage.success('接入代码已复制到剪贴板');
      })
      .catch(() => {
        fallbackCopy(text);
      });
  } else {
    fallbackCopy(text);
  }
}

function fallbackCopy(text: string) {
  const ta = document.createElement('textarea');
  ta.value = text;
  ta.style.position = 'fixed';
  ta.style.opacity = '0';
  document.body.appendChild(ta);
  ta.select();
  try {
    document.execCommand('copy');
    ElMessage.success('接入代码已复制到剪贴板');
  } catch {
    ElMessage.error('复制失败，请手动复制');
  }
  document.body.removeChild(ta);
}

// ============ 切换当前项目 ============
function handleSetCurrent(row: Project) {
  projectStore.setCurrent(row.id);
  ElMessage.success(`已切换到：${row.name}`);
}

// ============ 加载 ============
async function reload() {
  loading.value = true;
  try {
    await projectStore.fetchAll(keyword.value || undefined);
  } finally {
    loading.value = false;
  }
}

onMounted(reload);
</script>

<template>
  <el-card shadow="never">
    <template #header>
      <div class="project-list__header">
        <span>项目管理</span>
        <div class="project-list__actions">
          <el-input
            v-model="keyword"
            placeholder="搜索项目名"
            clearable
            style="width: 220px"
            @keyup.enter="reload"
            @clear="reload"
          />
          <el-button type="primary" @click="openCreate">新建项目</el-button>
        </div>
      </div>
    </template>

    <el-table :data="projectStore.list" v-loading="loading" border stripe>
      <el-table-column prop="id" label="ID" width="70" />
      <el-table-column prop="name" label="名称" min-width="160">
        <template #default="{ row }">
          <div class="project-list__name">
            <el-tag v-if="row.id === projectStore.currentId" type="success" size="small"
              >当前</el-tag
            >
            {{ row.name }}
          </div>
        </template>
      </el-table-column>
      <el-table-column prop="app_id" label="App ID" min-width="240">
        <template #default="{ row }">
          <code class="project-list__code">{{ row.app_id }}</code>
        </template>
      </el-table-column>
      <el-table-column prop="environment" label="环境" width="110">
        <template #default="{ row }">
          <el-tag :type="row.environment === 'production' ? 'danger' : 'info'" size="small">
            {{ row.environment }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="data_retention_days" label="保留天数" width="100" align="center" />
      <el-table-column prop="alert_threshold" label="告警阈值" width="100" align="center" />
      <el-table-column prop="created_at" label="创建时间" width="180">
        <template #default="{ row }">
          {{ new Date(row.created_at).toLocaleString() }}
        </template>
      </el-table-column>
      <el-table-column label="操作" width="280" fixed="right">
        <template #default="{ row }">
          <el-button link size="small" @click="handleSetCurrent(row)">切换</el-button>
          <el-button link size="small" type="primary" @click="showSnippet(row)">接入代码</el-button>
          <el-button link size="small" @click="openEdit(row)">编辑</el-button>
          <el-button link size="small" type="danger" @click="handleDelete(row)">删除</el-button>
        </template>
      </el-table-column>
    </el-table>
  </el-card>

  <!-- 新建 / 编辑 对话框 -->
  <el-dialog v-model="dialogVisible" :title="editing ? '编辑项目' : '新建项目'" width="500">
    <el-form :model="form" label-width="100px">
      <el-form-item label="项目名称" required>
        <el-input v-model="form.name" placeholder="例如：官网" />
      </el-form-item>
      <el-form-item label="描述">
        <el-input v-model="form.description" type="textarea" :rows="2" />
      </el-form-item>
      <el-form-item label="环境">
        <el-select v-model="form.environment">
          <el-option label="production" value="production" />
          <el-option label="staging" value="staging" />
          <el-option label="development" value="development" />
        </el-select>
      </el-form-item>
      <el-form-item label="告警阈值">
        <el-input-number v-model="form.alert_threshold" :min="1" />
      </el-form-item>
      <el-form-item label="数据保留">
        <el-input-number v-model="form.data_retention_days" :min="1" :max="365" />
        <span style="margin-left: 8px; color: var(--el-text-color-secondary)">天</span>
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="dialogVisible = false">取消</el-button>
      <el-button type="primary" @click="submitForm">{{ editing ? '保存' : '创建' }}</el-button>
    </template>
  </el-dialog>

  <!-- SDK 接入代码 -->
  <el-dialog v-model="snippetDialogVisible" title="SDK 接入代码" width="640">
    <template v-if="snippetProject">
      <p class="project-list__tip">
        将下面的代码片段加入到目标网站的
        <code>&lt;head&gt;</code> 中即可开始采集。
      </p>
      <el-input type="textarea" :model-value="buildSnippet(snippetProject)" :rows="10" readonly />
      <div class="project-list__snippet-meta">
        <div>
          <strong>App ID：</strong><code>{{ snippetProject.app_id }}</code>
        </div>
        <div>
          <strong>App Key：</strong><code>{{ snippetProject.app_key }}</code>
        </div>
      </div>
    </template>
    <template #footer>
      <el-button @click="snippetDialogVisible = false">关闭</el-button>
      <el-button type="primary" @click="copySnippet">复制代码</el-button>
    </template>
  </el-dialog>
</template>

<style scoped lang="scss">
.project-list__header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.project-list__actions {
  display: flex;
  gap: 12px;
}
.project-list__name {
  display: flex;
  align-items: center;
  gap: 6px;
}
.project-list__code {
  font-family: var(--el-font-family-monospace, monospace);
  font-size: 12px;
  color: var(--el-text-color-secondary);
}
.project-list__tip {
  margin: 0 0 12px;
  color: var(--el-text-color-secondary);
  font-size: 13px;
}
.project-list__snippet-meta {
  margin-top: 12px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 13px;
}
</style>
