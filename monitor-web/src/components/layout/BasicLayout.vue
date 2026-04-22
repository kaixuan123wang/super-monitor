<script setup lang="ts">
import { computed, onMounted, onUnmounted, watch, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useProjectStore } from '@/stores/project';
import { SSEClient } from '@/utils/sse';
import { ElNotification } from 'element-plus';

const route = useRoute();
const router = useRouter();
const projectStore = useProjectStore();

const menuRoutes = computed(() => {
  const root = router.options.routes.find((r) => r.path === '/');
  return root?.children ?? [];
});

const activeMenu = computed(() => route.path);

const sseConnected = ref(false);
let sseClient: SSEClient | null = null;

function connectSSE(projectId: number) {
  sseClient?.disconnect();
  sseClient = new SSEClient(`/api/dashboard/realtime?project_id=${projectId}`)
    .on('init', () => {
      sseConnected.value = true;
    })
    .on('error', (data: unknown) => {
      const d = data as { message?: string; error_type?: string };
      ElNotification({
        title: '新错误',
        message: d.message ?? '未知错误',
        type: 'error',
        duration: 4500,
      });
    })
    .on('alert', (data: unknown) => {
      const d = data as { alert_content?: string; severity?: string };
      ElNotification({
        title: `告警 [${d.severity ?? 'info'}]`,
        message: d.alert_content ?? '',
        type: 'warning',
        duration: 6000,
      });
    })
    .on('heartbeat', () => {
      sseConnected.value = true;
    });

  sseClient.connect();
}

onMounted(async () => {
  if (!projectStore.list.length) {
    await projectStore.fetchAll();
  }
  if (projectStore.currentId) {
    connectSSE(projectStore.currentId);
  }
});

onUnmounted(() => {
  sseClient?.disconnect();
  sseConnected.value = false;
});

watch(
  () => projectStore.currentId,
  (id) => {
    if (id) {
      connectSSE(id);
    } else {
      sseClient?.disconnect();
      sseConnected.value = false;
    }
  }
);
</script>

<template>
  <el-container class="basic-layout">
    <el-aside width="220px" class="basic-layout__aside">
      <div class="basic-layout__logo">JS Monitor</div>
      <el-menu
        :default-active="activeMenu"
        router
        class="basic-layout__menu"
        background-color="#001529"
        text-color="#e6e9ef"
        active-text-color="#409eff"
      >
        <el-menu-item
          v-for="item in menuRoutes"
          :key="item.path"
          :index="`/${item.path}`"
        >
          <span>{{ item.meta?.title ?? item.name }}</span>
        </el-menu-item>
      </el-menu>
    </el-aside>
    <el-container>
      <el-header class="basic-layout__header">
        <span class="basic-layout__title">{{ route.meta?.title ?? '监控平台' }}</span>
        <div class="basic-layout__right">
          <!-- SSE 状态指示 -->
          <el-tooltip :content="sseConnected ? 'SSE 实时连接中' : '实时连接断开'" placement="bottom">
            <span class="sse-dot" :class="{ 'sse-dot--on': sseConnected }" />
          </el-tooltip>

          <el-select
            v-if="projectStore.list.length"
            :model-value="projectStore.currentId ?? undefined"
            placeholder="选择项目"
            size="default"
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
          <el-button v-else type="primary" size="small" @click="router.push('/projects')">
            新建项目
          </el-button>
        </div>
      </el-header>
      <el-main class="basic-layout__main">
        <router-view />
      </el-main>
    </el-container>
  </el-container>
</template>

<style scoped lang="scss">
.basic-layout {
  height: 100vh;

  &__aside {
    background-color: #001529;
    color: #fff;
    display: flex;
    flex-direction: column;
  }

  &__logo {
    height: 56px;
    line-height: 56px;
    text-align: center;
    font-size: 18px;
    font-weight: 600;
    color: #fff;
    letter-spacing: 1px;
  }

  &__menu {
    border-right: 0;
    flex: 1;
  }

  &__header {
    background-color: #fff;
    border-bottom: 1px solid var(--el-border-color-light);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 24px;
  }

  &__title {
    font-size: 16px;
    font-weight: 500;
  }

  &__right {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  &__main {
    padding: 24px;
    background-color: var(--el-bg-color-page);
  }
}

.sse-dot {
  display: inline-block;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #dcdfe6;
  transition: background 0.3s;

  &--on {
    background: #67c23a;
    box-shadow: 0 0 4px #67c23a;
  }
}
</style>
