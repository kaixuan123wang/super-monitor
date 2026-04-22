<script setup lang="ts">
import { onMounted, reactive, ref, watch } from 'vue';
import { ElMessage } from 'element-plus';
import { useProjectStore } from '@/stores/project';
import { listErrors, type JsErrorRow, type ListErrorsParams } from '@/api/error';
import ErrorDetail from './ErrorDetail.vue';

const projectStore = useProjectStore();

const loading = ref(false);
const list = ref<JsErrorRow[]>([]);
const total = ref(0);

const filter = reactive<Omit<ListErrorsParams, 'project_id'>>({
  page: 1,
  page_size: 20,
  error_type: undefined,
  browser: undefined,
  keyword: undefined,
});

const detailVisible = ref(false);
const selected = ref<JsErrorRow | null>(null);

async function reload() {
  if (!projectStore.currentId) {
    list.value = [];
    total.value = 0;
    return;
  }
  loading.value = true;
  try {
    const resp = await listErrors({
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

function onSearch() {
  filter.page = 1;
  reload();
}

function onReset() {
  filter.error_type = undefined;
  filter.browser = undefined;
  filter.keyword = undefined;
  filter.page = 1;
  reload();
}

function onPageChange(p: number) {
  filter.page = p;
  reload();
}

function showDetail(row: JsErrorRow) {
  selected.value = row;
  detailVisible.value = true;
}

onMounted(async () => {
  if (!projectStore.list.length) {
    await projectStore.fetchAll();
  }
  reload();
});

// 切换当前项目时刷新
watch(
  () => projectStore.currentId,
  () => {
    filter.page = 1;
    reload();
  }
);

function errorTypeTag(t: string): 'danger' | 'warning' | 'info' {
  if (t === 'js') return 'danger';
  if (t === 'promise') return 'warning';
  return 'info';
}

function truncate(s: string | null | undefined, n = 80): string {
  if (!s) return '';
  return s.length > n ? s.slice(0, n) + '…' : s;
}
</script>

<template>
  <el-card shadow="never">
    <template #header>
      <div class="errors-page__header">
        <div>
          <span>错误监控</span>
          <span v-if="projectStore.current" class="errors-page__project">
            · {{ projectStore.current.name }}
          </span>
        </div>
        <el-select
          v-model="projectStore.currentId"
          placeholder="请选择项目"
          style="width: 220px"
          @change="(v: number) => projectStore.setCurrent(v)"
        >
          <el-option
            v-for="p in projectStore.list"
            :key="p.id"
            :label="p.name"
            :value="p.id"
          />
        </el-select>
      </div>
    </template>

    <el-empty
      v-if="!projectStore.currentId"
      description="请先创建并选择一个项目"
    />

    <template v-else>
      <!-- 筛选栏 -->
      <el-form inline :model="filter" class="errors-page__filter">
        <el-form-item label="错误类型">
          <el-select v-model="filter.error_type" clearable placeholder="全部" style="width: 140px">
            <el-option label="JS" value="js" />
            <el-option label="Promise" value="promise" />
            <el-option label="Resource" value="resource" />
          </el-select>
        </el-form-item>
        <el-form-item label="浏览器">
          <el-input v-model="filter.browser" clearable placeholder="例如 Chrome" style="width: 160px" />
        </el-form-item>
        <el-form-item label="关键字">
          <el-input
            v-model="filter.keyword"
            clearable
            placeholder="匹配错误消息"
            style="width: 220px"
            @keyup.enter="onSearch"
          />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="onSearch">查询</el-button>
          <el-button @click="onReset">重置</el-button>
        </el-form-item>
      </el-form>

      <el-table :data="list" v-loading="loading" border stripe @row-click="showDetail">
        <el-table-column prop="id" label="ID" width="80" />
        <el-table-column prop="error_type" label="类型" width="100">
          <template #default="{ row }">
            <el-tag :type="errorTypeTag(row.error_type)" size="small">{{ row.error_type }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="message" label="消息" min-width="320">
          <template #default="{ row }">
            <span :title="row.message">{{ truncate(row.message, 120) }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="browser" label="浏览器" width="160">
          <template #default="{ row }">
            <span v-if="row.browser">{{ row.browser }} {{ row.browser_version }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="os" label="系统" width="140">
          <template #default="{ row }">
            <span v-if="row.os">{{ row.os }} {{ row.os_version }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="url" label="页面 URL" min-width="220">
          <template #default="{ row }">
            <span :title="row.url">{{ truncate(row.url, 60) }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="created_at" label="时间" width="170">
          <template #default="{ row }">
            {{ new Date(row.created_at).toLocaleString() }}
          </template>
        </el-table-column>
      </el-table>

      <div class="errors-page__pagination">
        <el-pagination
          :current-page="filter.page"
          :page-size="filter.page_size"
          :total="total"
          layout="total, prev, pager, next, jumper"
          background
          @current-change="onPageChange"
        />
      </div>
    </template>
  </el-card>

  <ErrorDetail v-model="detailVisible" :row="selected" />
</template>

<style scoped lang="scss">
.errors-page__header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.errors-page__project {
  color: var(--el-text-color-secondary);
  font-weight: normal;
  margin-left: 8px;
  font-size: 13px;
}
.errors-page__filter {
  margin-bottom: 12px;
}
.errors-page__pagination {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}
</style>
