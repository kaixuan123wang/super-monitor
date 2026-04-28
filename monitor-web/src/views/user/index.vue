<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import { listGroups, type Group } from '@/api/group';
import {
  createUser,
  deleteUser,
  listUsers,
  updateUser,
  type CreateUserBody,
  type UpdateUserBody,
  type User,
} from '@/api/user';

const loading = ref(false);
const users = ref<User[]>([]);
const total = ref(0);
const page = ref(1);
const pageSize = ref(20);
const keyword = ref('');
const roleFilter = ref('');
const groupFilter = ref<number | undefined>();
const groups = ref<Group[]>([]);

const dialogVisible = ref(false);
const saving = ref(false);
const editing = ref<User | null>(null);
const form = reactive({
  username: '',
  email: '',
  password: '',
  role: 'member',
  group_id: undefined as number | undefined,
  avatar: '',
});

const roleOptions = ['super_admin', 'admin', 'owner', 'member', 'readonly'];

async function fetchGroups() {
  const res = await listGroups({ page: 1, page_size: 200 });
  groups.value = res.data?.list ?? [];
}

async function fetchUsers() {
  loading.value = true;
  try {
    const res = await listUsers({
      page: page.value,
      page_size: pageSize.value,
      keyword: keyword.value || undefined,
      role: roleFilter.value || undefined,
      group_id: groupFilter.value,
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

function resetForm() {
  editing.value = null;
  form.username = '';
  form.email = '';
  form.password = '';
  form.role = 'member';
  form.group_id = undefined;
  form.avatar = '';
}

function openCreate() {
  resetForm();
  dialogVisible.value = true;
}

function openEdit(row: User) {
  editing.value = row;
  form.username = row.username;
  form.email = row.email;
  form.password = '';
  form.role = row.role;
  form.group_id = row.group_id ?? undefined;
  form.avatar = row.avatar ?? '';
  dialogVisible.value = true;
}

async function saveUser() {
  if (!form.username.trim() || !form.email.trim()) {
    ElMessage.warning('请填写用户名和邮箱');
    return;
  }
  if (!editing.value && form.password.length < 12) {
    ElMessage.warning('新用户密码至少 12 位');
    return;
  }
  if (editing.value && form.password && form.password.length < 12) {
    ElMessage.warning('新密码至少 12 位');
    return;
  }

  saving.value = true;
  try {
    if (editing.value) {
      const body: UpdateUserBody = {
        username: form.username,
        email: form.email,
        role: form.role,
        group_id: form.group_id,
        avatar: form.avatar || undefined,
      };
      if (form.password) body.password = form.password;
      await updateUser(editing.value.id, body);
      ElMessage.success('用户已更新');
    } else {
      const body: CreateUserBody = {
        username: form.username,
        email: form.email,
        password: form.password,
        role: form.role,
        group_id: form.group_id,
        avatar: form.avatar || undefined,
      };
      await createUser(body);
      ElMessage.success('用户已创建');
    }
    dialogVisible.value = false;
    fetchUsers();
  } finally {
    saving.value = false;
  }
}

async function handleDelete(row: User) {
  try {
    await ElMessageBox.confirm(`确认删除用户「${row.username}」？`, '确认', { type: 'warning' });
    await deleteUser(row.id);
    ElMessage.success('已删除');
    fetchUsers();
  } catch {
    // 用户取消或请求失败
  }
}

function groupName(groupId?: number | null) {
  if (!groupId) return '-';
  return groups.value.find((g) => g.id === groupId)?.name ?? String(groupId);
}

function formatTime(value?: string | null) {
  return value ? value.slice(0, 19).replace('T', ' ') : '-';
}

onMounted(async () => {
  await fetchGroups();
  await fetchUsers();
});
</script>

<template>
  <el-card shadow="never">
    <template #header>
      <div class="user-page__header">
        <span>用户管理</span>
        <el-button type="primary" @click="openCreate">新建用户</el-button>
      </div>
    </template>

    <el-form inline class="user-page__filters">
      <el-form-item label="关键字">
        <el-input
          v-model="keyword"
          clearable
          placeholder="用户名 / 邮箱"
          style="width: 220px"
          @keyup.enter="search"
          @clear="search"
        />
      </el-form-item>
      <el-form-item label="角色">
        <el-select
          v-model="roleFilter"
          clearable
          placeholder="全部"
          style="width: 150px"
          @change="search"
        >
          <el-option v-for="role in roleOptions" :key="role" :label="role" :value="role" />
        </el-select>
      </el-form-item>
      <el-form-item label="分组">
        <el-select
          v-model="groupFilter"
          clearable
          placeholder="全部"
          style="width: 180px"
          @change="search"
        >
          <el-option v-for="g in groups" :key="g.id" :label="g.name" :value="g.id" />
        </el-select>
      </el-form-item>
      <el-form-item>
        <el-button type="primary" @click="search">查询</el-button>
        <el-button @click="fetchUsers">刷新</el-button>
      </el-form-item>
    </el-form>

    <el-table v-loading="loading" :data="users" border stripe>
      <el-table-column prop="id" label="ID" width="70" />
      <el-table-column prop="username" label="用户名" min-width="140" />
      <el-table-column prop="email" label="邮箱" min-width="220" />
      <el-table-column label="角色" width="130">
        <template #default="{ row }">
          <el-tag size="small" :type="row.role === 'super_admin' ? 'danger' : 'info'">
            {{ row.role }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column label="分组" min-width="140">
        <template #default="{ row }">{{ groupName(row.group_id) }}</template>
      </el-table-column>
      <el-table-column label="最近登录" width="170">
        <template #default="{ row }">{{ formatTime(row.last_login_at) }}</template>
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

    <div class="user-page__pagination">
      <el-pagination
        v-model:current-page="page"
        :page-size="pageSize"
        :total="total"
        layout="total, prev, pager, next"
        @current-change="fetchUsers"
      />
    </div>
  </el-card>

  <el-dialog v-model="dialogVisible" :title="editing ? '编辑用户' : '新建用户'" width="520">
    <el-form :model="form" label-width="90px">
      <el-form-item label="用户名" required>
        <el-input v-model="form.username" />
      </el-form-item>
      <el-form-item label="邮箱" required>
        <el-input v-model="form.email" />
      </el-form-item>
      <el-form-item :label="editing ? '新密码' : '密码'" :required="!editing">
        <el-input v-model="form.password" type="password" show-password />
      </el-form-item>
      <el-form-item label="角色">
        <el-select v-model="form.role" style="width: 100%">
          <el-option v-for="role in roleOptions" :key="role" :label="role" :value="role" />
        </el-select>
      </el-form-item>
      <el-form-item label="分组">
        <el-select v-model="form.group_id" clearable style="width: 100%">
          <el-option v-for="g in groups" :key="g.id" :label="g.name" :value="g.id" />
        </el-select>
      </el-form-item>
      <el-form-item label="头像 URL">
        <el-input v-model="form.avatar" />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="dialogVisible = false">取消</el-button>
      <el-button type="primary" :loading="saving" @click="saveUser">保存</el-button>
    </template>
  </el-dialog>
</template>

<style scoped lang="scss">
.user-page__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.user-page__filters {
  margin-bottom: 12px;
}

.user-page__pagination {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}
</style>
