import { get } from '@/utils/request';

export interface JsErrorRow {
  id: number;
  project_id: number;
  app_id: string;
  message: string;
  stack?: string | null;
  error_type: string;
  source_url?: string | null;
  line?: number | null;
  column?: number | null;
  user_agent?: string | null;
  browser?: string | null;
  browser_version?: string | null;
  os?: string | null;
  os_version?: string | null;
  device?: string | null;
  device_type?: string | null;
  url?: string | null;
  referrer?: string | null;
  viewport?: string | null;
  screen_resolution?: string | null;
  language?: string | null;
  timezone?: string | null;
  breadcrumb?: Array<Record<string, unknown>> | null;
  extra?: Record<string, unknown> | null;
  fingerprint?: string | null;
  sdk_version?: string | null;
  release?: string | null;
  environment?: string | null;
  is_ai_analyzed: boolean;
  distinct_id?: string | null;
  created_at: string;
}

export interface ListErrorsParams {
  project_id: number;
  page?: number;
  page_size?: number;
  error_type?: string;
  fingerprint?: string;
  browser?: string;
  os?: string;
  release?: string;
  environment?: string;
  keyword?: string;
}

export function listErrors(params: ListErrorsParams) {
  return get<{ list: JsErrorRow[]; total: number }>('/errors', { params });
}

export function getError(id: number) {
  return get<JsErrorRow>(`/errors/${id}`);
}
