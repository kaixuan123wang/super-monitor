import { del, get, post, put } from '@/utils/request';

export interface User {
  id: number;
  username: string;
  email: string;
  role: string;
  group_id?: number | null;
  avatar?: string | null;
  last_login_at?: string | null;
  created_at: string;
  updated_at: string;
}

export interface UserListParams {
  page?: number;
  page_size?: number;
  keyword?: string;
  group_id?: number;
  role?: string;
}

export interface CreateUserBody {
  username: string;
  email: string;
  password: string;
  role?: string;
  group_id?: number;
  avatar?: string;
}

export interface UpdateUserBody {
  username?: string;
  email?: string;
  password?: string;
  role?: string;
  group_id?: number;
  avatar?: string;
}

export function listUsers(params: UserListParams = {}) {
  return get<{ list: User[]; total: number }>('/users', { params });
}

export function getUser(id: number) {
  return get<User>(`/users/${id}`);
}

export function createUser(body: CreateUserBody) {
  return post<User>('/users', body);
}

export function updateUser(id: number, body: UpdateUserBody) {
  return put<User>(`/users/${id}`, body);
}

export function deleteUser(id: number) {
  return del<{ deleted: number }>(`/users/${id}`);
}
