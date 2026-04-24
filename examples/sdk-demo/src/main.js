/**
 * SDK Demo 主入口
 * 引入 SDK 并初始化，绑定所有交互事件
 */

import Monitor from '../../../sdk/build/sdk.esm.js';

// 将 Monitor 挂载到 window，方便页面内联脚本使用
window.Monitor = Monitor;

// ========== SDK 初始化 ==========
Monitor.init({
  appId: 'demo_app_001',
  appKey: 'demo_key_secret',
  server: 'http://localhost:5173', // 通过 Vite proxy 转发到 mock server
  environment: 'development',
  release: 'demo-v1.0.0',
  debug: true,

  plugins: {
    error: true,
    console: true,
    network: true,
    performance: true,
    breadcrumb: true,
  },

  tracking: {
    enableTracking: true,
    autoTrack: {
      pageView: true,
      click: true,
      pageLeave: true,
      exposure: true,
    },
    anonymousIdPrefix: 'demo_',
    trackFlushInterval: 3000,
    trackMaxBatchSize: 20,
  },

  reporter: {
    maxQueueSize: 100,
    flushInterval: 5000,
    retryMaxCount: 3,
    retryInterval: 30000,
  },

  sanitize: {
    sensitiveFields: ['password', 'token', 'secret'],
    sensitiveQueryKeys: ['api_key', 'access_token'],
    maxBodySize: 10240,
  },
});

// ========== 工具函数 ==========
window.showToast = function (msg) {
  let el = document.getElementById('globalToast');
  if (!el) {
    el = document.createElement('div');
    el.id = 'globalToast';
    el.className = 'toast';
    document.body.appendChild(el);
  }
  el.textContent = msg;
  el.classList.add('show');
  setTimeout(() => el.classList.remove('show'), 2500);
};

function updateIdentityDisplay() {
  const distinctEl = document.getElementById('distinctId');
  const anonEl = document.getElementById('anonymousId');
  const loginEl = document.getElementById('isLogin');
  if (!distinctEl) return;

  // 通过 localStorage 读取（因为 SDK 内部使用 safeStorage）
  const anonId = localStorage.getItem('__monitor_anon_id') || '-';
  const loginId = localStorage.getItem('__monitor_login_id');
  distinctEl.textContent = loginId || anonId;
  anonEl.textContent = anonId;
  loginEl.textContent = loginId ? '是 (' + loginId + ')' : '否';
}

// ========== 用户身份 ==========
document.getElementById('btnLogin')?.addEventListener('click', () => {
  const userId = document.getElementById('userIdInput')?.value?.trim();
  if (!userId) {
    showToast('请输入用户 ID');
    return;
  }
  Monitor.identify(userId);
  updateIdentityDisplay();
  showToast(`已登录: ${userId}`);
});

document.getElementById('btnLogout')?.addEventListener('click', () => {
  Monitor.logout();
  updateIdentityDisplay();
  showToast('已登出');
});

document.getElementById('btnSetAnon')?.addEventListener('click', () => {
  const anonId = document.getElementById('anonIdInput')?.value?.trim();
  if (!anonId) {
    showToast('请输入匿名 ID');
    return;
  }
  Monitor.identify_anonymous(anonId);
  updateIdentityDisplay();
  showToast(`已设置匿名 ID: ${anonId}`);
});

// ========== 手动埋点 ==========
document.getElementById('btnTrackSimple')?.addEventListener('click', () => {
  Monitor.track('demo_simple_click');
  showToast('已上报 demo_simple_click');
});

document.getElementById('btnTrackProps')?.addEventListener('click', () => {
  Monitor.track('demo_with_props', {
    button_name: '带属性按钮',
    page: 'home',
    index: 42,
    is_vip: false,
    tags: ['tag1', 'tag2'],
    meta: { source: 'demo', version: 1 },
  });
  showToast('已上报 demo_with_props');
});

