import { get } from '@/utils/request';

export interface OverviewParams {
  project_id: number;
  days?: number;
}

export interface TrendPoint {
  date: string;
  count: number;
}

export interface DistItem {
  name: string;
  value: number;
}

export interface TopError {
  fingerprint: string;
  message: string;
  count: number;
}

export interface AvgPerformance {
  fp: number | null;
  fcp: number | null;
  lcp: number | null;
  ttfb: number | null;
}

export interface OverviewData {
  total_errors: number;
  total_network_errors: number;
  error_trend: TrendPoint[];
  browser_distribution: DistItem[];
  os_distribution: DistItem[];
  device_distribution: DistItem[];
  top_errors: TopError[];
  avg_performance: AvgPerformance;
}

export function getDashboardOverview(params: OverviewParams) {
  return get<OverviewData>('/dashboard/overview', { params });
}
