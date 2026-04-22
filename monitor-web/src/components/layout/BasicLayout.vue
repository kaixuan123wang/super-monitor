<script setup lang="ts">
import { computed } from 'vue';
import { useRoute, useRouter } from 'vue-router';

const route = useRoute();
const router = useRouter();

const menuRoutes = computed(() => {
  const root = router.options.routes.find((r) => r.path === '/');
  return root?.children ?? [];
});

const activeMenu = computed(() => route.path);
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
    padding: 0 24px;
  }

  &__title {
    font-size: 16px;
    font-weight: 500;
  }

  &__main {
    padding: 24px;
    background-color: var(--el-bg-color-page);
  }
}
</style>
