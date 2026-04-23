<script setup lang="ts">
import { ref, reactive } from 'vue';
import { useRouter } from 'vue-router';
import { ElMessage } from 'element-plus';
import { User, Lock } from '@element-plus/icons-vue';
import { login } from '@/api/auth';
import { useUserStore } from '@/stores/user';

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
      localStorage.setItem('__monitor_access_token', data.access_token);
      localStorage.setItem('__monitor_refresh_token', data.refresh_token);
      userStore.setToken(data.access_token);
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
  <div class="login-page">
    <div class="login-box">
      <div class="login-header">
        <h1 class="login-title">JS Monitor</h1>
        <p class="login-subtitle">前端监控平台</p>
      </div>

      <el-form
        ref="formRef"
        :model="form"
        :rules="rules"
        size="large"
        @keyup.enter="handleLogin"
      >
        <el-form-item prop="account">
          <el-input
            v-model="form.account"
            placeholder="账号 / 邮箱"
            :prefix-icon="User"
            clearable
          />
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
            class="login-btn"
            :loading="loading"
            @click="handleLogin"
          >
            登 录
          </el-button>
        </el-form-item>
      </el-form>

      <div class="login-footer">
        <span>还没有账号？</span>
        <router-link to="/register">立即注册</router-link>
      </div>
    </div>
  </div>
</template>

<style scoped lang="scss">
.login-page {
  height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #1a2a3a 0%, #0d1b2a 100%);
}

.login-box {
  width: 400px;
  padding: 40px;
  background: #fff;
  border-radius: 12px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.25);
}

.login-header {
  text-align: center;
  margin-bottom: 32px;
}

.login-title {
  font-size: 28px;
  font-weight: 700;
  color: #001529;
  margin: 0 0 8px;
  letter-spacing: 1px;
}

.login-subtitle {
  font-size: 14px;
  color: #8c8c8c;
  margin: 0;
}

.login-btn {
  width: 100%;
  height: 44px;
  font-size: 16px;
  border-radius: 6px;
}

.login-footer {
  text-align: center;
  margin-top: 16px;
  font-size: 14px;
  color: #8c8c8c;

  a {
    color: var(--el-color-primary);
    margin-left: 4px;

    &:hover {
      text-decoration: underline;
    }
  }
}
</style>
