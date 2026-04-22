/**
 * 事件时长计时器
 *
 * trackTimerStart(event) → 记录开始时间
 * trackTimerEnd(event)   → 计算 $event_duration（秒）并返回
 */

export class Timer {
  private timers = new Map<string, number>();

  start(eventName: string): void {
    this.timers.set(eventName, Date.now());
  }

  /** 返回 duration 秒数，若未启动则返回 undefined */
  end(eventName: string): number | undefined {
    const startAt = this.timers.get(eventName);
    if (startAt === undefined) return undefined;
    this.timers.delete(eventName);
    return Math.round(((Date.now() - startAt) / 1000) * 1000) / 1000; // 保留三位小数
  }

  clear(eventName?: string): void {
    if (eventName) {
      this.timers.delete(eventName);
    } else {
      this.timers.clear();
    }
  }
}
