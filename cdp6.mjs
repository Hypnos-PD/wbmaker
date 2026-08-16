// E2E part 2: export through the worker + worker-target error capture.
// Run: node cdp6.mjs
const res = await fetch('http://127.0.0.1:9226/json/new?http://127.0.0.1:3636/', { method: 'PUT' });
const tab = await res.json();
const ws = new WebSocket(tab.webSocketDebuggerUrl);
let id = 0; const pending = new Map(); const errors = []; const workerErrors = [];
const sessions = new Map();
ws.onmessage = async (ev) => {
  const m = JSON.parse(ev.data);
  if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); return; }
  if (m.method === 'Target.attachedToTarget') {
    const sid = m.params.sessionId;
    sessions.set(sid, true);
    ws.send(JSON.stringify({ id: ++id, sessionId: sid, method: 'Runtime.enable' }));
  }
  if (m.method === 'Runtime.exceptionThrown') {
    if (m.sessionId && sessions.has(m.sessionId)) workerErrors.push(m.params.exceptionDetails.text);
    else errors.push(m.params.exceptionDetails.text);
  }
};
await new Promise(r => ws.onopen = r);
const send = (m, p = {}, sid) => new Promise(r => { const i = ++id; pending.set(i, r); ws.send(JSON.stringify({ id: i, method: m, params: p, ...(sid ? { sessionId: sid } : {}) })); });
await send('Runtime.enable');
await send('Target.setAutoAttach', { autoAttach: true, waitForDebuggerOnStart: false, flatten: true });

const evalJs = async (expression) => {
  const r = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
  if (r.result.exceptionDetails) throw new Error(r.result.exceptionDetails.text + ' :: ' + expression);
  return r.result.result.value;
};

// wait first render
for (let i = 0; i < 60; i++) {
  await new Promise(r => setTimeout(r, 500));
  const st = await evalJs(`(() => { const img = document.getElementById('preview');
    return img.src.startsWith('blob:') && document.getElementById('previewHint').style.display === 'none'; })()`);
  if (st) break;
}

// export: set download dir and click
await send('Page.setDownloadBehavior', { behavior: 'allow', downloadPath: '/tmp/wbmaker-dl' });
await evalJs(`document.getElementById('btnExportPng').click()`);
let exported = null;
for (let i = 0; i < 120; i++) {
  await new Promise(r => setTimeout(r, 500));
  try {
    const fs = await import('node:fs');
    const files = fs.readdirSync('/tmp/wbmaker-dl');
    if (files.length) { exported = files; break; }
  } catch (e) {}
}
console.log('导出文件:', exported);

// while a render is in flight, fire a burst of inputs — must not error/backlog
const burst = await evalJs(`
  (async () => {
    const btn = document.querySelector('[data-size-up]');
    for (let i = 0; i < 8; i++) { btn.click(); await new Promise(r => setTimeout(r, 20)); }
    const img = document.getElementById('preview');
    await new Promise(r => setTimeout(r, 6000));
    return { src: img.src.startsWith('blob:'), hint: document.getElementById('previewHint').style.display === 'none' };
  })()
`);
console.log('连点后预览仍正常:', JSON.stringify(burst));
console.log('页面异常:', errors.length ? errors : '无');
console.log('Worker 异常:', workerErrors.length ? workerErrors : '无');
ws.close();
