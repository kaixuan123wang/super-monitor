<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue';
import { useProjectStore } from '@/stores/project';
import { SSEClient } from '@/utils/sse';

const projectStore = useProjectStore();

type LiveItem = {
  id: number;
  type: 'track' | 'profile';
  event: string;
  distinct_id: string;
  properties: unknown;
  page_url?: string | null;
  created_at?: string;
  updated_at?: string;
};

const events = ref<LiveItem[]>([]);
const isPaused = ref(false);
const filterEvent = ref('');
const filterUser = ref('');
const sseConnected = ref(false);

let sseClient: SSEClient | null = null;

function connectSSE() {
  if (!projectStore.currentId || isPaused.value) return;

  sseClient?.disconnect();
  const url = `/api/tracking/live-events?project_id=${projectStore.currentId}${filterUser.value ? '&distinct_id=' + encodeURIComponent(filterUser.value) : ''}`;
  sseClient = new SSEClient(url)
    .on('init', () => { sseConnected.value = true; })
    .on('track', (data: unknown) => {
      const d = data as { id: number; event: string; distinct_id: string; properties: unknown; page_url: string | null; created_at: string };
      if (filterEvent.value && d.event !== filterEvent.value) return;
      events.value.unshift({ ...d, type: 'track' });
      if (events.value.length > 200) events.value.pop();
    })
    .on('profile', (data: unknown) => {
      const d = data as { id: number; distinct_id: string; properties: unknown; updated_at: string };
      if (filterEvent.value && filterEvent.value !== 'profile') return;
      events.value.unshift({ ...d, type: 'profile', event: 'profile' });
      if (events.value.length > 200) events.value.pop();
    })
    .on('heartbeat', () => { sseConnected.value = true; });

  sseClient.connect();
}

function disconnect() {
  sseClient?.disconnect();
  sseConnected.value = false;
}

function togglePause() {
  isPaused.value = !isPaused.value;
  if (isPaused.value) {
    disconnect();
  } else {
    connectSSE();
  }
}

function clearEvents() {
  events.value = [];
}

function formatJson(obj: unknown): string {
  return JSON.stringify(obj, null, 2);
}

function formatTime(iso?: string): string {
  return iso?.slice(0, 19).replace('T', ' ') ?? '';
}

onMounted(async () => {
  if (!projectStore.list.length) await projectStore.fetchAll();
  if (projectStore.currentId) connectSSE();
});

onUnmounted(() => {
  disconnect();
});

watch(() => projectStore.currentId, (id) => {
  events.value = [];
  if (id && !isPaused.value) connectSSE();
  else disconnect();
});
</script>

<template>
  <div>
    <el-card shadow="never">
      <template #header>
        <div style="display:flex;justify-content:space-between;align-items:center">
          <span>
            实时事件流
            <el-tag :type="sseConnected ? 'success' : 'danger'" size="small" style="margin-left:8px">
              {{ sseConnected ? '已连接' : '已断开' }}
            </el-tag>
          </span>
          <div style="display:flex;gap:8px;align-items:center">
            <el-input v-model="filterEvent" placeholder="事件名过滤" clearable style="width:140px" @change="connectSSE" />
            <el-input v-model="filterUser" placeholder="用户 ID 过滤" clearable style="width:160px" @change="connectSSE" />
            <el-button :type="isPaused ? 'warning' : 'primary'" @click="togglePause">
              {{ isPaused ? '继续' : '暂停' }}
            </el-button>
            <el-button @click="clearEvents">清空</el-button>
          </div>
        </div>
      </template>

      <el-empty v-if="!events.length" description="等待事件流入..." />
      <el-scrollbar v-else max-height="600px" always>
        <div v-for="e in events" :key="`${e.type}-${e.id}`" class="event-item">
          <div class="event-header">
            <el-tag size="small" :type="e.type === 'profile' ? 'success' : 'primary'">{{ e.event }}</el-tag>
            <span class="event-id">#{{ e.id }}</span>
            <span class="event-user">{{ e.distinct_id }}</span>
            <span class="event-url">{{ e.page_url || '-' }}</span>
            <span class="event-time">{{ formatTime(e.created_at || e.updated_at) }}</span>
          </div>
          <pre class="event-json">{{ formatJson(e.properties) }}</pre>
        </div>
      </el-scrollbar>
    </el-card>
  </div>
</template>

<style scoped>
.event-item {
  padding: 10px 12px;
  border-bottom: 1px solid #ebeef5;
  font-size: 13px;
}
.event-item:hover { background: #f5f7fa; }
.event-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 6px;
}
.event-id { color: #999; font-size: 12px; }
.event-user { color: #409eff; font-weight: 500; }
.event-url { color: #67c23a; flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.event-time { color: #999; font-size: 12px; }
.event-json {
  margin: 0;
  padding: 8px 12px;
  background: #f5f7fa;
  border-radius: 4px;
  font-size: 12px;
  line-height: 1.5;
  max-height: 200px;
  overflow: auto;
}
</style>
