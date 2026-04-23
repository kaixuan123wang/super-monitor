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

  /** 性能采样率（0-1，默认 0.1） */
  performanceSampleRate?: number;

  /** 脱敏配置 */
  sanitize?: SanitizeConfig;
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

/** 脱敏配置 */
export interface SanitizeConfig {
  /** 请求/响应 body 需替换的敏感字段 */
  sensitiveFields?: string[];
  /** URL 查询参数需移除的敏感参数 */
  sensitiveQueryKeys?: string[];
  /** body 最大截断长度（字节），默认 10240 */
  maxBodySize?: number;
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
  | 'track_signup'
  | 'batch';

/** 上报数据包 */
export interface CollectPayload<T = unknown> {
  type: CollectType;
  data: T;
  /** 上报优先级（P0：实时 / P1：批量），默认 P1 */
  priority?: 'P0' | 'P1';
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

/** JS 错误数据 */
export interface ErrorData {
  type: 'js' | 'promise' | 'resource' | 'vue' | 'react';
  message: string;
  stack?: string;
  source_url?: string;
  line?: number;
  column?: number;
  fingerprint?: string;
  extra?: Record<string, unknown>;
}

/** 接口请求/报错数据 */
export interface NetworkData {
  url: string;
  method: string;
  status: number;
  duration: number;
  request_headers?: Record<string, string>;
  request_body?: string;
  response_headers?: Record<string, string>;
  response_text?: string;
  error_type?: string;
}

/** 性能指标数据 */
export interface PerformanceData {
  url?: string;
  fp?: number;
  fcp?: number;
  lcp?: number;
  cls?: number;
  ttfb?: number;
  fid?: number;
  load_time?: number;
  dns_time?: number;
  tcp_time?: number;
  ssl_time?: number;
  dom_parse_time?: number;
  resource_count?: number;
  resource_size?: number;
}

/** 面包屑条目 */
export interface BreadcrumbItem {
  category: 'click' | 'navigation' | 'console' | 'xhr' | 'fetch' | 'input' | 'custom';
  message: string;
  level?: 'info' | 'warn' | 'error' | 'debug';
  timestamp: number;
  data?: Record<string, unknown>;
}

/** 上报时拼装的上下文（由 reporter 自动注入） */
export interface ReportContext {
  sdk_version?: string;
  release?: string;
  environment?: string;
  url?: string;
  title?: string;
  referrer?: string;
  user_agent?: string;
  browser?: string;
  browser_version?: string;
  os?: string;
  os_version?: string;
  device_type?: string;
  language?: string;
  timezone?: string;
  viewport?: string;
  screen_resolution?: string;
  breadcrumb?: BreadcrumbItem[];
  distinct_id?: string;
  anonymous_id?: string;
  is_login_id?: boolean;
}