document.getElementById('btnTrackP0')?.addEventListener('click', () => {
  // 通过 report 直接发送 P0 优先级
  Monitor.report({
    type: 'track',
    data: {
      event: 'demo_realtime_event',
      properties: { urgency: 'high' },
      client_time: Date.now(),
    },
    priority: 'P0',
  });
  showToast('已实时上报 (P0)');
});

document.getElementById('btnTrackCustom')?.addEventListener('click', () => {
  const eventName = document.getElementById('customEventName')?.value?.trim() || 'custom_event';
  let props = {};
  try {
    const raw = document.getElementById('customEventProps')?.value;
    if (raw) props = JSON.parse(raw);
  } catch {
    showToast('属性 JSON 格式错误');
    return;
  }
  Monitor.track(eventName, props);
  showToast(`已上报 ${eventName}`);
});

// ========== 计时器 ==========
let timerEventName = 'demo_timer';

document.getElementById('btnTimerStart')?.addEventListener('click', () => {
  Monitor.trackTimerStart(timerEventName);
  const status = document.getElementById('timerStatus');
  if (status) {
    status.textContent = `计时器 "${timerEventName}" 运行中...`;
    status.classList.add('running');
  }
  showToast('计时器已启动');
});

document.getElementById('btnTimerEnd')?.addEventListener('click', () => {
  Monitor.trackTimerEnd(timerEventName, { action: 'manual_stop' });
  const status = document.getElementById('timerStatus');
  if (status) {
    status.textContent = '计时器已结束，事件已上报（含 $event_duration）';
    status.classList.remove('running');
  }
  showToast('计时结束并已上报');
});

document.getElementById('btnTimerMulti')?.addEventListener('click', async () => {
  const events = ['timer_page_load', 'timer_api_call', 'timer_render'];
  events.forEach((e, i) => {
    setTimeout(() => Monitor.trackTimerStart(e), i * 200);
  });
  await new Promise(r => setTimeout(r, 2000));
  events.forEach((e, i) => {
    setTimeout(() => Monitor.trackTimerEnd(e, { batch: 'multi_test' }), i * 100);
  });
  showToast('多计时器测试完成');
});

// ========== 用户属性 ==========
document.getElementById('btnSetProps')?.addEventListener('click', () => {
  Monitor.setUserProperties({
    nickname: 'DemoUser',
    age: 25,
    city: 'Shanghai',
    is_tester: true,
    interests: ['coding', 'music'],
  });
  showToast('已设置用户属性');
});

document.getElementById('btnSetOnce')?.addEventListener('click', () => {
  Monitor.setUserPropertiesOnce({
    first_visit_date: new Date().toISOString(),
    source_channel: 'demo_page',
  });
  showToast('已首次设置属性 (set_once)');
});

document.getElementById('btnAppendProps')?.addEventListener('click', () => {
  Monitor.appendUserProperties({
    viewed_pages: 'home',
    click_history: 'btn_append_test',
  });
  showToast('已追加用户属性');
});

document.getElementById('btnUnsetProp')?.addEventListener('click', () => {
  Monitor.unsetUserProperty('temp_flag');
  showToast('已删除属性 temp_flag');
});

// ========== 超级属性 ==========
document.getElementById('btnRegisterSuper')?.addEventListener('click', () => {
  const key = document.getElementById('superPropKey')?.value?.trim();
  const value = document.getElementById('superPropValue')?.value?.trim();
  if (!key) {
    showToast('请输入属性名');
    return;
  }
  Monitor.registerSuperProperties({ [key]: value });
  showToast(`已注册超级属性: ${key}=${value}`);
  displaySuperProps();
});

document.getElementById('btnShowSuper')?.addEventListener('click', () => {
  displaySuperProps();
});

document.getElementById('btnUnregisterSuper')?.addEventListener('click', () => {
  const key = document.getElementById('superPropKey')?.value?.trim();
  if (!key) {
    showToast('请输入要删除的属性名');
    return;
  }
  Monitor.unregisterSuperProperty(key);
  showToast(`已删除超级属性: ${key}`);
  displaySuperProps();
});

