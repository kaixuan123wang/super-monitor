import { get } from '@/utils/request';

export interface NetworkErrorRow {
  id: number;
  project_id: number;
  app_id: string;
  url: string;
  method: string;
  status?: number | null;
  request_body?: string | null;
  response_text?: string | null;
  duration?: number | null;
  error_type?: string | null;
  user_agent?: string | null;
  browser?: string | null;
  os?: string | null;
  device?: string | null;
  sdk_version?: string | null;
  release?: string | null;
  environment?: string | null;
  page_url?: string | null;
  distinct_id?: string | null;
  created_at: string;
}

export interface ListNetworkErrorsParams {
  project_id: number;
  page?: number;
  page_size?: number;
  url?: string;
  method?: string;
  status?: number;
  keyword?: string;
}

export interface NetworkStats {
  total: number;
  top_urls: Array<{ url: string; count: number }>;
  status_distribution: Array<{ status: number; count: number }>;
  method_distribution: Array<{ method: string; count: number }>;
  avg_duration: number;
}

export function listNetworkErrors(params: ListNetworkErrorsParams) {
  return get<{ list: NetworkErrorRow[]; total: number }>('/network', { params });
}

export function getNetworkStats(project_id: number, days?: number) {
  return get<NetworkStats>('/network/stats', { params: { project_id, days: days ?? 7 } });
}

export function getNetworkError(id: number) {
  return get<NetworkErrorRow>(`/network/${id}`);
}
