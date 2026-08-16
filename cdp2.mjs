const res = await fetch('http://127.0.0.1:9224/json/new?http://127.0.0.1:3636/', { method: 'PUT' });
const tab = await res.json();
const ws = new WebSocket(tab.webSocketDebuggerUrl);
let id = 0; const pending = new Map(); const errors = [];
ws.onmessage = (ev) => {
  const m = JSON.parse(ev.data);
  if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); }
  if (m.method === 'Runtime.exceptionThrown') errors.push(m.params.exceptionDetails.text);
  if (m.method === 'Log.entryAdded' && m.params.entry.level === 'error') errors.push(m.params.entry.text);
};
await new Promise(r => ws.onopen = r);
const send = (m,p={}) => new Promise(r => { const i=++id; pending.set(i,r); ws.send(JSON.stringify({id:i,method:m,params:p})); });
await send('Runtime.enable'); await send('Log.enable');
await new Promise(r => setTimeout(r, 3000));
const r = await send('Runtime.evaluate', { expression: `
  (() => {
    const f = document.getElementById('cardForm').elements;
    document.getElementById('btnStyleDiy').click();
    return { name: f.name.value, trait: f.trait.value,
             d1: f.d1.value.slice(0,20), d2: f.d2.value.slice(0,12),
             showDetail2: f.showDetail2.checked };
  })()`, returnByValue: true });
console.log('表单默认值:', r.result.result.value);
await new Promise(r => setTimeout(r, 3000));
const r2 = await send('Runtime.evaluate', { expression: `
  (() => { const img = document.getElementById('preview');
            return { diySrc: img.src.startsWith('blob:'), hintHidden: document.getElementById('previewHint').style.display === 'none' }; })()`,
  returnByValue: true });
console.log('效果图渲染:', r2.result.result.value);
console.log('页面异常:', errors.length ? errors : '无');
ws.close();
