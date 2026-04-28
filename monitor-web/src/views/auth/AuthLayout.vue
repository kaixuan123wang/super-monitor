<script setup lang="ts">
interface Props {
  title?: string;
  subtitle?: string;
  boxWidth?: number;
}
withDefaults(defineProps<Props>(), {
  title: 'JS Monitor',
  subtitle: '',
  boxWidth: 400,
});
</script>

<template>
  <div class="auth-page">
    <div class="auth-box" :style="{ width: boxWidth + 'px' }">
      <div class="auth-header">
        <h1 class="auth-title">{{ title }}</h1>
        <p v-if="subtitle" class="auth-subtitle">{{ subtitle }}</p>
      </div>

      <slot />

      <div class="auth-footer">
        <slot name="footer" />
      </div>
    </div>
  </div>
</template>

<style scoped lang="scss">
.auth-page {
  height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #1a2a3a 0%, #0d1b2a 100%);
}

.auth-box {
  padding: 40px;
  background: #fff;
  border-radius: 12px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.25);
}

.auth-header {
  text-align: center;
  margin-bottom: 32px;
}

.auth-title {
  font-size: 28px;
  font-weight: 700;
  color: #001529;
  margin: 0 0 8px;
  letter-spacing: 1px;
}

.auth-subtitle {
  font-size: 14px;
  color: #8c8c8c;
  margin: 0;
}

.auth-footer {
  text-align: center;
  margin-top: 16px;
  font-size: 14px;
  color: #8c8c8c;

  :deep(a) {
    color: var(--el-color-primary);
    margin-left: 4px;

    &:hover {
      text-decoration: underline;
    }
  }
}
</style>
