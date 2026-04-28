/**
 * 格式化日期时间为本地字符串
 * @param date 日期字符串或 Date 对象
 * @returns YYYY-MM-DD HH:mm:ss 格式字符串
 */
export function formatDateTime(date: string | Date | number | undefined | null): string {
  if (!date) return '';
  const d = typeof date === 'object' ? date : new Date(date);
  if (isNaN(d.getTime())) return String(date);
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
}

/**
 * 格式化日期为本地字符串
 * @param date 日期字符串或 Date 对象
 * @returns YYYY-MM-DD 格式字符串
 */
export function formatDate(date: string | Date | number | undefined | null): string {
  if (!date) return '';
  const d = typeof date === 'object' ? date : new Date(date);
  if (isNaN(d.getTime())) return String(date);
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}
