/**
 * 模拟上报服务器
 * 接收 SDK 上报的数据并存储，提供 API 供 dashboard 页面查询
 */

import http from 'http';
import url from 'url';

const PORT = 3456;
let records = [];

const server = http.createServer((req, res) => {
  // CORS
  res.setHeader('Access-Control-Allow-Origin', '*');
  res.setHeader('Access-Control-Allow-Methods', 'GET, POST, DELETE, OPTIONS');
  res.setHeader('Access-Control-Allow-Headers', 'Content-Type, X-App-Id, X-App-Key');

  if (req.method === 'OPTIONS') {
    res.writeHead(204);
    res.end();
    return;
  }

  const parsed = url.parse(req.url, true);

  // 接收上报数据
  if (parsed.pathname === '/api/v1/collect' && req.method === 'POST') {
    let body = '';
    req.on('data', chunk => body += chunk);
    req.on('end', () => {
      try {
        const payload = JSON.parse(body);
        const record = {
          id: records.length + 1,
          receivedAt: Date.now(),
          headers: {
            appId: req.headers['x-app-id'],
            appKey: req.headers['x-app-key'],
          },
          payload,
        };
        records.push(record);
        console.log(`[${new Date().toLocaleTimeString()}] 收到上报 [${payload.type || 'unknown'}] #${record.id}`);
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ code: 0, message: 'ok' }));
      } catch (e) {
        console.error('解析上报数据失败:', e.message);
        res.writeHead(400, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ code: 400, message: 'bad request' }));
      }
    });
    return;
  }

  // 查询上报记录
  if (parsed.pathname === '/api/v1/records' && req.method === 'GET') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ code: 0, data: { total: records.length, records } }));
    return;
  }

  // 清空记录
  if (parsed.pathname === '/api/v1/records' && req.method === 'DELETE') {
    records = [];
    console.log('上报记录已清空');
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ code: 0, message: 'cleared' }));
    return;
  }

  res.writeHead(404, { 'Content-Type': 'application/json' });
  res.end(JSON.stringify({ code: 404, message: 'not found' }));
});

server.listen(PORT, () => {
  console.log(`
╔══════════════════════════════════════════════╗
║       SDK 模拟上报服务器已启动               ║
╠══════════════════════════════════════════════╣
║  端口: ${PORT}                                  ║
║  接收: POST /api/v1/collect                   ║
║  查询: GET  /api/v1/records                   ║
║  清空: DELETE /api/v1/records                 ║
╚══════════════════════════════════════════════╝
`);
});
