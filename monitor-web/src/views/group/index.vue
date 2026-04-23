<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import {
  createGroup,
  deleteGroup,
  listGroups,
  updateGroup,
  type CreateGroupBody,
  type Group,
  type UpdateGroupBody,
} from '@/api/group';
import { listUsers, type User } from '@/api/user';

const loading = ref(false);
const groups = ref<Group[]>([]);
const users = ref<User[]>([]);
const total = ref(0);
const page = ref(1);
const pageSize = ref(20);
const keyword = ref('');

const dialogVisible = ref(false);
const saving = ref(false);
const editing = ref<Group | null>(null);
const form = reactive({
  name: '',
  description: '',
  owner_id: undefined as number | undefined,
});

async function fetchUsers() {
  const res = await listUsers({ page: 1, page_size: 200 });
  users.value = res.data?.list ?? [];
}

async function fetchGroups() {
  loading.value = true;
  try {
    const res = await listGroups({
      page: page.value,
      page_size: pageSize.value,
      keyword: keyword.value || undefined,
    });
    groups.value = res.data?.list ?? [];
    total.value = res.data?.total ?? 0;
  } finally {
    loading.value = false;
  }
}

function search() {
  page.value = 1;
  fetchGroups();
}

function resetForm() {
  editing.value = null;
  form.name = '';
  form.description = '';
  form.owner_id = users.value[0]?.id;
}

function openCreate() {
  resetForm();
  dialogVisible.value = true;
}

function openEdit(row: Group) {
  editing.value = row;
  form.name = row.name;
  form.description = row.description ?? '';
  form.owner_id = row.owner_id;
  dialogVisible.value = true;
}

async function saveGroup() {
  if (!form.name.trim()) {
    ElMessage.warning('请填写分组名称');
    return;
  }
  saving.value = true;
  try {
    if (editing.value) {
      const body: UpdateGroupBody = {
        name: form.name,
        description: form.description,
        owner_id: form.owner_id,
      };
      await updateGroup(editing.value.id, body);
      ElMessage.success('分组已更新');
    } else {
      const body: CreateGroupBody = {
        name: form.name,
        description: form.description,
        owner_id: form.owner_id,
      };
      await createGroup(body);
      ElMessage.success('分组已创建');
    }
    dialogVisible.value = false;
    fetchGroups();
  } finally {
    saving.value = false;
  }
}

async function handleDelete(row: Group) {
  await ElMessageBox.confirm(`确认删除分组「${row.name}」？`, '确认', { type: 'warning' });
  await deleteGroup(row.id);
  ElMessage.success('已删除');
  fetchGroups();
}

function ownerName(ownerId: number) {
  return users.value.find((u) => u.id === ownerId)?.username ?? String(ownerId);
}

function formatTime(value?: string | null) {
  return value ? value.slice(0, 19).replace('T', ' ') : '-';
}

onMounted(async () => {
  await fetchUsers();
  await fetchGroups();
});
</script>

<template>
  <el-card shadow="never">
    <template #header>
      <div class="group-page__header">
        <span>分组管理</span>
        <el-button type="primary" @click="openCreate">新建分组</el-button>
      </div>
    </template>

    <div class="group-page__toolbar">
      <el-input
        v-model="keyword"
        clearable
        placeholder="搜索分组名称"
        style="width: 260px"
        @keyup.enter="search"
        @clear="search"
      />
      <el-button type="primary" @click="search">查询</el-button>
      <el-button @click="fetchGroups">刷新</el-button>
    </div>

    <el-table v-loading="loading" :data="groups" border stripe>
      <el-table-column prop="id" label="ID" width="70" />
      <el-table-column prop="name" label="分组名称" min-width="180" />
      <el-table-column prop="description" label="描述" min-width="260" show-overflow-tooltip />
      <el-table-column label="负责人" width="140">
        <template #default="{ row }">{{ ownerName(row.owner_id) }}</template>
      </el-table-column>
      <el-table-column label="创建时间" width="170">
        <template #default="{ row }">{{ formatTime(row.created_at) }}</template>
      </el-table-column>
      <el-table-column label="操作" width="140" fixed="right">
        <template #default="{ row }">
          <el-button link type="primary" @click="openEdit(row)">编辑</el-button>
          <el-button link type="danger" @click="handleDelete(row)">删除</el-button>
        </template>
      </el-table-column>
    </el-table>

    <div class="group-page__pagination">
      <el-pagination
        v-model:current-page="page"
        :page-size="pageSize"
        :total="total"
        layout="total, prev, pager, next"
        @current-change="fetchGroups"
      />
    </div>
  </el-card>

  <el-dialog v-model="dialogVisible" :title="editing ? '编辑分组' : '新建分组'" width="520">
    <el-form :model="form" label-width="90px">
      <el-form-item label="分组名称" required>
        <el-input v-model="form.name" />
      </el-form-item>
      <el-form-item label="负责人">
        <el-select v-model="form.owner_id" filterable style="width: 100%">
          <el-option v-for="u in users" :key="u.id" :label="u.username" :value="u.id" />
        </el-select>
      </el-form-item>
      <el-form-item label="描述">
        <el-input v-model="form.description" type="textarea" :rows="3" />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="dialogVisible = false">取消</el-button>
      <el-button type="primary" :loading="saving" @click="saveGroup">保存</el-button>
    </template>
  </el-dialog>
</template>

<style scoped lang="scss">
.group-page__header,
.group-page__toolbar {
  display: flex;
  align-items: center;
}

.group-page__header {
  justify-content: space-between;
}

.group-page__toolbar {
  gap: 10px;
  margin-bottom: 12px;
}

.group-page__pagination {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}
</style>
