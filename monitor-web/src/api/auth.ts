import { post } from '@/utils/request';
import type { User } from './user';

export interface AuthResult {
  access_token: string;
  refresh_token: string;
  token_type: 'Bearer';
  expires_in: number;
  user: User;
}

export interface RegisterBody {
  username: string;
  email: string;
  password: string;
  group_name?: string;
}

export interface LoginBody {
  account: string;
  password: string;
}

export function register(body: RegisterBody) {
  return post<AuthResult>('/auth/register', body);
}

export function login(body: LoginBody) {
  return post<AuthResult>('/auth/login', body);
}

export function refreshToken(refresh_token: string) {
  return post<AuthResult>('/auth/refresh', { refresh_token });
}
