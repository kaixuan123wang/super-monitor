<script setup lang="ts">
import { ref, onMounted, watch, reactive } from 'vue';
import { useProjectStore } from '@/stores/project';
import {
  listRules,
  createRule,
  updateRule,
  deleteRule,
  listLogs,
  type AlertRule,
  type AlertLog,
  type CreateRuleBody,
} from '@/api/alert';
import { ElMessage, ElMessageBox } from 'element-plus';
import { Check } from '@element-plus/icons-vue';

const projectStore = useProjectStore();

// ── 规则 ──────────────────────────────────────────────────────────
const rulesLoading = ref(false);
const rules = ref<AlertRule[]>([]);

async function fetchRules() {
  if (!projectStore.currentId) return;
  rulesLoading.value = true;
  try {
    const res = await listRules(projectStore.currentId);
    rules.value = res.data?.list ?? [];
  } finally {
    rulesLoading.value = false;
  }
}

// ── 日志 ──────────────────────────────────────────────────────────
const logsLoading = ref(false);
const logs = ref<AlertLog[]>([]);
const logsTotal = ref(0);
const logsPage = ref(1);

async function fetchLogs() {
  if (!projectStore.currentId) return;
  logsLoading.value = true;
  try {
    const res = await listLogs({
      project_id: projectStore.currentId,
      page: logsPage.value,
      page_size: 20,
    });
    logs.value = res.data?.list ?? [];
    logsTotal.value = res.data?.total ?? 0;
  } finally {
    logsLoading.value = false;
  }
}

// ── 创建/编辑 dialog ──────────────────────────────────────────────
const dialogVisible = ref(false);
const editingId = ref<number | null>(null);
const saving = ref(false);

const form = reactive<CreateRuleBody>({
  project_id: 0,
  name: '',
  rule_type: 'error_spike',
  threshold: 10,
  interval_minutes: 60,
  webhook_url: undefined,
  email: undefined,
});

const RULE_TYPE_OPTIONS = [
  { label: '错误激增 (error_spike)', value: 'error_spike' },
  { label: '接口失败率 (failure_rate)', value: 'failure_rate' },
  { label: '新错误 (new_error)', value: 'new_error' },
  { label: 'P0 错误 (p0_error)', value: 'p0_error' },
  { label: '错误趋势 (error_trend)', value: 'error_trend' },
];

function openCreate() {
  editingId.value = null;
  Object.assign(form, {
    project_id: projectStore.currentId ?? 0,
    name: '',
    rule_type: 'error_spike',
    threshold: 10,
    interval_minutes: 60,
    webhook_url: '',
    email: '',
  });
  dialogVisible.value = true;
}

function openEdit(rule: AlertRule) {
  editingId.value = rule.id;
  Object.assign(form, {
    project_id: rule.project_id,
    name: rule.name,
    rule_type: rule.rule_type,
    threshold: rule.threshold ?? 10,
    interval_minutes: rule.interval_minutes,
    webhook_url: rule.webhook_url ?? '',
    email: rule.email ?? '',
  });
  dialogVisible.value = true;
}

async function saveRule() {
  if (!form.name) {
    ElMessage.warning('请填写规则名称');
    return;
  }
  saving.value = true;
  try {
    if (editingId.value) {
      await updateRule(editingId.value, form);
    } else {
      form.project_id = projectStore.currentId ?? 0;
      await createRule(form);
    }
    ElMessage.success('保存成功');
    dialogVisible.value = false;
    fetchRules();
  } finally {
    saving.value = false;
  }
}

async function handleDelete(rule: AlertRule) {
  try {
    await ElMessageBox.confirm(`确认删除规则「${rule.name}」？`, '确认', { type: 'warning' });
    await deleteRule(rule.id);
    ElMessage.success('已删除');
    fetchRules();
  } catch {
    // 用户取消或请求失败
  }
}

async function toggleEnabled(rule: AlertRule) {
  try {
    await updateRule(rule.id, { is_enabled: !rule.is_enabled });
    ElMessage.success(rule.is_enabled ? '已禁用' : '已启用');
    fetchRules();
  } catch {
    // 请求失败
  }
}

function severityTag(s: string): 'danger' | 'warning' | 'info' | 'success' {
  if (s === 'critical') return 'danger';
  if (s === 'warning') return 'warning';
  if (s === 'info') return 'info';
  return 'success';
}

const activeTab = ref<'rules' | 'logs'>('rules');

onMounted(async () => {
  if (!projectStore.list.length) await projectStore.fetchAll();
  fetchRules();
  fetchLogs();
});

