<script setup lang="ts">
import { ref, reactive, onMounted, watch } from 'vue';
import { useRouter } from 'vue-router';
import { useProjectStore } from '@/stores/project';
import {
  listTrackEvents,
  listDefinitions,
  createDefinition,
  updateDefinition,
  deleteDefinition,
  listProperties,
  type EventItem,
  type EventDefinition,
  type PropertyDef,
  type PropItem,
} from '@/api/tracking';
import { ElMessage, ElMessageBox } from 'element-plus';

const router = useRouter();
const projectStore = useProjectStore();

// ── Tab ────────────────────────────────────────────────────────────
const activeTab = ref<'collected' | 'definitions' | 'properties'>('collected');

// ── 已采集事件 ─────────────────────────────────────────────────────
const loading = ref(false);
const list = ref<EventItem[]>([]);
const total = ref(0);
const page = ref(1);
const pageSize = ref(20);
const keyword = ref('');

async function fetchList() {
  if (!projectStore.currentId) return;
  loading.value = true;
  try {
    const res = await listTrackEvents({
      project_id: projectStore.currentId,
      keyword: keyword.value || undefined,
      page: page.value,
      page_size: pageSize.value,
    });
    list.value = res.data?.list ?? [];
    total.value = res.data?.total ?? 0;
  } finally {
    loading.value = false;
  }
}

function onSearch() {
  page.value = 1;
  fetchList();
}
function goDetail(row: EventItem) {
  router.push(`/tracking/events/${encodeURIComponent(row.event)}`);
}

// ── 事件定义 ───────────────────────────────────────────────────────
const defLoading = ref(false);
const defList = ref<EventDefinition[]>([]);
const defTotal = ref(0);
const defPage = ref(1);
const defKeyword = ref('');

async function fetchDefs() {
  if (!projectStore.currentId) return;
  defLoading.value = true;
  try {
    const res = await listDefinitions({
      project_id: projectStore.currentId,
      keyword: defKeyword.value || undefined,
      page: defPage.value,
      page_size: 20,
    });
    defList.value = res.data?.list ?? [];
    defTotal.value = res.data?.total ?? 0;
  } finally {
    defLoading.value = false;
  }
}

function searchDefs() {
  defPage.value = 1;
  fetchDefs();
}

// 创建/编辑 dialog
const dialogVisible = ref(false);
const isEdit = ref(false);
const editingId = ref<number | null>(null);

const defaultProp = (): PropertyDef => ({
  name: '',
  type: 'string',
  description: '',
  required: false,
});

const form = reactive({
  event_name: '',
  display_name: '',
  category: '',
  description: '',
  properties: [] as PropertyDef[],
});

function openCreate() {
  isEdit.value = false;
  editingId.value = null;
  form.event_name = '';
  form.display_name = '';
  form.category = '';
  form.description = '';
  form.properties = [];
  dialogVisible.value = true;
}

function openEdit(row: EventDefinition) {
  isEdit.value = true;
  editingId.value = row.id;
  form.event_name = row.event_name;
  form.display_name = row.display_name ?? '';
  form.category = row.category ?? '';
  form.description = row.description ?? '';
  form.properties = row.properties ? [...row.properties] : [];
  dialogVisible.value = true;
}

async function submitForm() {
  if (!form.event_name.trim()) {
    ElMessage.error('事件名不能为空');
    return;
  }
  try {
    const props = form.properties.filter((p) => p.name.trim());
    if (isEdit.value && editingId.value) {
      await updateDefinition(editingId.value, {
        display_name: form.display_name || undefined,
        category: form.category || undefined,
        description: form.description || undefined,
        properties: props.length ? props : undefined,
      });
      ElMessage.success('更新成功');
    } else {
      await createDefinition({
        project_id: projectStore.currentId!,
        event_name: form.event_name.trim(),
        display_name: form.display_name || undefined,
        category: form.category || undefined,
        description: form.description || undefined,
        properties: props.length ? props : undefined,
      });
      ElMessage.success('创建成功');
    }
    dialogVisible.value = false;
    fetchDefs();
  } catch {
    /* error shown by request interceptor */
  }
}

async function deleteDef(row: EventDefinition) {
  try {
    await ElMessageBox.confirm(`确定删除事件定义「${row.event_name}」？`, '警告', {
      type: 'warning',
    });
    await deleteDefinition(row.id);
    ElMessage.success('已删除');
    fetchDefs();
  } catch {
    // 用户取消或请求失败
  }
}

// ── 属性管理 ───────────────────────────────────────────────────────
const propLoading = ref(false);
const propList = ref<PropItem[]>([]);

async function fetchProps() {
  if (!projectStore.currentId) return;
  propLoading.value = true;
  try {
    const res = await listProperties(projectStore.currentId);
    propList.value = res.data?.list ?? [];
  } finally {
    propLoading.value = false;
  }
}

function onTabChange(tab: 'collected' | 'definitions' | 'properties') {
  activeTab.value = tab;
  if (tab === 'collected') fetchList();
  else if (tab === 'definitions') fetchDefs();
  else fetchProps();
}

onMounted(() => fetchList());
watch(
  () => projectStore.currentId,
  () => {
    if (activeTab.value === 'collected') fetchList();
    else if (activeTab.value === 'definitions') fetchDefs();
    else fetchProps();
  }
);
</script>

