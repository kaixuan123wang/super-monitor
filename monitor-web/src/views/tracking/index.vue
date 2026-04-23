<script setup lang="ts">
import { useRoute, useRouter } from 'vue-router';
import { computed } from 'vue';

const route = useRoute();
const router = useRouter();

const tabs = [
  { name: 'TrackEvents', path: '/tracking/events', label: '事件管理' },
  { name: 'TrackAnalysis', path: '/tracking/analysis', label: '事件分析' },
  { name: 'TrackFunnel', path: '/tracking/funnel', label: '漏斗分析' },
  { name: 'TrackRetention', path: '/tracking/retention', label: '留存分析' },
  { name: 'TrackUsers', path: '/tracking/users', label: '用户画像' },
  { name: 'TrackDebug', path: '/tracking/debug', label: '实时事件流' },
];

const activeTab = computed(() => {
  if (route.name === 'TrackEventDetail') return 'TrackEvents';
  return String(route.name);
});
</script>

<template>
  <div class="tracking-layout">
    <el-tabs
      :model-value="activeTab"
      class="tracking-layout__tabs"
      @tab-click="(tab) => router.push(tabs.find((t) => t.name === tab.paneName)?.path ?? '/tracking/events')"
    >
      <el-tab-pane
        v-for="t in tabs"
        :key="t.name"
        :label="t.label"
        :name="t.name"
      />
    </el-tabs>
    <router-view />
  </div>
</template>

<style scoped lang="scss">
.tracking-layout {
  display: flex;
  flex-direction: column;
  gap: 0;

  &__tabs {
    margin-bottom: 16px;
  }
}
</style>
