/**
 * SDK 主入口：Monitor 单例
 *
 * Phase 2 在 Phase 1 骨架基础上补齐：
 * - 集成 error / network / performance / breadcrumb / console 插件
 * - Reporter 注入公共上下文（url / ua / breadcrumb / distinct_id 等）
 * - 补充 identify_anonymous / appendUserProperties / setUserPropertiesOnce 等 API
 */

import type {
  CollectPayload,
  MonitorConfig,
  Properties,
  ReportContext,
  BreadcrumbItem,
} from '../types';
import { createLogger, now, parseUA, sanitizeUrl } from './utils';
import { Store } from './store';
import { Reporter, buildReporterFromConfig } from './reporter';
import { Identity } from './identity';
import { SuperProperties } from './super-props';
import { Timer } from './timer';
import { BreadcrumbBuffer } from './breadcrumb-buffer';
import { installErrorPlugin } from '../plugins/error';
import { installNetworkPlugin } from '../plugins/network';
import { installPerformancePlugin } from '../plugins/performance';
import { installBreadcrumbPlugin } from '../plugins/breadcrumb';
import { installConsolePlugin } from '../plugins/console';
import { installAutoTrackPlugin } from '../plugins/auto-track';
import { installExposurePlugin } from '../plugins/exposure';

export const SDK_VERSION = '1.0.0';

class MonitorSDK {
  private config: MonitorConfig | null = null;
  private store: Store | null = null;
  private reporter: Reporter | null = null;
  private identity: Identity | null = null;
  private superProps: SuperProperties | null = null;
  private timer: Timer | null = null;
  private breadcrumb: BreadcrumbBuffer | null = null;
  private logger = createLogger(false);
  private initialized = false;
  private cleanups: Array<() => void> = [];

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
      performanceSampleRate: 0.1,
      plugins: {
        error: true,
        console: true,
        network: true,
        performance: true,
        breadcrumb: true,
      },
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
    this.breadcrumb = new BreadcrumbBuffer(30);

    this.identity = new Identity(this.config.tracking?.anonymousIdPrefix);
    this.superProps = new SuperProperties();
    this.timer = new Timer();

    this.reporter = buildReporterFromConfig(this.config, this.store, () => this.buildContext());
    this.reporter.start();

    this.installPlugins();

    this.initialized = true;
    this.logger.log('SDK initialized', {
      appId: this.config.appId,
      env: this.config.environment,
    });
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

  /** 销毁 SDK（主要用于测试 / 插件热卸载），销毁后可重新 init */
  destroy(): void {
    for (const fn of this.cleanups) {
      try {
        fn();
      } catch {
        /* ignore */
      }
    }
    this.cleanups = [];
    this.reporter?.stop();
    this.reporter = null;
    this.store = null;
    this.identity = null;
    this.superProps = null;
    this.timer = null;
    this.breadcrumb = null;
    this.config = null;
    this.initialized = false;
  }

  /** ========== 埋点 API ========== */

  track(eventName: string, properties?: Properties): void {
    this.trackWithPriority(eventName, properties);
  }

