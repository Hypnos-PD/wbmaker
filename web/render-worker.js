// Off-main-thread card renderer. Receives render/export requests, loads any
// missing font chunks itself, and posts the finished PNG bytes back — the UI
// thread stays responsive no matter how long a render takes.
//
// Runs as a module worker: `new Worker('render-worker.js', { type: 'module' })`.

import init, { render_card, render_diy_card } from './pkg/wbmaker.js';
import { ensureFonts } from './fonts.js';

// wbm 职业编号 -> byd-diy 背景文件名（与 wasm 内 DIY_CLASSES 一致）
const DIY_BG_CLASS = ['neutral', 'forestcraft', 'swordcraft', 'runecraft',
  'dragoncraft', 'abysscraft', 'havencraft', 'portalcraft'];

let wasmReady = null;
function boot() {
  if (!wasmReady) wasmReady = init();
  return wasmReady;
}

const bgCache = {}; // 文件名 -> Uint8Array
async function loadBackground(config) {
  const cls = DIY_BG_CLASS[config.class] || 'neutral';
  const gen = config.bg_type === 1 ? 1 : 2;
  const file = `backgrounds/${cls}-${gen}.jpg`;
  if (bgCache[file]) return bgCache[file];
  try {
    const resp = await fetch(file);
    if (!resp.ok) return new Uint8Array(0);
    const buf = new Uint8Array(await resp.arrayBuffer());
    bgCache[file] = buf;
    return buf;
  } catch (e) {
    return new Uint8Array(0);
  }
}

async function runRender(msg) {
  try {
    await boot();
    await ensureFonts(msg.lang, msg.cfg);
    const cfgJson = JSON.stringify(msg.cfg);
    const art = msg.art || new Uint8Array(0);
    const png = msg.cfg.style === 'diy'
      ? render_diy_card(cfgJson, art, await loadBackground(msg.cfg))
      : render_card(cfgJson, art);
    const out = new Uint8Array(png);
    self.postMessage({ type: 'result', id: msg.id, png: out }, [out.buffer]);
  } catch (e) {
    self.postMessage({ type: 'error', id: msg.id, message: String((e && e.message) || e) });
  }
}

// Latest-wins coalescing for previews (bursts of input collapse to the newest
// request), FIFO for exports (never silently dropped).
let busy = false;
let pendingRender = null;
const exportQueue = [];

self.onmessage = (e) => {
  const msg = e.data;
  if (!msg) return;
  if (msg.type === 'export') exportQueue.push(msg);
  else if (msg.type === 'render') pendingRender = msg;
  else return;
  if (!busy) pump();
};

async function pump() {
  busy = true;
  try {
    for (;;) {
      const msg = exportQueue.shift() || takePendingRender();
      if (!msg) break;
      await runRender(msg);
    }
  } finally {
    busy = false;
  }
}

function takePendingRender() {
  const m = pendingRender;
  pendingRender = null;
  return m;
}