watch(
  () => projectStore.currentId,
  () => {
    fetchRules();
    fetchLogs();
  }
);
watch(activeTab, (v) => {
  if (v === 'logs') fetchLogs();
});
</script>

<template>
  <div>
    <el-tabs v-model="activeTab">
      <!-- 告警规则 -->
      <el-tab-pane label="告警规则" name="rules">
        <div style="display: flex; justify-content: flex-end; margin-bottom: 12px">
          <el-button type="primary" @click="openCreate">新建规则</el-button>
        </div>

        <el-table :data="rules" v-loading="rulesLoading" border>
          <el-table-column prop="name" label="规则名称" min-width="160" />
          <el-table-column prop="rule_type" label="类型" width="160" />
          <el-table-column label="阈值" width="80">
            <template #default="{ row }">{{ row.threshold ?? '-' }}</template>
          </el-table-column>
          <el-table-column prop="interval_minutes" label="窗口(分钟)" width="110" />
          <el-table-column label="状态" width="90">
            <template #default="{ row }">
              <el-tag :type="row.is_enabled ? 'success' : 'info'" size="small">
                {{ row.is_enabled ? '启用' : '禁用' }}
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column label="Webhook" width="80">
            <template #default="{ row }">
              <el-icon v-if="row.webhook_url" color="#67c23a"><Check /></el-icon>
              <span v-else>-</span>
            </template>
          </el-table-column>
          <el-table-column label="邮件" width="70">
            <template #default="{ row }">
              <el-icon v-if="row.email" color="#67c23a"><Check /></el-icon>
              <span v-else>-</span>
            </template>
          </el-table-column>
          <el-table-column label="操作" width="170" fixed="right">
            <template #default="{ row }">
              <el-button link @click="toggleEnabled(row)">{{
                row.is_enabled ? '禁用' : '启用'
              }}</el-button>
              <el-button link type="primary" @click="openEdit(row)">编辑</el-button>
              <el-button link type="danger" @click="handleDelete(row)">删除</el-button>
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>

      <!-- 告警日志 -->
      <el-tab-pane label="告警历史" name="logs">
        <el-table :data="logs" v-loading="logsLoading" border>
          <el-table-column prop="id" label="ID" width="70" />
          <el-table-column prop="alert_type" label="类型" width="130" />
          <el-table-column label="严重度" width="90">
            <template #default="{ row }">
              <el-tag :type="severityTag(row.severity)" size="small">{{ row.severity }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="content" label="内容" min-width="200" show-overflow-tooltip />
          <el-table-column prop="error_count" label="错误数" width="80" />
          <el-table-column label="状态" width="80">
            <template #default="{ row }">
              <el-tag :type="row.status === 'sent' ? 'success' : 'danger'" size="small">{{
                row.status
              }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column label="时间" width="170">
            <template #default="{ row }">{{
              row.created_at?.slice(0, 19).replace('T', ' ')
            }}</template>
          </el-table-column>
        </el-table>
        <div style="margin-top: 12px; text-align: right">
          <el-pagination
            v-model:current-page="logsPage"
            :page-size="20"
            :total="logsTotal"
            layout="total, prev, pager, next"
            @current-change="fetchLogs"
          />
        </div>
      </el-tab-pane>
    </el-tabs>

    <!-- 创建/编辑规则 dialog -->
    <el-dialog
      v-model="dialogVisible"
      :title="editingId ? '编辑告警规则' : '新建告警规则'"
      width="500px"
    >
      <el-form :model="form" label-width="110px">
        <el-form-item label="规则名称" required>
          <el-input v-model="form.name" placeholder="例如：生产环境错误激增" />
        </el-form-item>
        <el-form-item label="规则类型" required>
          <el-select v-model="form.rule_type" style="width: 100%">
            <el-option
              v-for="o in RULE_TYPE_OPTIONS"
              :key="o.value"
              :label="o.label"
              :value="o.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item
          v-if="['error_spike', 'failure_rate', 'error_trend'].includes(form.rule_type)"
          label="阈值"
        >
          <el-input-number v-model="form.threshold" :min="1" />
          <span style="margin-left: 8px; color: #999">
            {{ form.rule_type === 'failure_rate' ? '%' : '次' }}
          </span>
        </el-form-item>
        <el-form-item label="检查窗口(分钟)">
          <el-input-number v-model="form.interval_minutes" :min="1" :max="1440" />
        </el-form-item>
        <el-form-item label="Webhook URL">
          <el-input v-model="form.webhook_url" placeholder="飞书/钉钉/企微 webhook（预留）" />
        </el-form-item>
        <el-form-item label="通知邮箱">
          <el-input v-model="form.email" placeholder="多个邮箱用逗号分隔（预留）" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="saving" @click="saveRule">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>
