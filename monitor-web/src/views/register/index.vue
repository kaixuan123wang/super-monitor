<script setup lang="ts">
import { ref, reactive } from 'vue';
import { useRouter } from 'vue-router';
import { ElMessage } from 'element-plus';
import { User, Message, Lock, OfficeBuilding } from '@element-plus/icons-vue';
import { register } from '@/api/auth';
import { useUserStore } from '@/stores/user';
import AuthLayout from '@/views/auth/AuthLayout.vue';

const router = useRouter();
const userStore = useUserStore();
const loading = ref(false);

const form = reactive({
  username: '',
  email: '',
  password: '',
  confirmPassword: '',
  groupName: '',
});

const validatePass = (_rule: unknown, value: string, callback: (error?: Error) => void) => {
  if (value === '') {
    callback(new Error('请输入密码'));
  } else if (value.length < 12) {
    callback(new Error('密码长度不能少于12位'));
  } else {
    callback();
  }
};

const validatePass2 = (_rule: unknown, value: string, callback: (error?: Error) => void) => {
  if (value === '') {
    callback(new Error('请再次输入密码'));
  } else if (value !== form.password) {
    callback(new Error('两次输入密码不一致'));
  } else {
    callback();
  }
};

const rules = {
  username: [{ required: true, message: '请输入用户名', trigger: 'blur' }],
  email: [
    { required: true, message: '请输入邮箱', trigger: 'blur' },
    { type: 'email' as const, message: '请输入正确的邮箱格式', trigger: 'blur' },
  ],
  password: [{ validator: validatePass, trigger: 'blur' }],
  confirmPassword: [{ validator: validatePass2, trigger: 'blur' }],
};

const formRef = ref();

async function handleRegister() {
  const valid = await formRef.value?.validate().catch(() => false);
  if (!valid) return;

  loading.value = true;
  try {
    const res = await register({
      username: form.username,
      email: form.email,
      password: form.password,
      group_name: form.groupName || undefined,
    });
    const data = res.data;
    if (data) {
      userStore.setToken(data.access_token, data.refresh_token);
      userStore.setUser({
        id: data.user.id,
        username: data.user.username,
        email: data.user.email,
        role: data.user.role,
        group_id: data.user.group_id ?? undefined,
      });
      ElMessage.success('注册成功');
      router.replace('/');
    }
  } catch {
    // 错误已由请求拦截器提示
  } finally {
    loading.value = false;
  }
}
</script>

<template>
  <AuthLayout subtitle="注册管理员账号" :box-width="420">
    <el-form ref="formRef" :model="form" :rules="rules" size="large" @keyup.enter="handleRegister">
      <el-form-item prop="username">
        <el-input v-model="form.username" placeholder="用户名" :prefix-icon="User" clearable />
      </el-form-item>

      <el-form-item prop="email">
        <el-input v-model="form.email" placeholder="邮箱" :prefix-icon="Message" clearable />
      </el-form-item>

      <el-form-item prop="password">
        <el-input
          v-model="form.password"
          type="password"
          placeholder="密码（至少12位）"
          :prefix-icon="Lock"
          show-password
          clearable
        />
      </el-form-item>

      <el-form-item prop="confirmPassword">
        <el-input
          v-model="form.confirmPassword"
          type="password"
          placeholder="确认密码"
          :prefix-icon="Lock"
          show-password
          clearable
        />
      </el-form-item>

      <el-form-item prop="groupName">
        <el-input
          v-model="form.groupName"
          placeholder="分组名称（可选，默认 Default）"
          :prefix-icon="OfficeBuilding"
          clearable
        />
      </el-form-item>

      <el-form-item>
        <el-button
          type="primary"
          style="width: 100%; height: 44px; font-size: 16px; border-radius: 6px"
          :loading="loading"
          @click="handleRegister"
        >
          注 册
        </el-button>
      </el-form-item>
    </el-form>

    <template #footer>
      <span>已有账号？</span>
      <router-link to="/login">立即登录</router-link>
    </template>
  </AuthLayout>
</template>