  private trackWithPriority(
    eventName: string,
    properties?: Properties,
    priority?: CollectPayload['priority']
  ): void {
    if (!this.ensureReady()) return;
    const superProperties = this.superProps!.getAll();
    this.report({
      type: 'track',
      data: {
        distinct_id: this.identity!.getDistinctId(),
        anonymous_id: this.identity!.getAnonymousId(),
        is_login_id: this.identity!.isLoginId(),
        event: eventName,
        properties: { ...(properties || {}) },
        super_properties: superProperties,
        client_time: now(),
      },
      priority,
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

  /** 手动设置匿名 ID（高级用法，如跨端统一匿名标识） */
  identify_anonymous(anonymousId: string): void {
    if (!this.ensureReady()) return;
    this.identity!.setAnonymousId(anonymousId);
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

  unsetUserProperty(propertyName: string): void {
    if (!this.ensureReady()) return;
    this.report({
      type: 'profile',
      data: {
        distinct_id: this.identity!.getDistinctId(),
        is_login_id: this.identity!.isLoginId(),
        operation: 'unset',
        properties: { [propertyName]: true },
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

  /** 手动新增一条面包屑 */
  addBreadcrumb(item: Omit<BreadcrumbItem, 'timestamp'>): void {
    if (!this.ensureReady()) return;
    this.breadcrumb!.push(item);
  }

  /** ========== 内部实现 ========== */

  private installPlugins(): void {
    const cfg = this.config!;
    const report = (p: CollectPayload): void => this.reporter!.report(p);

    if (cfg.plugins?.breadcrumb !== false) {
      this.cleanups.push(
        installBreadcrumbPlugin({ buffer: this.breadcrumb!, sanitize: cfg.sanitize })
      );
    }

    if (cfg.plugins?.console) {
      this.cleanups.push(installConsolePlugin({
        buffer: this.breadcrumb!,
        sanitize: cfg.sanitize,
      }));
    }

    if (cfg.plugins?.error !== false) {
      this.cleanups.push(
        installErrorPlugin({ report, debug: cfg.debug, sanitize: cfg.sanitize })
      );
    }

    if (cfg.plugins?.network !== false) {
      this.cleanups.push(
        installNetworkPlugin({
          report,
          breadcrumb: this.breadcrumb!,
          sanitize: cfg.sanitize,
        })
      );
    }

    if (cfg.plugins?.performance !== false) {
      this.cleanups.push(
        installPerformancePlugin({
          report,
          sampleRate: cfg.performanceSampleRate,
        })
      );
    }

    if (cfg.tracking?.enableTracking !== false && cfg.tracking?.autoTrack) {
      const autoTrackCfg = cfg.tracking.autoTrack;
      const hasAnyAutoTrack =
        autoTrackCfg.pageView !== false ||
        autoTrackCfg.click !== false ||
        autoTrackCfg.pageLeave !== false;

      if (hasAnyAutoTrack) {
        this.cleanups.push(
          installAutoTrackPlugin({
            track: (event, properties, priority) =>
              this.trackWithPriority(event, properties as import('../types').Properties, priority),
            config: cfg.tracking,
            sanitize: cfg.sanitize,
          })
        );
      }

      if (autoTrackCfg.exposure === true) {
        this.cleanups.push(
          installExposurePlugin({
            track: (event, properties) =>
              this.track(event, properties as import('../types').Properties),
            sanitize: cfg.sanitize,
          })
        );
      }
    }
  }

  /** 构造上报上下文（每次发送时调用一次） */
  private buildContext(): ReportContext {
    const cfg = this.config!;
    const ctx: ReportContext = {
      sdk_version: cfg.sdkVersion || SDK_VERSION,
      release: cfg.release,
      environment: cfg.environment,
    };
    if (typeof window !== 'undefined') {
      try {
        ctx.url = sanitizeUrl(window.location?.href, cfg.sanitize?.sensitiveQueryKeys);
        ctx.title = document.title;
        ctx.referrer = sanitizeUrl(document.referrer, cfg.sanitize?.sensitiveQueryKeys);
        ctx.user_agent = navigator.userAgent;
        ctx.language = navigator.language;
        ctx.timezone = Intl?.DateTimeFormat
          ? Intl.DateTimeFormat().resolvedOptions().timeZone
          : undefined;
        ctx.viewport = `${window.innerWidth}x${window.innerHeight}`;
        ctx.screen_resolution = `${screen.width}x${screen.height}`;
        const parsed = parseUA(navigator.userAgent);
        ctx.browser = parsed.browser;
        ctx.browser_version = parsed.browser_version;
        ctx.os = parsed.os;
        ctx.os_version = parsed.os_version;
        ctx.device_type = parsed.device_type;
      } catch {
        /* ignore */
      }
    }
    if (this.identity) {
      ctx.distinct_id = this.identity.getDistinctId();
      ctx.anonymous_id = this.identity.getAnonymousId();
      ctx.is_login_id = this.identity.isLoginId();
    }
    if (this.breadcrumb) {
      ctx.breadcrumb = this.breadcrumb.getAll();
    }
    return ctx;
  }

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
