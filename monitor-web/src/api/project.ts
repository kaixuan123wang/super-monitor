import { get, post, put, del } from '@/utils/request';

export interface Project {
  id: number;
  name: string;
  app_id: string;
  app_key: string;
  group_id: number;
  owner_id: number;
  description?: string | null;
  alert_threshold: number;
  alert_webhook?: string | null;
  data_retention_days: number;
  environment: string;
  created_at: string;
  updated_at: string;
}

export interface ProjectListParams {
  page?: number;
  page_size?: number;
  group_id?: number;
  keyword?: string;
}

export interface CreateProjectBody {
  name: string;
  group_id?: number;
  description?: string;
  alert_threshold?: number;
  data_retention_days?: number;
  environment?: string;
}

export interface UpdateProjectBody {
  name?: string;
  description?: string;
  alert_threshold?: number;
  alert_webhook?: string;
  data_retention_days?: number;
  environment?: string;
}

export function listProjects(params: ProjectListParams = {}) {
  return get<{ list: Project[]; total: number }>('/projects', { params });
}

export function getProject(id: number) {
  return get<Project>(`/projects/${id}`);
}

export function createProject(body: CreateProjectBody) {
  return post<Project>('/projects', body);
}

export function updateProject(id: number, body: UpdateProjectBody) {
  return put<Project>(`/projects/${id}`, body);
}

export function deleteProject(id: number) {
  return del<{ deleted: number }>(`/projects/${id}`);
}
