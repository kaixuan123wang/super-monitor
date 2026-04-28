import axios, {
  type AxiosInstance,
  type AxiosRequestConfig,
  type InternalAxiosRequestConfig,
} from 'axios';
import { ElMessage } from 'element-plus';
import router from '@/router';

export interface ApiResponse<T = unknown> {
  code: number;
  message: string;
  data: T;
  pagination?: {
    page: number;
    page_size: number;
    total: number;
    total_pages: number;
  };
}

const request: AxiosInstance = axios.create({
  baseURL: '/api',
  timeout: 15_000,
});

request.interceptors.request.use((config) => {
  const token = localStorage.getItem('__monitor_access_token');
  if (token) {
    config.headers.Authorization = `Bearer ${token}`;
  }
  return config;
});

function handleUnauthorized() {
  localStorage.removeItem('__monitor_access_token');
  localStorage.removeItem('__monitor_refresh_token');
  if (router.currentRoute.value.path !== '/login') {
    router.replace('/login');
  }
}

// Token 刷新状态：防止多个请求同时刷新
let isRefreshing = false;
let refreshSubscribers: Array<(token: string | null) => void> = [];

function subscribeTokenRefresh(cb: (token: string | null) => void) {
  refreshSubscribers.push(cb);
}

function onTokenRefreshSettled(newToken: string | null) {
  refreshSubscribers.forEach((cb) => cb(newToken));
  refreshSubscribers = [];
}

async function tryRefreshToken(): Promise<string | null> {
  const refreshToken = localStorage.getItem('__monitor_refresh_token');
  if (!refreshToken) return null;
  try {
    const resp = await axios.post('/api/auth/refresh', { refresh_token: refreshToken });
    const body = resp.data;
    if (body?.code === 0 && body?.data) {
      const { access_token, refresh_token: newRefreshToken } = body.data;
      localStorage.setItem('__monitor_access_token', access_token);
      if (newRefreshToken) {
        localStorage.setItem('__monitor_refresh_token', newRefreshToken);
      }
      return access_token;
    }
  } catch {
    // refresh 失败
  }
  return null;
}

request.interceptors.response.use(
  (response) => {
    const body = response.data as ApiResponse;
    if (body && body.code !== undefined && body.code !== 0) {
      if (body.code === 401) {
        handleUnauthorized();
      }
      ElMessage.error(body.message || '请求失败');
      return Promise.reject(body);
    }
    return response;
  },
  async (error) => {
    const status = error?.response?.status;
    const body = error?.response?.data as ApiResponse | undefined;
    const originalRequest = error.config as InternalAxiosRequestConfig & { _retry?: boolean };

    // 如果是 401 且未重试过，尝试 refresh token
    if ((status === 401 || body?.code === 401) && !originalRequest._retry) {
      originalRequest._retry = true;

      if (!isRefreshing) {
        isRefreshing = true;
        const newToken = await tryRefreshToken();
        isRefreshing = false;
        onTokenRefreshSettled(newToken);

        if (newToken) {
          originalRequest.headers.Authorization = `Bearer ${newToken}`;
          return request(originalRequest);
        }
        // refresh 也失败了，跳转登录
        handleUnauthorized();
      } else {
        // 正在刷新中，等待刷新完成后重试
        return new Promise((resolve, reject) => {
          subscribeTokenRefresh((newToken) => {
            if (!newToken) {
              handleUnauthorized();
              reject(error);
              return;
            }
            originalRequest.headers.Authorization = `Bearer ${newToken}`;
            resolve(request(originalRequest));
          });
        });
      }
    }

    ElMessage.error(body?.message || error.message || '网络错误');
    return Promise.reject(error);
  }
);

export function get<T>(url: string, config?: AxiosRequestConfig): Promise<ApiResponse<T>> {
  return request.get(url, config).then((r) => r.data);
}

export function post<T>(
  url: string,
  data?: unknown,
  config?: AxiosRequestConfig
): Promise<ApiResponse<T>> {
  return request.post(url, data, config).then((r) => r.data);
}

export function put<T>(
  url: string,
  data?: unknown,
  config?: AxiosRequestConfig
): Promise<ApiResponse<T>> {
  return request.put(url, data, config).then((r) => r.data);
}

export function del<T>(url: string, config?: AxiosRequestConfig): Promise<ApiResponse<T>> {
  return request.delete(url, config).then((r) => r.data);
}

export default request;
