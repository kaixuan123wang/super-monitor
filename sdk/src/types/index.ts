/**
 * SDK 类型定义
 */

/** SDK 初始化配置 */
export interface MonitorConfig {
  /** 项目 appId（由监控端分配） */
  appId: string;
  /** 项目 appKey（SDK 上报鉴权） */
  appKey: string;
  /** 监控服务端地址，例如 https://monitor.example.com */
  server: string;
  /** 代码版本（用于 Source Map 关联），通常为 git commit hash */
  release?: string;
  /** 运行环境 */
  environment?: 'production' | 'staging' | 'development' | string;
  /** SDK 版本号（自动填充） */
  sdkVersion?: string;
  /** 是否开启调试日志 */
  debug?: boolean;

  /** 错误监控插件开关 */
  plugins?: {
    error?: boolean;
    console?: boolean;
    network?: boolean;
    performance?: boolean;
    breadcrumb?: boolean;
  };

  /** 埋点相关配置 */
  tracking?: TrackingConfig;

  /** 上报策略 */
  reporter?: ReporterConfig;
}

/** 埋点配置 */
export interface TrackingConfig {
  /** 是否启用埋点 */
  enableTracking?: boolean;
  /** 全埋点（自动采集）开关 */
  autoTrack?: {
    pageView?: boolean;
    click?: boolean;
    pageLeave?: boolean;
    exposure?: boolean;
  };
  /** 匿名 ID 前缀 */
  anonymousIdPrefix?: string;
  /** 埋点批量上报间隔（ms） */
  trackFlushInterval?: number;
  /** 埋点批量上报最大条数 */
  trackMaxBatchSize?: number;
}

/** 上报策略配置 */
export interface ReporterConfig {
  /** 最大队列长度 */
  maxQueueSize?: number;
  /** 批量上报间隔（ms） */
  flushInterval?: number;
  /** 最大重试次数 */
  retryMaxCount?: number;
  /** 重试间隔（ms） */
  retryInterval?: number;
}

/** 上报数据类型 */
export type CollectType =
  | 'error'
  | 'network'
  | 'performance'
  | 'breadcrumb'
  | 'track'
  | 'track_batch'
  | 'profile'
  | 'track_signup';

/** 上报数据包 */
export interface CollectPayload<T = unknown> {
  type: CollectType;
  data: T;
}

/** 事件属性值支持的类型 */
export type PropertyValue =
  | string
  | number
  | boolean
  | null
  | undefined
  | Array<string | number>
  | Record<string, unknown>;

export type Properties = Record<string, PropertyValue>;
