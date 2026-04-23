<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from 'vue';
import { useProjectStore } from '@/stores/project';
import {
  getTrackUserDetail,
  listTrackUserEvents,
  listTrackUsers,
  type TrackUserEvent,
  type TrackUserFilter,
  type TrackUserProfile,
} from '@/api/tracking';

const projectStore = useProjectStore();

const loading = ref(false);
const users = ref<TrackUserProfile[]>([]);
const total = ref(0);
const page = ref(1);
const pageSize = ref(20);
const keyword = ref('');

const filters = ref<TrackUserFilter[]>([]);

const drawerVisible = ref(false);
const detailLoading = ref(false);
const selectedUser = ref<TrackUserProfile | null>(null);
const timeline = ref<TrackUserEvent[]>([]);
const eventTotal = ref(0);
const eventPage = ref(1);
const eventPageSize = ref(20);
const eventName = ref('');

const draftFilter = reactive<TrackUserFilter>({
  property: '',
  operator: 'eq',
  value: '',
});

const activeFilters = computed(() =>
  filters.value.filter((f) => f.property && (needsValue(f.operator) ? f.value !== '' : true))
);

async function fetchUsers() {
  if (!projectStore.currentId) return;
  loading.value = true;
  try {
    const res = await listTrackUsers({
      project_id: projectStore.currentId,
      keyword: keyword.value || undefined,
      filters: activeFilters.value.length ? JSON.stringify(activeFilters.value) : undefined,
      page: page.value,
      page_size: pageSize.value,
    });
    users.value = res.data?.list ?? [];
    total.value = res.data?.total ?? 0;
  } finally {
    loading.value = false;
  }
}

function search() {
  page.value = 1;
  fetchUsers();
}

function addFilter() {
  if (!draftFilter.property.trim()) return;
  filters.value.push({
    property: draftFilter.property.trim(),
    operator: draftFilter.operator,
    value: needsValue(draftFilter.operator) ? draftFilter.value : undefined,
  });
  draftFilter.property = '';
  draftFilter.operator = 'eq';
  draftFilter.value = '';
  search();
}

function removeFilter(index: number) {
  filters.value.splice(index, 1);
  search();
}

function needsValue(operator: TrackUserFilter['operator']) {
  return operator !== 'exists' && operator !== 'not_exists';
}

async function openDetail(row: TrackUserProfile) {
  if (!projectStore.currentId) return;
  drawerVisible.value = true;
  detailLoading.value = true;
  selectedUser.value = row;
  eventPage.value = 1;
  eventName.value = '';
  try {
    const res = await getTrackUserDetail(projectStore.currentId, row.distinct_id);
    selectedUser.value = res.data.user;
    timeline.value = res.data.recent_events ?? [];
    eventTotal.value = timeline.value.length;
  } finally {
    detailLoading.value = false;
  }
}

async function fetchTimeline() {
  if (!projectStore.currentId || !selectedUser.value) return;
  detailLoading.value = true;
  try {
    const res = await listTrackUserEvents(selectedUser.value.distinct_id, {
      project_id: projectStore.currentId,
      page: eventPage.value,
      page_size: eventPageSize.value,
      event_name: eventName.value || undefined,
    });
    timeline.value = res.data?.list ?? [];
    eventTotal.value = res.data?.total ?? 0;
  } finally {
    detailLoading.value = false;
  }
}

function formatTime(value?: string | null) {
  return value ? value.slice(0, 19).replace('T', ' ') : '-';
}

function valueText(value: unknown) {
  if (value === null || value === undefined || value === '') return '-';
  if (typeof value === 'object') return JSON.stringify(value);
  return String(value);
}

function profileEntries(profile?: TrackUserProfile | null) {
  return Object.entries(profile?.properties ?? {});
}

function propertyPreview(row: TrackUserProfile) {
  const entries = Object.entries(row.properties ?? {}).slice(0, 3);
  if (!entries.length) return '-';
  return entries.map(([key, value]) => `${key}: ${valueText(value)}`).join(' / ');
}

onMounted(async () => {
  if (!projectStore.list.length) await projectStore.fetchAll();
  fetchUsers();
});

watch(
  () => projectStore.currentId,
  () => {
    page.value = 1;
    drawerVisible.value = false;
    fetchUsers();
  }
);
</script>