document.getElementById('btnClearSuper')?.addEventListener('click', () => {
  Monitor.clearSuperProperties();
  showToast('已清空所有超级属性');
  displaySuperProps();
});

function displaySuperProps() {
  const raw = localStorage.getItem('__monitor_super_props');
  const el = document.getElementById('superPropsDisplay');
  if (el) {
    el.textContent = raw ? JSON.stringify(JSON.parse(raw), null, 2) : '{}';
  }
}

// ========== 面包屑 ==========
document.getElementById('btnBreadcrumbInfo')?.addEventListener('click', () => {
  Monitor.addBreadcrumb({ category: 'custom', message: '用户点击了 Info 面包屑按钮', level: 'info', data: { page: 'home' } });
  showToast('已添加 Info 面包屑');
});

document.getElementById('btnBreadcrumbWarn')?.addEventListener('click', () => {
  Monitor.addBreadcrumb({ category: 'custom', message: '这是一个警告级别的面包屑', level: 'warn', data: { threshold: 80 } });
  showToast('已添加 Warn 面包屑');
});

document.getElementById('btnBreadcrumbError')?.addEventListener('click', () => {
  Monitor.addBreadcrumb({ category: 'custom', message: '用户触发了错误操作', level: 'error', data: { code: 500 } });
  showToast('已添加 Error 面包屑');
});

// ========== 错误触发 ==========
document.getElementById('btnJsError')?.addEventListener('click', () => {
  // 直接抛出错误
  const obj = null;
  obj.someMethod(); // TypeError
});

document.getElementById('btnPromiseError')?.addEventListener('click', () => {
  Promise.reject(new Error('手动触发的 Promise 拒绝错误'));
});

document.getElementById('btnResourceError')?.addEventListener('click', () => {
  const img = document.createElement('img');
  img.src = '/non-existent-image.png';
  img.style.display = 'none';
  document.body.appendChild(img);
  showToast('已加载不存在的资源');
});

document.getElementById('btnConsoleLog')?.addEventListener('click', () => {
  console.log('这是一条 console.log 日志，会被面包屑插件捕获');
  console.warn('这是一条 console.warn 日志');
  console.error('这是一条 console.error 日志');
  showToast('Console 日志已输出（查看面包屑）');
});

document.getElementById('btnManualReport')?.addEventListener('click', () => {
  Monitor.report({
    type: 'track',
    data: {
      event: 'manual_report_test',
      properties: { source: 'direct_report_api' },
      client_time: Date.now(),
    },
  });
  showToast('已直接调用 report API');
});

// ========== 网络请求 ==========
document.getElementById('btnFetchGet')?.addEventListener('click', async () => {
  try {
    const res = await fetch('https://httpbin.org/get');
    const data = await res.json();
    showToast('GET 请求成功');
    console.log('httpbin response', data);
  } catch (e) {
    showToast('GET 请求失败: ' + e.message);
  }
});

document.getElementById('btnFetchPost')?.addEventListener('click', async () => {
  try {
    const res = await fetch('https://httpbin.org/post', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ test: true, timestamp: Date.now() }),
    });
    const data = await res.json();
    showToast('POST 请求成功');
    console.log('httpbin response', data);
  } catch (e) {
    showToast('POST 请求失败: ' + e.message);
  }
});

document.getElementById('btnFetchError')?.addEventListener('click', async () => {
  try {
    await fetch('https://httpbin.org/status/500');
  } catch (e) {
    showToast('请求失败（预期内）');
  }
});

// ========== 页面初始化 ==========
updateIdentityDisplay();
displaySuperProps();

// 上报页面加载完成事件
Monitor.track('page_ready', {
  page_name: document.title,
  path: window.location.pathname,
});

console.log('[SDK Demo] Monitor initialized and ready. SDK Version:', Monitor.SDK_VERSION || '1.0.0');
