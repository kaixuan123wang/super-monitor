/**
 * SSE 客户端，支持自动重连（指数退避）。
 * 用法：
 *   const client = new SSEClient('/api/dashboard/realtime?project_id=1')
 *   client.on('error', (data) => { ... })
 *   client.on('heartbeat', (data) => { ... })
 *   client.connect()
 *   // 卸载时：
 *   client.disconnect()
 *
 * 安全：不将 access_token 作为 URL query 参数传递（会泄漏到日志/历史）。
 * 改为先通过 POST 获取短生命周期的 SSE token，再用该 token 连接。
 */
export class SSEClient {
  private url: string;
  private es: EventSource | null = null;
  private handlers: Map<string, Array<(data: unknown) => void>> = new Map();
  private reconnectDelay = 1000;
  private readonly maxDelay = 30000;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private stopped = false;
  private boundListeners: Map<string, (e: MessageEvent) => void> = new Map();

  constructor(url: string) {
    this.url = url;
  }

  on(event: string, handler: (data: unknown) => void): this {
    if (!this.handlers.has(event)) this.handlers.set(event, []);
    this.handlers.get(event)!.push(handler);

    // If already connected, register the listener on the active EventSource
    if (this.es) {
      this.addListener(this.es, event);
    }
    return this;
  }

  off(event: string, handler?: (data: unknown) => void): this {
    if (!this.handlers.has(event)) return this;
    if (handler) {
      const arr = this.handlers.get(event)!;
      const idx = arr.indexOf(handler);
      if (idx !== -1) arr.splice(idx, 1);
    } else {
      this.handlers.delete(event);
    }
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
    this.boundListeners.clear();
  }

  private addListener(es: EventSource, evt: string): void {
    if (this.boundListeners.has(evt)) return; // already bound
    const listener = (e: MessageEvent) => {
      try {
        const data = JSON.parse(e.data);
        this.handlers.get(evt)?.forEach((fn) => fn(data));
      } catch {
        this.handlers.get(evt)?.forEach((fn) => fn(e.data));
      }
    };
    this.boundListeners.set(evt, listener);
    es.addEventListener(evt, listener);
  }

  private async open(): Promise<void> {
    this.es?.close();
    this.boundListeners.clear();

    // 获取短生命周期 SSE token（避免将 access_token 泄漏到 URL）
    let sseToken = '';
    try {
      const token = localStorage.getItem('__monitor_access_token');
      if (token) {
        const resp = await fetch('/api/auth/sse-token', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            Authorization: `Bearer ${token}`,
          },
        });
        if (resp.ok) {
          const body = await resp.json();
          sseToken = body?.data?.token || '';
        }
      }
    } catch {
      this.scheduleReconnect();
      return;
    }

    if (!sseToken) {
      this.scheduleReconnect();
      return;
    }

    const separator = this.url.includes('?') ? '&' : '?';
    const fullUrl = `${this.url}${separator}token=${encodeURIComponent(sseToken)}`;

    this.es = new EventSource(fullUrl);

    this.es.onopen = () => {
      this.reconnectDelay = 1000; // 连接成功后重置退避
    };

    this.es.onerror = () => {
      this.es?.close();
      this.es = null;
      this.boundListeners.clear();
      this.scheduleReconnect();
    };

    // Register listeners for all handlers (including those added after construction)
    for (const [evt] of this.handlers) {
      this.addListener(this.es, evt);
    }
  }

  private scheduleReconnect(): void {
    if (this.stopped || this.reconnectTimer) return;
    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null;
      this.reconnectDelay = Math.min(this.reconnectDelay * 2, this.maxDelay);
      this.open();
    }, this.reconnectDelay);
  }
}
