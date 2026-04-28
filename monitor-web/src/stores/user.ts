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
  const refreshToken = ref<string>(localStorage.getItem('__monitor_refresh_token') ?? '');
  const user = ref<CurrentUser | null>(null);

  const isLoggedIn = computed(() => !!token.value);

  function setToken(accessToken: string, refresh?: string) {
    token.value = accessToken;
    localStorage.setItem('__monitor_access_token', accessToken);
    if (refresh) {
      refreshToken.value = refresh;
      localStorage.setItem('__monitor_refresh_token', refresh);
    }
  }

  function setUser(u: CurrentUser | null) {
    user.value = u;
  }

  function reset() {
    token.value = '';
    refreshToken.value = '';
    user.value = null;
    localStorage.removeItem('__monitor_access_token');
    localStorage.removeItem('__monitor_refresh_token');
  }

  return { token, refreshToken, user, isLoggedIn, setToken, setUser, reset };
});
