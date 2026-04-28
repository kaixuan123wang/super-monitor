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

// ── 漏斗分析 ──────────────────────────────────────────────────────────────────

export interface FunnelStep {
  event: string;
  display_name?: string;
  filters?: Record<string, unknown>;
}

export interface TrackFunnel {
  id: number;
  project_id: number;
  name: string;
  description: string | null;
  steps: FunnelStep[];
  window_minutes: number;
  created_at: string;
  updated_at: string;
}

export interface FunnelStepResult {
  event: string;
  display_name: string;
  user_count: number;
  conversion_rate: number;
  avg_time_to_next_ms: number | null;
}

export interface FunnelAnalysisResult {
  steps: FunnelStepResult[];
  overall_conversion: number;
  breakdown?: Array<{
    group: string;
    steps: FunnelStepResult[];
    overall_conversion: number;
  }>;
}

export function listFunnels(project_id: number) {
  return get<{ list: TrackFunnel[]; total: number }>('/tracking/funnels', {
    params: { project_id },
  });
}

export function createFunnel(body: {
  project_id: number;
  name: string;
  description?: string;
  steps: FunnelStep[];
  window_minutes?: number;
}) {
  return post<TrackFunnel>('/tracking/funnels', body);
}

export function getFunnel(id: number) {
  return get<TrackFunnel>(`/tracking/funnels/${id}`);
}

export function updateFunnel(
  id: number,
  body: Partial<Pick<TrackFunnel, 'name' | 'description' | 'steps' | 'window_minutes'>>
) {
  return put<TrackFunnel>(`/tracking/funnels/${id}`, body);
}

export function deleteFunnel(id: number) {
  return del<{ deleted: number }>(`/tracking/funnels/${id}`);
}

export interface AnalyzeTimeRange {
  days?: number;
  start?: string;
  end?: string;
}

export interface AnalyzeFunnelOptions {
  time_range?: AnalyzeTimeRange;
  group_by?: string;
}

export function analyzeFunnel(id: number, daysOrOptions: number | AnalyzeFunnelOptions = 7) {
  const body =
    typeof daysOrOptions === 'number' ? { time_range: { days: daysOrOptions } } : daysOrOptions;
  return post<FunnelAnalysisResult>(`/tracking/funnels/${id}/analyze`, body);
}

// ── 留存分析 ──────────────────────────────────────────────────────────────────

export interface TrackRetentionConfig {
  id: number;
  project_id: number;
  name: string;
  initial_event: string;
  return_event: string;
  initial_filters?: Record<string, unknown> | null;
  return_filters?: Record<string, unknown> | null;
  retention_days: number;
  created_at: string;
}

export interface RetentionTableRow {
  cohort_date: string;
  cohort_size: number;
  [key: string]: number | string;
}

export interface RetentionResult {
  retention_table: RetentionTableRow[];
  avg_retention: number[];
  retention_type?: 'day' | 'week';
}

export function listRetentions(project_id: number) {
  return get<{ list: TrackRetentionConfig[]; total: number }>('/tracking/retentions', {
    params: { project_id },
  });
}

export function createRetention(body: {
  project_id: number;
  name: string;
  initial_event: string;
  return_event: string;
  initial_filters?: Record<string, unknown>;
  return_filters?: Record<string, unknown>;
  retention_days?: number;
}) {
  return post<TrackRetentionConfig>('/tracking/retentions', body);
}

export interface AnalyzeRetentionOptions {
  time_range?: AnalyzeTimeRange;
  retention_type?: 'day' | 'week';
}

export function analyzeRetention(id: number, daysOrOptions: number | AnalyzeRetentionOptions = 14) {
  const body =
    typeof daysOrOptions === 'number'
      ? { time_range: { days: daysOrOptions }, retention_type: 'day' }
      : daysOrOptions;
  return post<RetentionResult>(`/tracking/retentions/${id}/analyze`, body);
}

// ── 用户画像 ──────────────────────────────────────────────────────────────────

export interface TrackUserProfile {
  id: number;
  project_id: number;
  distinct_id: string;
  anonymous_id?: string | null;
  user_id?: string | null;
  name?: string | null;
  email?: string | null;
  phone?: string | null;
  properties: Record<string, unknown>;
  first_visit_at?: string | null;
  last_visit_at?: string | null;
  total_events: number;
  total_sessions: number;
  created_at: string;
  updated_at: string;
}

export interface TrackUserEvent {
  id: number;
  project_id: number;
  distinct_id: string;
  anonymous_id?: string | null;
  user_id?: string | null;
  is_login_id: boolean;
  event: string;
  event_type: string;
  properties?: Record<string, unknown> | null;
  super_properties?: Record<string, unknown> | null;
  session_id?: string | null;
  page_url?: string | null;
  page_title?: string | null;
  referrer?: string | null;
  browser?: string | null;
  os?: string | null;
  device_type?: string | null;
  client_time?: string | null;
  created_at: string;
}

export interface TrackUserFilter {
  property: string;
  operator: 'eq' | 'neq' | 'contains' | 'exists' | 'not_exists';
  value?: string | number;
}

export interface TrackUserListParams {
  project_id: number;
  page?: number;
  page_size?: number;
  keyword?: string;
  filters?: string;
}

export function listTrackUsers(params: TrackUserListParams) {
  return get<{ list: TrackUserProfile[]; total: number }>('/tracking/users', { params });
}

export function getTrackUserDetail(projectId: number, distinctId: string) {
  return get<{ user: TrackUserProfile; recent_events: TrackUserEvent[] }>(
    `/tracking/users/${encodeURIComponent(distinctId)}`,
    { params: { project_id: projectId } }
  );
}

export interface TrackUserEventsParams {
  project_id: number;
  page?: number;
  page_size?: number;
  event_name?: string;
  start_time?: string;
  end_time?: string;
}

export function listTrackUserEvents(distinctId: string, params: TrackUserEventsParams) {
  return get<{ list: TrackUserEvent[]; total: number }>(
    `/tracking/users/${encodeURIComponent(distinctId)}/events`,
    { params }
  );
}
