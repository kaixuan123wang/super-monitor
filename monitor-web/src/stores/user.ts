import { defineStore } from 'pinia';
import { ref } from 'vue';

export interface CurrentUser {
  id: number;
  username: string;
  email: string;
  role: string;
  group_id?: number;
}

/** 当前登录用户状态（Phase 2 接入登录后填充） */
export const useUserStore = defineStore('user', () => {
  const token = ref<string>('');
  const user = ref<CurrentUser | null>(null);

  function setToken(t: string) {
    token.value = t;
  }

  function setUser(u: CurrentUser | null) {
    user.value = u;
  }

  function reset() {
    token.value = '';
    user.value = null;
  }

  return { token, user, setToken, setUser, reset };
});
