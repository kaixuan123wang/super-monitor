import { get } from '@/utils/request';

/** 后端健康检查（Phase 1 验证用） */
export function checkHealth() {
  return get<{ status: string; version?: string }>('/health');
}
