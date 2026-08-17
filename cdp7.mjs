// E2E part 3: standalone (packaged-app) behavior — marker injected, native
// save endpoint mocked. Verifies the export bridge + topbar hiding.
// Run: node cdp7.mjs
import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';

const saved = [];
// 模拟壳内服务器：静态伺服 web/ + POST /api/save_png
http.createServer((req, res) => {
  if (req.method === 'POST' && req.url.startsWith('/api/save_png')) {
    const chunks = [];
    req.on('data', c => chunks.push(c));
    req.on('end', () => {
      const name = decodeURIComponent(req.headers['x-filename'] || 'card.png');
      const out = `/tmp/wbmaker-standalone-${name}`;
      fs.writeFileSync(out, Buffer.concat(chunks));
      saved.push({ name, size: Buffer.concat(chunks).length, out });
      res.setHeader('content-type', 'application/json');
      res.end(JSON.stringify({ ok: true, path: out }));
    });
    return;
  }
  let p = decodeURIComponent(req.url.split('?')[0]);
  if (p === '/') p = '/index.html';
  const f = path.join('web', p);
  fs.readFile(f, (e, d) => {
    if (e) { res.statusCode = 404; res.end('not found'); return; }
    res.setHeader('content-type', mime(p));
    res.end(d);
  });
}).listen(3637, '127.0.0.1');

function mime(p) {
  if (p.endsWith('.html')) return 'text/html; charset=utf-8';
  if (p.endsWith('.js')) return 'text/javascript; charset=utf-8';
  if (p.endsWith('.css')) return 'text/css; charset=utf-8';
  if (p.endsWith('.wasm')) return 'application/wasm';
  if (p.endsWith('.otf')) return 'font/otf';
  if (p.endsWith('.json')) return 'application/json';
  if (p.endsWith('.png')) return 'image/png';
  if (p.endsWith('.jpg')) return 'image/jpeg';
  return 'application/octet-stream';
}

const res = await fetch('http://127.0.0.1:9226/json/new?http://127.0.0.1:3637/', { method: 'PUT' });
const tab = await res.json();
const ws = new WebSocket(tab.webSocketDebuggerUrl);
let id = 0; const pending = new Map(); const errors = [];
ws.onmessage = (ev) => {
  const m = JSON.parse(ev.data);
  if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); }
  if (m.method === 'Runtime.exceptionThrown') errors.push(m.params.exceptionDetails.text);
};
await new Promise(r => ws.onopen = r);
const send = (m, p = {}) => new Promise(r => { const i = ++id; pending.set(i, r); ws.send(JSON.stringify({ id: i, method: m, params: p })); });
await send('Runtime.enable');
await send('Page.enable');
// 注入 standalone 标记（壳服务器对 index.html 的等价物）
await send('Page.addScriptToEvaluateOnNewDocument', { source: 'window.__WBMAKER_STANDALONE__=true;' });
await send('Page.navigate', { url: 'http://127.0.0.1:3637/index.html' });

const evalJs = async (expression) => {
  const r = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
  if (r.result.exceptionDetails) throw new Error(r.result.exceptionDetails.text);
  return r.result.result.value;
};

for (let i = 0; i < 60; i++) {
  await new Promise(r => setTimeout(r, 500));
  const ok = await evalJs(`document.getElementById('preview') && document.getElementById('preview').src.startsWith('blob:')`);
  if (ok) break;
}

const topbar = await evalJs(`(() => {
  const logo = document.querySelector('.topbar-logo');
  const back = document.querySelector('.topbar .top-action[href="/"]');
  return { bodyClass: document.body.className, logoHidden: getComputedStyle(logo).display === 'none', backHidden: back ? getComputedStyle(back).display === 'none' : null };
})()`);
console.log('standalone 样式:', JSON.stringify(topbar));

await send('Page.setDownloadBehavior', { behavior: 'deny' });
await evalJs(`document.getElementById('btnExportPng').click()`);
let exportOk = false;
for (let i = 0; i < 120; i++) {
  await new Promise(r => setTimeout(r, 500));
  if (saved.length) { exportOk = true; break; }
}
console.log('导出走 /api/save_png:', exportOk, JSON.stringify(saved[0] || null));
if (saved[0]) {
  const magic = fs.readFileSync(saved[0].out).subarray(0, 4).toString('hex');
  console.log('PNG 魔数:', magic, magic === '89504e47' ? '(OK)' : '(异常)');
}
console.log('页面异常:', errors.length ? errors : '无');
ws.close();
process.exit(0);
