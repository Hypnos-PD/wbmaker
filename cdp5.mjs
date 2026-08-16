// E2E: worker rendering + instant button feedback.
// Run: node cdp5.mjs  (needs Chrome on :9226 and http server on :3636)
const res = await fetch('http://127.0.0.1:9226/json/new?http://127.0.0.1:3636/', { method: 'PUT' });
const tab = await res.json();
const ws = new WebSocket(tab.webSocketDebuggerUrl);
let id = 0; const pending = new Map(); const errors = []; const workerLoads = [];
ws.onmessage = (ev) => {
  const m = JSON.parse(ev.data);
  if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); }
  if (m.method === 'Runtime.exceptionThrown') errors.push(m.params.exceptionDetails.text);
  if (m.method === 'Log.entryAdded' && m.params.entry.level === 'error') errors.push(m.params.entry.text);
  if (m.method === 'Network.responseReceived') {
    const u = m.params.response.url;
    if (u.includes('render-worker') || u.includes('fonts.js')) workerLoads.push(u.split('/').pop());
  }
};
await new Promise(r => ws.onopen = r);
const send = (m, p = {}) => new Promise(r => { const i = ++id; pending.set(i, r); ws.send(JSON.stringify({ id: i, method: m, params: p })); });
await send('Runtime.enable'); await send('Log.enable'); await send('Network.enable');

const evalJs = async (expression) => {
  const r = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
  if (r.result.exceptionDetails) throw new Error(r.result.exceptionDetails.text + ' :: ' + expression);
  return r.result.result.value;
};

// wait until first preview is up
let firstOk = false;
for (let i = 0; i < 60; i++) {
  await new Promise(r => setTimeout(r, 500));
  const st = await evalJs(`(() => { const img = document.getElementById('preview');
    return { src: img.src.startsWith('blob:'), hint: document.getElementById('previewHint').style.display === 'none' }; })()`);
  if (st.src && st.hint) { firstOk = true; break; }
}
console.log('1) 首帧渲染(Worker):', firstOk);

// 2) style switching works
await evalJs(`document.getElementById('btnStyleDiy').click()`);
let diyOk = false;
for (let i = 0; i < 60; i++) {
  await new Promise(r => setTimeout(r, 500));
  const src = await evalJs(`document.getElementById('preview').src`);
  const st = await evalJs(`(() => { const img = document.getElementById('preview');
    return { src: img.src.startsWith('blob:'), hint: document.getElementById('previewHint').style.display === 'none' }; })()`);
  if (st.src && st.hint && src !== undefined) { diyOk = true; break; }
}
console.log('2) 效果图渲染(Worker):', diyOk);

// 3) size buttons: label must change in the same task; preview follows later
const sizeTest = await evalJs(`
  (async () => {
    const btn = document.querySelector('[data-size-up]');
    const label = document.getElementById(btn.dataset.sizeUp + 'Size');
    const before = label.textContent;
    const img = document.getElementById('preview');
    const srcBefore = img.src;
    const t0 = performance.now();
    btn.click();
    const labelLatency = performance.now() - t0;
    const labelChanged = label.textContent !== before;
    // wait for the preview to update to a new blob (max 15s)
    let renderLatency = -1;
    for (let i = 0; i < 150; i++) {
      await new Promise(r => setTimeout(r, 100));
      if (img.src !== srcBefore && img.src.startsWith('blob:')) { renderLatency = performance.now() - t0; break; }
    }
    return { before, after: label.textContent, labelChanged, labelLatency: Math.round(labelLatency), renderLatency: Math.round(renderLatency) };
  })()
`);
console.log('3) 字号按钮:', JSON.stringify(sizeTest));

// 4) rapid-fire clicks: 5 quick clicks, label should track every click
const rapid = await evalJs(`
  (async () => {
    const btn = document.querySelector('[data-size-up]');
    const label = document.getElementById(btn.dataset.sizeUp + 'Size');
    const clicks = 5;
    const seen = [];
    for (let i = 0; i < clicks; i++) {
      btn.click();
      seen.push(label.textContent);
      await new Promise(r => setTimeout(r, 30));
    }
    // wait until a render settles
    await new Promise(r => setTimeout(r, 5000));
    return { seen, final: label.textContent };
  })()
`);
console.log('4) 连点5次标签变化:', JSON.stringify(rapid));

// 5) typing in a textarea still updates preview (input debounce path)
const typing = await evalJs(`
  (async () => {
    const ta = document.querySelector('textarea[data-field]') || document.querySelector('textarea');
    if (!ta) return { ok: false, reason: 'no textarea' };
    const img = document.getElementById('preview');
    const srcBefore = img.src;
    ta.value += '测试';
    ta.dispatchEvent(new Event('input', { bubbles: true }));
    let ok = false;
    for (let i = 0; i < 150; i++) {
      await new Promise(r => setTimeout(r, 100));
      if (img.src !== srcBefore && img.src.startsWith('blob:')) { ok = true; break; }
    }
    return { ok };
  })()
`);
console.log('5) 文本输入触发预览:', JSON.stringify(typing));

// 6) UI responsiveness during render: measure how long a click event handler takes to run
const responsive = await evalJs(`
  (async () => {
    const btn = document.querySelector('[data-size-down]');
    const t0 = performance.now();
    btn.click();
    const dt = performance.now() - t0;
    return { clickHandlerMs: Math.round(dt) };
  })()
`);
console.log('6) 渲染期间点击事件处理耗时(ms):', JSON.stringify(responsive));

console.log('worker 加载的模块:', [...new Set(workerLoads)]);
console.log('页面异常:', errors.length ? errors : '无');
ws.close();
