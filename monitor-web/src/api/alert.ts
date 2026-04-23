import { get, post, put, del } from '@/utils/request';

export interface AlertRule {
  id: number;
  project_id: number;
  name: string;
  rule_type: 'error_spike' | 'failure_rate' | 'new_error' | 'p0_error' | 'error_trend';
  threshold: number | null;
  interval_minutes: number;
  is_enabled: boolean;
  webhook_url: string | null;
  email: string | null;
  created_at: string;
  updated_at: string;
}

export interface AlertLog {
  id: number;
  rule_id: number;
  project_id: number;
  alert_type: string;
  severity: string;
  content: string;
  error_count: number | null;
  sample_errors: unknown[] | null;
  status: string;
  created_at: string;
}

export interface CreateRuleBody {
  project_id: number;
  name: string;
  rule_type: AlertRule['rule_type'];
  threshold?: number;
  interval_minutes?: number;
  webhook_url?: string;
  email?: string;
}

export interface UpdateRuleBody {
  name?: string;
  threshold?: number;
  interval_minutes?: number;
  is_enabled?: boolean;
  webhook_url?: string;
  email?: string;
}

export interface LogsParams {
  project_id: number;
  page?: number;
  page_size?: number;
  status?: string;
}

export function listRules(project_id: number) {
  return get<{ list: AlertRule[]; total: number }>('/alerts/rules', { params: { project_id } });
}

export function createRule(body: CreateRuleBody) {
  return post<AlertRule>('/alerts/rules', body);
}

export function updateRule(id: number, body: UpdateRuleBody) {
  return put<AlertRule>(`/alerts/rules/${id}`, body);
}

export function deleteRule(id: number) {
  return del<{ deleted: number }>(`/alerts/rules/${id}`);
}

export function listLogs(params: LogsParams) {
  return get<{ list: AlertLog[]; total: number }>('/alerts/logs', { params });
}

export function getLog(id: number) {
  return get<AlertLog>(`/alerts/logs/${id}`);
}
