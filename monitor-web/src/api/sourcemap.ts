import { get, del } from '@/utils/request';
import axios from 'axios';

export interface SourceMap {
  id: number;
  project_id: number;
  release: string;
  filename: string;
  file_size: number | null;
  storage_path: string;
  content_hash: string | null;
  uploaded_at: string;
}

export interface ListParams {
  project_id: number;
  release?: string;
  page?: number;
  page_size?: number;
}

export function listSourceMaps(params: ListParams) {
  return get<{ list: SourceMap[]; total: number }>('/sourcemaps', { params });
}

export function getSourceMap(id: number) {
  return get<SourceMap>(`/sourcemaps/${id}`);
}

export function deleteSourceMap(id: number) {
  return del<{ deleted: number }>(`/sourcemaps/${id}`);
}

export function uploadSourceMap(
  project_id: number,
  release: string,
  file: File,
  onProgress?: (pct: number) => void,
) {
  const form = new FormData();
  form.append('project_id', String(project_id));
  form.append('release', release);
  form.append('file', file);

  return axios.post('/api/sourcemaps', form, {
    headers: { 'Content-Type': 'multipart/form-data' },
    onUploadProgress: (e) => {
      if (onProgress && e.total) {
        onProgress(Math.round((e.loaded * 100) / e.total));
      }
    },
  });
}