<template>
  <div>
    <!-- 子 Tab -->
    <el-tabs :model-value="activeTab" @tab-click="(t) => onTabChange(t.paneName as any)">
      <el-tab-pane label="已采集事件" name="collected" />
      <el-tab-pane label="事件定义管理" name="definitions" />
      <el-tab-pane label="属性管理" name="properties" />
    </el-tabs>

    <!-- ── 已采集事件 ── -->
    <template v-if="activeTab === 'collected'">
      <div class="toolbar">
        <el-input
          v-model="keyword"
          placeholder="搜索事件名"
          clearable
          style="width: 260px"
          @keyup.enter="onSearch"
          @clear="onSearch"
        />
        <el-button type="primary" @click="onSearch">搜索</el-button>
      </div>
      <el-table v-loading="loading" :data="list" style="width: 100%">
        <el-table-column label="事件名" prop="event" min-width="200">
          <template #default="{ row }">
            <el-link type="primary" @click="goDetail(row)">{{ row.event }}</el-link>
          </template>
        </el-table-column>
        <el-table-column label="类型" width="90">
          <template #default="{ row }">
            <el-tag :type="row.category === 'auto' ? 'info' : 'success'" size="small">
              {{ row.category === 'auto' ? '自动' : '自定义' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="总次数" prop="total_count" width="110" align="right" />
        <el-table-column label="独立用户" prop="unique_users" width="110" align="right" />
        <el-table-column label="最后上报" prop="last_seen" width="180" />
      </el-table>
      <el-pagination
        v-if="total > pageSize"
        v-model:current-page="page"
        :page-size="pageSize"
        :total="total"
        layout="prev, pager, next, total"
        class="pagination"
        @current-change="fetchList"
      />
    </template>

    <!-- ── 事件定义管理 ── -->
    <template v-if="activeTab === 'definitions'">
      <div class="toolbar">
        <el-input
          v-model="defKeyword"
          placeholder="搜索事件名"
          clearable
          style="width: 260px"
          @keyup.enter="searchDefs"
          @clear="searchDefs"
        />
        <el-button @click="searchDefs">搜索</el-button>
        <el-button type="primary" @click="openCreate">+ 新建事件定义</el-button>
      </div>
      <el-table v-loading="defLoading" :data="defList" style="width: 100%">
        <el-table-column label="事件名" prop="event_name" min-width="160" />
        <el-table-column label="展示名" prop="display_name" width="140" />
        <el-table-column label="分类" prop="category" width="100" />
        <el-table-column label="属性数" width="90" align="center">
          <template #default="{ row }">{{ row.properties?.length ?? 0 }}</template>
        </el-table-column>
        <el-table-column label="状态" width="90">
          <template #default="{ row }">
            <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
              {{ row.status === 'active' ? '启用' : '停用' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="140" align="center">
          <template #default="{ row }">
            <el-button link type="primary" @click="openEdit(row)">编辑</el-button>
            <el-button link type="danger" @click="deleteDef(row)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>
      <el-pagination
        v-if="defTotal > 20"
        v-model:current-page="defPage"
        :page-size="20"
        :total="defTotal"
        layout="prev, pager, next, total"
        class="pagination"
        @current-change="fetchDefs"
      />
    </template>

    <!-- ── 属性管理 ── -->
    <template v-if="activeTab === 'properties'">
      <el-table v-loading="propLoading" :data="propList" style="width: 100%">
        <el-table-column label="属性名" prop="name" min-width="160" />
        <el-table-column label="类型" prop="prop_type" width="100" />
        <el-table-column label="必填" width="80" align="center">
          <template #default="{ row }">
            <el-tag :type="row.required ? 'danger' : 'info'" size="small">
              {{ row.required ? '是' : '否' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="说明" prop="description" min-width="160" show-overflow-tooltip />
        <el-table-column label="关联事件" min-width="200">
          <template #default="{ row }">
            <el-tag v-for="e in row.event_names" :key="e" size="small" style="margin: 2px">{{
              e
            }}</el-tag>
          </template>
        </el-table-column>
      </el-table>
    </template>

    <!-- ── 创建/编辑 Dialog ── -->
    <el-dialog
      v-model="dialogVisible"
      :title="isEdit ? '编辑事件定义' : '新建事件定义'"
      width="640px"
    >
      <el-form label-width="90px">
        <el-form-item label="事件名" required>
          <el-input v-model="form.event_name" :disabled="isEdit" placeholder="如: button_click" />
        </el-form-item>
        <el-form-item label="展示名">
          <el-input v-model="form.display_name" placeholder="如: 按钮点击" />
        </el-form-item>
        <el-form-item label="分类">
          <el-input v-model="form.category" placeholder="如: 点击事件" />
        </el-form-item>
        <el-form-item label="说明">
          <el-input v-model="form.description" type="textarea" :rows="2" />
        </el-form-item>
        <el-form-item label="属性定义">
          <div style="width: 100%">
            <div v-for="(prop, idx) in form.properties" :key="idx" class="prop-row">
              <el-input v-model="prop.name" placeholder="属性名" style="width: 130px" />
              <el-select v-model="prop.type" style="width: 100px">
                <el-option label="string" value="string" />
                <el-option label="number" value="number" />
                <el-option label="boolean" value="boolean" />
                <el-option label="array" value="array" />
              </el-select>
              <el-input v-model="prop.description" placeholder="说明" style="flex: 1" />
              <el-checkbox v-model="prop.required" label="必填" />
              <el-button link type="danger" @click="form.properties.splice(idx, 1)">删除</el-button>
            </div>
            <el-button size="small" @click="form.properties.push(defaultProp())"
              >+ 添加属性</el-button
            >
          </div>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" @click="submitForm">{{ isEdit ? '保存' : '创建' }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped lang="scss">
.toolbar {
  display: flex;
  gap: 8px;
  margin-bottom: 16px;
}
.pagination {
  margin-top: 16px;
  justify-content: flex-end;
}
.prop-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}
</style>
