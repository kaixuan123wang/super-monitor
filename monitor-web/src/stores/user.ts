import { defineStore } from 'pinia';
import { ref, computed } from 'vue';

export interface CurrentUser {
  id: number;
  username: string;
  email: string;
  role: string;
  group_id?: number;
}

export const useUserStore = defineStore('user', () => {
  const token = ref<string>(localStorage.getItem('__monitor_access_token') ?? '');
  const user = ref<CurrentUser | null>(null);

  const isLoggedIn = computed(() => !!token.value);

  function setToken(t: string) {
    token.value = t;
  }

  function setUser(u: CurrentUser | null) {
    user.value = u;
  }

  function reset() {
    token.value = '';
    user.value = null;
    localStorage.removeItem('__monitor_access_token');
    localStorage.removeItem('__monitor_refresh_token');
  }

  return { token, user, isLoggedIn, setToken, setUser, reset };
});
