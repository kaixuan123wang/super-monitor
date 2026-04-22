/**
 * SDK 主入口：Monitor 单例
 *
 * Phase 1 目标：搭建骨架，提供初始化 / 上报 / 身份 / 超级属性 / 计时器等核心能力，
 * 能成功编译并生成 UMD / ESM / IIFE 三份产物。
 * 插件（error / network / performance / auto-track 等）在 Phase 2 具体实现。
 */

import type { CollectPayload, MonitorConfig, Properties } from '../types';
import { createLogger, now } from './utils';
import { Store } from './store';
import { Reporter, buildReporterFromConfig } from './reporter';
import { Identity } from './identity';
import { SuperProperties } from './super-props';
import { Timer } from './timer';

export const SDK_VERSION = '1.0.0';

class MonitorSDK {
  private config: MonitorConfig | null = null;
  private store: Store | null = null;
  private reporter: Reporter | null = null;
  private identity: Identity | null = null;
  private superProps: SuperProperties | null = null;
  private timer: Timer | null = null;
  private logger = createLogger(false);
  private initialized = false;

  /** 初始化 SDK */
  init(config: MonitorConfig): void {
    if (this.initialized) {
      this.logger.warn('Monitor.init called twice, ignored');
      return;
    }
    if (!config || !config.appId || !config.appKey || !config.server) {
      throw new Error('[Monitor] init requires appId, appKey and server');
    }

    this.config = {
      sdkVersion: SDK_VERSION,
      environment: 'production',
      debug: false,
      plugins: { error: true, console: false, network: true, performance: true, breadcrumb: true },
      tracking: {
        enableTracking: true,
        autoTrack: { pageView: true, click: true, pageLeave: true, exposure: false },
        anonymousIdPrefix: 'anon_',
        trackFlushInterval: 3000,
        trackMaxBatchSize: 20,
      },
      reporter: {
        maxQueueSize: 100,
        flushInterval: 5000,
        retryMaxCount: 3,
        retryInterval: 30000,
      },
      ...config,
    };

    this.logger = createLogger(!!this.config.debug);
    this.store = new Store({ maxQueueSize: this.config.reporter?.maxQueueSize ?? 100 });
    this.reporter = buildReporterFromConfig(this.config, this.store);
    this.reporter.start();

    this.identity = new Identity(this.config.tracking?.anonymousIdPrefix);
    this.superProps = new SuperProperties();
    this.timer = new Timer();

    this.initialized = true;
    this.logger.log('SDK initialized', { appId: this.config.appId, env: this.config.environment });
  }

  /** 手动上报一条数据 */
  report(payload: CollectPayload): void {
    if (!this.ensureReady()) return;
    this.reporter!.report(payload);
  }

  /** 立即 flush 队列 */
  flush(): Promise<void> | void {
    if (!this.ensureReady()) return;
    return this.reporter!.flush();
  }

  /** ========== 埋点 API（Phase 2 完整实现） ========== */

  track(eventName: string, properties?: Properties): void {
    if (!this.ensureReady()) return;
    const mergedProps: Properties = {
      ...this.superProps!.getAll(),
      ...(properties || {}),
    };
    this.report({
      type: 'track',
      data: {
        distinct_id: this.identity!.getDistinctId(),
        anonymous_id: this.identity!.getAnonymousId(),
        is_login_id: this.identity!.isLoginId(),
        event: eventName,
        properties: mergedProps,
        client_time: now(),
      },
    });
  }

  identify(userId: string): void {
    if (!this.ensureReady()) return;
    const { originalId, distinctId } = this.identity!.identify(userId);
    if (originalId !== distinctId) {
      this.report({
        type: 'track_signup',
        data: {
          distinct_id: distinctId,
          original_id: originalId,
          is_login_id: true,
        },
      });
    }
  }

  logout(): void {
    if (!this.ensureReady()) return;
    this.identity!.logout();
  }

  setUserProperties(properties: Properties): void {
    if (!this.ensureReady()) return;
    this.report({
      type: 'profile',
      data: {
        distinct_id: this.identity!.getDistinctId(),
        is_login_id: this.identity!.isLoginId(),
        operation: 'set',
        properties,
      },
    });
  }

  setUserPropertiesOnce(properties: Properties): void {
    if (!this.ensureReady()) return;
    this.report({
      type: 'profile',
      data: {
        distinct_id: this.identity!.getDistinctId(),
        is_login_id: this.identity!.isLoginId(),
        operation: 'set_once',
        properties,
      },
    });
  }

  appendUserProperties(properties: Properties): void {
    if (!this.ensureReady()) return;
    this.report({
      type: 'profile',
      data: {
        distinct_id: this.identity!.getDistinctId(),
        is_login_id: this.identity!.isLoginId(),
        operation: 'append',
        properties,
      },
    });
  }

  registerSuperProperties(properties: Properties): void {
    if (!this.ensureReady()) return;
    this.superProps!.register(properties);
  }

  unregisterSuperProperty(propertyName: string): void {
    if (!this.ensureReady()) return;
    this.superProps!.unregister(propertyName);
  }

  clearSuperProperties(): void {
    if (!this.ensureReady()) return;
    this.superProps!.clear();
  }

  trackTimerStart(eventName: string): void {
    if (!this.ensureReady()) return;
    this.timer!.start(eventName);
  }

  trackTimerEnd(eventName: string, properties?: Properties): void {
    if (!this.ensureReady()) return;
    const duration = this.timer!.end(eventName);
    this.track(eventName, { ...(properties || {}), $event_duration: duration ?? 0 });
  }

  /** 内部工具 */
  private ensureReady(): boolean {
    if (!this.initialized) {
      this.logger.warn('Monitor not initialized. Call Monitor.init(config) first.');
      return false;
    }
    return true;
  }
}

/** 导出单例 */
export const Monitor = new MonitorSDK();
export type { MonitorSDK };
