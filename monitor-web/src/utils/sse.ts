/**
 * SSE 客户端，支持自动重连（指数退避）。
 * 用法：
 *   const client = new SSEClient('/api/dashboard/realtime?project_id=1')
 *   client.on('error', (data) => { ... })
 *   client.on('heartbeat', (data) => { ... })
 *   client.connect()
 *   // 卸载时：
 *   client.disconnect()
 */
export class SSEClient {
  private url: string;
  private es: EventSource | null = null;
  private handlers: Map<string, Array<(data: unknown) => void>> = new Map();
  private reconnectDelay = 1000;
  private readonly maxDelay = 30000;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private stopped = false;

  constructor(url: string) {
    this.url = url;
  }

  on(event: string, handler: (data: unknown) => void): this {
    if (!this.handlers.has(event)) this.handlers.set(event, []);
    this.handlers.get(event)!.push(handler);
    return this;
  }

  connect(): void {
    this.stopped = false;
    this.open();
  }

  disconnect(): void {
    this.stopped = true;
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    this.es?.close();
    this.es = null;
  }

  private open(): void {
    this.es?.close();
    this.es = new EventSource(this.url);

    this.es.onopen = () => {
      this.reconnectDelay = 1000; // 连接成功后重置退避
    };

    // 监听所有已注册事件
    for (const [evt] of this.handlers) {
      this.es.addEventListener(evt, (e: MessageEvent) => {
        try {
          const data = JSON.parse(e.data);
          this.handlers.get(evt)?.forEach((fn) => fn(data));
        } catch {
          this.handlers.get(evt)?.forEach((fn) => fn(e.data));
        }
      });
    }

    this.es.onerror = () => {
      this.es?.close();
      this.es = null;
      if (!this.stopped) {
        this.reconnectTimer = setTimeout(() => {
          this.reconnectDelay = Math.min(this.reconnectDelay * 2, this.maxDelay);
          this.open();
        }, this.reconnectDelay);
      }
    };
  }
}
