<script setup lang="ts">
import { ref, reactive } from 'vue';
import { useRouter } from 'vue-router';
import { ElMessage } from 'element-plus';
import { User, Lock } from '@element-plus/icons-vue';
import { login } from '@/api/auth';
import { useUserStore } from '@/stores/user';
import AuthLayout from '@/views/auth/AuthLayout.vue';

const router = useRouter();
const userStore = useUserStore();
const loading = ref(false);

const form = reactive({
  account: '',
  password: '',
});

const rules = {
  account: [{ required: true, message: '请输入账号', trigger: 'blur' }],
  password: [{ required: true, message: '请输入密码', trigger: 'blur' }],
};

const formRef = ref();

async function handleLogin() {
  const valid = await formRef.value?.validate().catch(() => false);
  if (!valid) return;

  loading.value = true;
  try {
    const res = await login({
      account: form.account,
      password: form.password,
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
      ElMessage.success('登录成功');
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
  <AuthLayout subtitle="前端监控平台">
    <el-form ref="formRef" :model="form" :rules="rules" size="large" @keyup.enter="handleLogin">
      <el-form-item prop="account">
        <el-input v-model="form.account" placeholder="账号 / 邮箱" :prefix-icon="User" clearable />
      </el-form-item>

      <el-form-item prop="password">
        <el-input
          v-model="form.password"
          type="password"
          placeholder="密码"
          :prefix-icon="Lock"
          show-password
          clearable
        />
      </el-form-item>

      <el-form-item>
        <el-button
          type="primary"
          style="width: 100%; height: 44px; font-size: 16px; border-radius: 6px"
          :loading="loading"
          @click="handleLogin"
        >
          登 录
        </el-button>
      </el-form-item>
    </el-form>

    <template #footer>
      <span>还没有账号？</span>
      <router-link to="/register">立即注册</router-link>
    </template>
  </AuthLayout>
</template>
