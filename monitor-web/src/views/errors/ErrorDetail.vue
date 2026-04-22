<script setup lang="ts">
import { computed } from 'vue';
import type { JsErrorRow } from '@/api/error';

interface Props {
  modelValue: boolean;
  row: JsErrorRow | null;
}

const props = defineProps<Props>();
const emit = defineEmits<{ (e: 'update:modelValue', v: boolean): void }>();

const visible = computed({
  get: () => props.modelValue,
  set: (v: boolean) => emit('update:modelValue', v),
});

function breadcrumbItems(): Array<Record<string, unknown>> {
  const b = props.row?.breadcrumb;
  return Array.isArray(b) ? (b as Array<Record<string, unknown>>) : [];
}

function formatBreadcrumbTime(ts: unknown): string {
  if (typeof ts !== 'number') return '';
  return new Date(ts).toLocaleTimeString();
}
</script>

<template>
  <el-drawer v-model="visible" :size="720" title="错误详情" direction="rtl">
    <template v-if="row">
      <el-descriptions :column="2" border>
        <el-descriptions-item label="ID">{{ row.id }}</el-descriptions-item>
        <el-descriptions-item label="类型">
          <el-tag size="small">{{ row.error_type }}</el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="时间" :span="2">
          {{ new Date(row.created_at).toLocaleString() }}
        </el-descriptions-item>
        <el-descriptions-item label="消息" :span="2">
          <pre class="error-detail__pre">{{ row.message }}</pre>
        </el-descriptions-item>
        <el-descriptions-item label="源文件" :span="2">
          <code>{{ row.source_url }}:{{ row.line }}:{{ row.column }}</code>
        </el-descriptions-item>
        <el-descriptions-item label="浏览器">{{ row.browser }} {{ row.browser_version }}</el-descriptions-item>
        <el-descriptions-item label="操作系统">{{ row.os }} {{ row.os_version }}</el-descriptions-item>
        <el-descriptions-item label="设备类型">{{ row.device_type }}</el-descriptions-item>
        <el-descriptions-item label="语言">{{ row.language }}</el-descriptions-item>
        <el-descriptions-item label="页面 URL" :span="2">{{ row.url }}</el-descriptions-item>
        <el-descriptions-item label="Referrer" :span="2">{{ row.referrer }}</el-descriptions-item>
        <el-descriptions-item label="指纹">
          <code>{{ row.fingerprint }}</code>
        </el-descriptions-item>
        <el-descriptions-item label="用户">{{ row.distinct_id || '-' }}</el-descriptions-item>
        <el-descriptions-item label="Release">{{ row.release || '-' }}</el-descriptions-item>
        <el-descriptions-item label="环境">{{ row.environment || '-' }}</el-descriptions-item>
      </el-descriptions>

      <h4 class="error-detail__title">堆栈</h4>
      <pre class="error-detail__pre error-detail__stack">{{ row.stack || '(无堆栈信息)' }}</pre>

      <h4 class="error-detail__title">面包屑</h4>
      <el-timeline v-if="breadcrumbItems().length" class="error-detail__timeline">
        <el-timeline-item
          v-for="(item, idx) in breadcrumbItems()"
          :key="idx"
          :timestamp="formatBreadcrumbTime(item.timestamp)"
          placement="top"
        >
          <el-tag size="small">{{ item.category }}</el-tag>
          <span class="error-detail__bc-msg">{{ item.message }}</span>
        </el-timeline-item>
      </el-timeline>
      <el-empty v-else description="该错误未携带面包屑" :image-size="80" />
    </template>
  </el-drawer>
</template>

<style scoped lang="scss">
.error-detail__pre {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
  font-family: var(--el-font-family-monospace, monospace);
  font-size: 12px;
}
.error-detail__stack {
  background-color: var(--el-bg-color-page);
  padding: 12px;
  border-radius: 4px;
  max-height: 320px;
  overflow: auto;
}
.error-detail__title {
  margin: 20px 0 10px;
  font-size: 14px;
  font-weight: 600;
}
.error-detail__timeline {
  padding-left: 10px;
}
.error-detail__bc-msg {
  margin-left: 8px;
  word-break: break-all;
  font-size: 13px;
}
</style>