<template>
  <div class="track-users">
    <aside class="track-users__filters">
      <div class="filter-title">属性筛选</div>
      <el-form label-position="top">
        <el-form-item label="属性名">
          <el-input v-model="draftFilter.property" placeholder="city" clearable />
        </el-form-item>
        <el-form-item label="条件">
          <el-select v-model="draftFilter.operator" style="width: 100%">
            <el-option label="等于" value="eq" />
            <el-option label="不等于" value="neq" />
            <el-option label="包含" value="contains" />
            <el-option label="存在" value="exists" />
            <el-option label="不存在" value="not_exists" />
          </el-select>
        </el-form-item>
        <el-form-item v-if="needsValue(draftFilter.operator)" label="属性值">
          <el-input v-model="draftFilter.value" placeholder="premium" clearable @keyup.enter="addFilter" />
        </el-form-item>
        <el-button type="primary" style="width: 100%" @click="addFilter">添加筛选</el-button>
      </el-form>

      <div v-if="filters.length" class="filter-list">
        <el-tag
          v-for="(f, index) in filters"
          :key="`${f.property}-${index}`"
          closable
          class="filter-tag"
          @close="removeFilter(index)"
        >
          {{ f.property }} {{ f.operator }} {{ needsValue(f.operator) ? f.value : '' }}
        </el-tag>
      </div>
    </aside>

    <section class="track-users__content">
      <div class="toolbar">
        <el-input
          v-model="keyword"
          placeholder="搜索 distinct_id / user_id / name / email"
          clearable
          class="keyword-input"
          @keyup.enter="search"
          @clear="search"
        />
        <el-button type="primary" @click="search">搜索</el-button>
        <el-button @click="fetchUsers">刷新</el-button>
      </div>

      <el-table v-loading="loading" :data="users" style="width: 100%" @row-click="openDetail">
        <el-table-column label="Distinct ID" prop="distinct_id" min-width="220">
          <template #default="{ row }">
            <el-link type="primary" @click.stop="openDetail(row)">{{ row.distinct_id }}</el-link>
          </template>
        </el-table-column>
        <el-table-column label="User ID" prop="user_id" min-width="160" />
        <el-table-column label="最近访问" width="180">
          <template #default="{ row }">{{ formatTime(row.last_visit_at) }}</template>
        </el-table-column>
        <el-table-column label="事件数" prop="total_events" width="100" align="right" />
        <el-table-column label="属性预览" min-width="260">
          <template #default="{ row }">{{ propertyPreview(row) }}</template>
        </el-table-column>
      </el-table>

      <el-pagination
        v-if="total > pageSize"
        v-model:current-page="page"
        :page-size="pageSize"
        :total="total"
        layout="prev, pager, next, total"
        class="pagination"
        @current-change="fetchUsers"
      />
    </section>

    <el-drawer v-model="drawerVisible" size="720px" :title="selectedUser?.distinct_id || '用户详情'">
      <div v-loading="detailLoading" class="user-detail">
        <div v-if="selectedUser" class="profile-summary">
          <el-descriptions :column="2" border>
            <el-descriptions-item label="Distinct ID">{{ selectedUser.distinct_id }}</el-descriptions-item>
            <el-descriptions-item label="User ID">{{ selectedUser.user_id || '-' }}</el-descriptions-item>
            <el-descriptions-item label="首次访问">{{ formatTime(selectedUser.first_visit_at) }}</el-descriptions-item>
            <el-descriptions-item label="最近访问">{{ formatTime(selectedUser.last_visit_at) }}</el-descriptions-item>
            <el-descriptions-item label="事件数">{{ selectedUser.total_events }}</el-descriptions-item>
            <el-descriptions-item label="会话数">{{ selectedUser.total_sessions }}</el-descriptions-item>
          </el-descriptions>
        </div>

        <el-divider content-position="left">用户属性</el-divider>
        <el-empty v-if="!profileEntries(selectedUser).length" description="暂无属性" />
        <el-table v-else :data="profileEntries(selectedUser)" size="small" border>
          <el-table-column label="属性" width="180">
            <template #default="{ row }">{{ row[0] }}</template>
          </el-table-column>
          <el-table-column label="值">
            <template #default="{ row }">{{ valueText(row[1]) }}</template>
          </el-table-column>
        </el-table>

        <el-divider content-position="left">事件时间线</el-divider>
        <div class="timeline-toolbar">
          <el-input
            v-model="eventName"
            placeholder="事件名过滤"
            clearable
            style="width: 220px"
            @keyup.enter="fetchTimeline"
            @clear="fetchTimeline"
          />
          <el-button @click="fetchTimeline">筛选</el-button>
        </div>

        <el-timeline v-if="timeline.length" class="event-timeline">
          <el-timeline-item
            v-for="event in timeline"
            :key="event.id"
            :timestamp="formatTime(event.created_at)"
            placement="top"
          >
            <div class="event-line">
              <div class="event-line__head">
                <el-tag size="small" :type="event.event_type === 'auto' ? 'info' : 'success'">
                  {{ event.event }}
                </el-tag>
                <span>{{ event.page_url || '-' }}</span>
              </div>
              <pre>{{ JSON.stringify(event.properties || {}, null, 2) }}</pre>
            </div>
          </el-timeline-item>
        </el-timeline>
        <el-empty v-else description="暂无事件" />

        <el-pagination
          v-if="eventTotal > eventPageSize"
          v-model:current-page="eventPage"
          :page-size="eventPageSize"
          :total="eventTotal"
          layout="prev, pager, next, total"
          class="pagination"
          @current-change="fetchTimeline"
        />
      </div>
    </el-drawer>
  </div>
</template>

<style scoped lang="scss">
.track-users {
  display: grid;
  grid-template-columns: 280px minmax(0, 1fr);
  gap: 16px;

  &__filters,
  &__content {
    background: var(--el-bg-color);
    border: 1px solid var(--el-border-color-light);
    border-radius: 6px;
    padding: 16px;
  }
}

.filter-title {
  font-size: 15px;
  font-weight: 600;
  margin-bottom: 14px;
}

.filter-list {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 14px;
}

.filter-tag {
  max-width: 100%;
}

.toolbar,
.timeline-toolbar {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 14px;
}

.keyword-input {
  max-width: 360px;
}

.pagination {
  justify-content: flex-end;
  margin-top: 16px;
}

.user-detail {
  min-height: 420px;
}

.event-timeline {
  margin-top: 14px;
}

.event-line {
  border: 1px solid var(--el-border-color-lighter);
  border-radius: 6px;
  padding: 10px 12px;

  &__head {
    display: flex;
    gap: 10px;
    align-items: center;
    margin-bottom: 8px;

    span {
      color: var(--el-text-color-secondary);
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
  }

  pre {
    margin: 0;
    max-height: 180px;
    overflow: auto;
    background: var(--el-fill-color-light);
    border-radius: 4px;
    padding: 8px;
    font-size: 12px;
    line-height: 1.5;
  }
}

@media (max-width: 960px) {
  .track-users {
    grid-template-columns: 1fr;
  }
}
</style>
