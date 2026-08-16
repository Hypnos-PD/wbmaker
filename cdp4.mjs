const res = await fetch('http://127.0.0.1:9226/json/new?http://127.0.0.1:3636/', { method: 'PUT' });
const tab = await res.json();
const ws = new WebSocket(tab.webSocketDebuggerUrl);
let id=0; const pending=new Map(); const errors=[]; const loads=[];
ws.onmessage=(ev)=>{const m=JSON.parse(ev.data);
  if(m.id&&pending.has(m.id)){pending.get(m.id)(m);pending.delete(m.id);}
  if(m.method==='Runtime.exceptionThrown') errors.push(m.params.exceptionDetails.text);
  if(m.method==='Log.entryAdded'&&m.params.entry.level==='error') errors.push(m.params.entry.text);
  if(m.method==='Network.responseReceived'&&m.params.response.url.includes('.otf')) loads.push(m.params.response.url.split('/').pop());};
await new Promise(r=>ws.onopen=r);
const send=(m,p={})=>new Promise(r=>{const i=++id;pending.set(i,r);ws.send(JSON.stringify({id:i,method:m,params:p}));});
await send('Runtime.enable'); await send('Log.enable'); await send('Network.enable');
await new Promise(r=>setTimeout(r,9000)); // 等大字体下载
const r = await send('Runtime.evaluate',{expression:`
  (() => { const f = document.getElementById('cardForm').elements;
    document.getElementById('btnStyleDiy').click();
    return { diy: f.diy.value.slice(0,12), showDiy: f.showDiy.checked,
             illusChecked: f.showIllustrator.checked }; })()`,returnByValue:true});
console.log('默认值:', r.result.result.value);
await new Promise(r=>setTimeout(r,4000));
const r2 = await send('Runtime.evaluate',{expression:`
  (() => { const img=document.getElementById('preview');
    return { src: img.src.startsWith('blob:'), hint: document.getElementById('previewHint').style.display==='none' }; })()`,returnByValue:true});
console.log('效果图渲染:', r2.result.result.value);
console.log('加载的 otf:', [...new Set(loads)]);
console.log('页面异常:', errors.length?errors:'无');
ws.close();
