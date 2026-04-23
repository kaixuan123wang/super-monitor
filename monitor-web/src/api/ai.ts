import { get, post } from '@/utils/request';

export interface AiAnalysis {
  id: number;
  error_id: number;
  fingerprint: string | null;
  project_id: number;
  model_used: string | null;
  status: 'pending' | 'success' | 'failed';
  ai_suggestion: string | null;
  severity_score: number | null;
  confidence: number | null;
  probable_file: string | null;
  probable_line: number | null;
  tags: string[] | null;
  analyzed_stack: string | null;
  is_cached: boolean;
  cost_ms: number | null;
  created_at: string;
  updated_at: string;
}

export interface AnalysisListParams {
  project_id: number;
  page?: number;
  page_size?: number;
  model_used?: string;
  has_suggestion?: boolean;
}

export function triggerAnalysis(error_id: number) {
  return post<{ task_id: number; status: string }>(`/ai/analyze/${error_id}`);
}

export function getAnalysisResult(error_id: number) {
  return get<AiAnalysis & { status?: string }>(`/ai/analysis/${error_id}`);
}

export function listAnalyses(params: AnalysisListParams) {
  return get<{ list: AiAnalysis[]; total: number }>('/ai/analyses', { params });
}

export function triggerBatchAnalysis(fingerprint: string, project_id: number) {
  return post<{ queued: number }>('/ai/analyze-batch', { fingerprint, project_id });
}
