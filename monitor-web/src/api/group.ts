import { del, get, post, put } from '@/utils/request';

export interface Group {
  id: number;
  name: string;
  description?: string | null;
  owner_id: number;
  created_at: string;
  updated_at: string;
}

export interface GroupListParams {
  page?: number;
  page_size?: number;
  keyword?: string;
  owner_id?: number;
}

export interface CreateGroupBody {
  name: string;
  description?: string;
  owner_id?: number;
}

export interface UpdateGroupBody {
  name?: string;
  description?: string;
  owner_id?: number;
}

export function listGroups(params: GroupListParams = {}) {
  return get<{ list: Group[]; total: number }>('/groups', { params });
}

export function getGroup(id: number) {
  return get<Group>(`/groups/${id}`);
}

export function createGroup(body: CreateGroupBody) {
  return post<Group>('/groups', body);
}

export function updateGroup(id: number, body: UpdateGroupBody) {
  return put<Group>(`/groups/${id}`, body);
}

export function deleteGroup(id: number) {
  return del<{ deleted: number }>(`/groups/${id}`);
}
