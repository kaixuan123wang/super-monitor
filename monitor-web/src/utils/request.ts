import axios, { type AxiosInstance, type AxiosRequestConfig } from 'axios';
import { ElMessage } from 'element-plus';

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
  // Phase 2 起注入 Authorization: Bearer <access_token>
  return config;
});

request.interceptors.response.use(
  (response) => {
    const body = response.data as ApiResponse;
    if (body && body.code !== undefined && body.code !== 0) {
      ElMessage.error(body.message || '请求失败');
      return Promise.reject(body);
    }
    return response;
  },
  (error) => {
    const status = error?.response?.status;
    if (status === 401) {
      // Phase 2 起接入刷新 token / 跳转登录
    }
    ElMessage.error(error?.response?.data?.message || error.message || '网络错误');
    return Promise.reject(error);
  }
);

export function get<T>(url: string, config?: AxiosRequestConfig): Promise<ApiResponse<T>> {
  return request.get(url, config).then((r) => r.data);
}

export function post<T>(url: string, data?: unknown, config?: AxiosRequestConfig): Promise<ApiResponse<T>> {
  return request.post(url, data, config).then((r) => r.data);
}

export default request;
