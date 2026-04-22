/**
 * 用户身份管理（简易 IDM）
 *
 * - 首次访问分配匿名 ID（anonymous_id），存 localStorage
 * - identify(userId) 后，distinct_id 切换为登录 ID
 * - logout() 清除登录 ID，distinct_id 回退为匿名 ID
 *
 * Phase 1 提供最小 API，Phase 2 再接入 track_signup 上报。
 */

import { uuid, safeStorage } from './utils';

const ANON_STORAGE_KEY = '__monitor_anon_id';
const LOGIN_STORAGE_KEY = '__monitor_login_id';

export class Identity {
  private anonymousId: string;
  private loginId: string | null;
  private readonly prefix: string;

  constructor(prefix = 'anon_') {
    this.prefix = prefix;
    const stored = safeStorage.get(ANON_STORAGE_KEY);
    if (stored) {
      this.anonymousId = stored;
    } else {
      this.anonymousId = prefix + uuid();
      safeStorage.set(ANON_STORAGE_KEY, this.anonymousId);
    }
    this.loginId = safeStorage.get(LOGIN_STORAGE_KEY);
  }

  /** 当前生效的用户标识（登录 ID 优先，否则匿名 ID） */
  getDistinctId(): string {
    return this.loginId || this.anonymousId;
  }

  getAnonymousId(): string {
    return this.anonymousId;
  }

  getLoginId(): string | null {
    return this.loginId;
  }

  isLoginId(): boolean {
    return !!this.loginId;
  }

  /** 手动重置匿名 ID（跨端统一 / 测试场景） */
  setAnonymousId(anonymousId: string): void {
    this.anonymousId = anonymousId;
    safeStorage.set(ANON_STORAGE_KEY, anonymousId);
  }

  /** 用户登录：关联登录 ID */
  identify(userId: string): { originalId: string; distinctId: string } {
    const originalId = this.getDistinctId();
    this.loginId = userId;
    safeStorage.set(LOGIN_STORAGE_KEY, userId);
    return { originalId, distinctId: userId };
  }

  /** 用户登出：重置为新匿名 ID */
  logout(): void {
    this.loginId = null;
    safeStorage.remove(LOGIN_STORAGE_KEY);
    this.anonymousId = this.prefix + uuid();
    safeStorage.set(ANON_STORAGE_KEY, this.anonymousId);
  }
}
