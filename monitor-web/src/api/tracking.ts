import { get, post, put, del } from '@/utils/request';

// ── 已采集事件 ─────────────────────────────────────────────────────────────────

export interface EventItem {
  event: string;
  category: 'auto' | 'custom';
  total_count: number;
  unique_users: number;
  last_seen: string;
}

export interface EventDetail {
  event: string;
  total_count: number;
  unique_users: number;
  properties: string[];
  trend: Array<{ date: string; count: number }>;
}

export interface EventListParams {
  project_id: number;
  keyword?: string;
  page?: number;
  page_size?: number;
}

export function listTrackEvents(params: EventListParams) {
  return get<{ list: EventItem[]; total: number }>('/track/events', { params });
}

export function getEventDetail(eventName: string, project_id: number) {
  return get<EventDetail>(`/track/events/${encodeURIComponent(eventName)}`, {
    params: { project_id },
  });
}

// ── 自定义事件定义 CRUD ────────────────────────────────────────────────────────

export interface PropertyDef {
  name: string;
  type: string;
  description?: string;
  required?: boolean;
}

export interface EventDefinition {
  id: number;
  project_id: number;
  event_name: string;
  display_name?: string | null;
  category?: string | null;
  description?: string | null;
  properties?: PropertyDef[] | null;
  status: string;
  created_at: string;
  updated_at: string;
}

export interface DefListParams {
  project_id: number;
  keyword?: string;
  page?: number;
  page_size?: number;
}

export interface CreateDefBody {
  project_id: number;
  event_name: string;
  display_name?: string;
  category?: string;
  description?: string;
  properties?: PropertyDef[];
}

export interface UpdateDefBody {
  display_name?: string;
  category?: string;
  description?: string;
  properties?: PropertyDef[];
  status?: string;
}

export function listDefinitions(params: DefListParams) {
  return get<{ list: EventDefinition[]; total: number }>('/track/definitions', { params });
}

export function createDefinition(body: CreateDefBody) {
  return post<EventDefinition>('/track/definitions', body);
}

export function updateDefinition(id: number, body: UpdateDefBody) {
  return put<EventDefinition>(`/track/definitions/${id}`, body);
}

export function deleteDefinition(id: number) {
  return del<{ deleted: number }>(`/track/definitions/${id}`);
}

// ── 属性汇总 ──────────────────────────────────────────────────────────────────

export interface PropItem {
  name: string;
  prop_type: string;
  description?: string | null;
  required: boolean;
  event_names: string[];
}

export function listProperties(project_id: number) {
  return get<{ list: PropItem[]; total: number }>('/track/properties', {
    params: { project_id },
  });
}

// ── 事件分析 ──────────────────────────────────────────────────────────────────

export interface AnalysisSeries {
  name: string;
  data: number[];
}

export interface AnalysisResult {
  dates: string[];
  series: AnalysisSeries[];
}

export interface AnalysisParams {
  project_id: number;
  events: string;
  days?: number;
  metric?: 'pv' | 'uv';
  group_by?: string;
}

export function getEventAnalysis(params: AnalysisParams) {
  return get<AnalysisResult>('/track/analysis', { params });
}
