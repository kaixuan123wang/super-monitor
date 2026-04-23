<script setup lang="ts">
import { ref, onMounted, watch } from 'vue';
import { useProjectStore } from '@/stores/project';
import { listSourceMaps, deleteSourceMap, uploadSourceMap, type SourceMap } from '@/api/sourcemap';
import { ElMessage, ElMessageBox } from 'element-plus';
import type { UploadFile } from 'element-plus';

const projectStore = useProjectStore();

const loading = ref(false);
const list = ref<SourceMap[]>([]);
const total = ref(0);
const page = ref(1);
const pageSize = ref(20);
const releaseFilter = ref('');

const uploadVisible = ref(false);
const uploadRelease = ref('');
const uploadFile = ref<File | null>(null);
const uploadProgress = ref(0);
const uploading = ref(false);

async function fetchList() {
  if (!projectStore.currentId) return;
  loading.value = true;
  try {
    const res = await listSourceMaps({
      project_id: projectStore.currentId,
      release: releaseFilter.value || undefined,
      page: page.value,
      page_size: pageSize.value,
    });
    list.value = res.data?.list ?? [];
    total.value = res.data?.total ?? 0;
  } finally {
    loading.value = false;
  }
}

async function handleDelete(row: SourceMap) {
  await ElMessageBox.confirm(`确认删除 ${row.filename} ?`, '确认', { type: 'warning' });
  await deleteSourceMap(row.id);
  ElMessage.success('已删除');
  fetchList();
}

function onFileChange(file: UploadFile) {
  uploadFile.value = file.raw ?? null;
  return false;
}

async function doUpload() {
  if (!projectStore.currentId) return;
  if (!uploadRelease.value) { ElMessage.warning('请输入 Release 版本号'); return; }
  if (!uploadFile.value) { ElMessage.warning('请选择文件'); return; }

  uploading.value = true;
  uploadProgress.value = 0;
  try {
    await uploadSourceMap(
      projectStore.currentId,
      uploadRelease.value,
      uploadFile.value,
      (pct) => { uploadProgress.value = pct; },
    );
    ElMessage.success('上传成功');
    uploadVisible.value = false;
    uploadRelease.value = '';
    uploadFile.value = null;
    fetchList();
  } catch (e: unknown) {
    const msg = (e as { response?: { data?: { message?: string } } })?.response?.data?.message;
    ElMessage.error(msg ?? '上传失败');
  } finally {
    uploading.value = false;
  }
}

function formatSize(bytes: number | null) {
  if (!bytes) return '-';
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(2)} MB`;
}

onMounted(async () => {
  if (!projectStore.list.length) await projectStore.fetchAll();
  fetchList();
});

watch(() => projectStore.currentId, () => { page.value = 1; fetchList(); });
</script>

<template>
  <div>
    <el-card shadow="never">
      <template #header>
        <div style="display:flex;justify-content:space-between;align-items:center">
          <span>Source Map 管理</span>
          <el-button type="primary" @click="uploadVisible = true">上传 Source Map</el-button>
        </div>
      </template>

      <el-form inline style="margin-bottom:12px">
        <el-form-item label="Release">
          <el-input v-model="releaseFilter" placeholder="过滤版本号" clearable style="width:200px" @change="fetchList" />
        </el-form-item>
        <el-form-item>
          <el-button @click="fetchList">查询</el-button>
        </el-form-item>
      </el-form>

      <el-table :data="list" v-loading="loading" border>
        <el-table-column prop="id" label="ID" width="70" />
        <el-table-column prop="release" label="Release" width="160" />
        <el-table-column prop="filename" label="文件名" min-width="200" show-overflow-tooltip />
        <el-table-column label="大小" width="100">
          <template #default="{ row }">{{ formatSize(row.file_size) }}</template>
        </el-table-column>
        <el-table-column label="Hash" width="150">
          <template #default="{ row }">
            <span style="font-family:monospace;font-size:12px">{{ row.content_hash?.slice(0, 12) ?? '-' }}</span>
          </template>
        </el-table-column>
        <el-table-column label="上传时间" width="170">
          <template #default="{ row }">{{ row.uploaded_at?.slice(0, 19).replace('T', ' ') }}</template>
        </el-table-column>
        <el-table-column label="操作" width="80" fixed="right">
          <template #default="{ row }">
            <el-button link type="danger" @click="handleDelete(row)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>

      <div style="margin-top:12px;text-align:right">
        <el-pagination
          v-model:current-page="page"
          :page-size="pageSize"
          :total="total"
          layout="total, prev, pager, next"
          @current-change="fetchList"
        />
      </div>
    </el-card>

    <!-- 上传对话框 -->
    <el-dialog v-model="uploadVisible" title="上传 Source Map" width="480px">
      <el-form label-width="100px">
        <el-form-item label="Release 版本" required>
          <el-input v-model="uploadRelease" placeholder="例如: v1.2.3 或 git commit hash" />
        </el-form-item>
        <el-form-item label="Source Map 文件" required>
          <el-upload
            :auto-upload="false"
            :limit="1"
            accept=".map"
            :on-change="onFileChange"
          >
            <el-button>选择 .map 文件</el-button>
            <template #tip>
              <div class="el-upload__tip">仅支持 .map 格式的 Source Map 文件</div>
            </template>
          </el-upload>
        </el-form-item>
        <el-form-item v-if="uploading" label="上传进度">
          <el-progress :percentage="uploadProgress" style="width:100%" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="uploadVisible = false">取消</el-button>
        <el-button type="primary" :loading="uploading" @click="doUpload">上传</el-button>
      </template>
    </el-dialog>
  </div>
</template>
