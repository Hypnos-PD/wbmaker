// Bench: where does the DIY render time go?
// Run:  node bench-node.mjs
import { initSync, register_font, render_card, render_diy_card } from './pkg/wbmaker.js';
import fs from 'node:fs';

const wasm = fs.readFileSync('./pkg/wbmaker_bg.wasm');
initSync(wasm);

register_font('title_chs', fs.readFileSync('./fonts/arweibeigbpro_bd.otf'));
register_font('title_eng', fs.readFileSync('./fonts/MOC-KaiminTsuki-B.otf'));
register_font('number', fs.readFileSync('./fonts/FOT-TsukuAOldMin-Pr6-E.digits.otf'));

const diyCfg = {
  name: '不屈的剑斗士', language: 'chs', class: 4, kind: 1, rarity: 4,
  cost: '10', atk: '7', life: '5', style: 'diy', bg_type: 2,
  trait_text: '士兵', class_title: '职业', type_title: '类型', class_text: '龙族',
  crest_border: 0, crest_scale: 1.0,
  d1_size: 32.4, d2_size: 32.4, ev_size: 32.4, super_size: 32.4, crest_size: 32.4,
  crest_icon1: 'builtin:0', crest_icon2: 'builtin:1',
  show_crest_icon2: true, show_detail2: true, show_evolve: true, show_super: true,
  show_illustrator: true, show_diy: true,
  crests: [{ text: '龙之纹章', description: '这个纹章给主战者带来祝福。', border: 0, scale: 1.0,
    icon1: 'builtin:0', icon2: 'builtin:1', show_icon2: true, size: 32.4 }],
  bg_alpha: 0.3,
  detail1: '【守护】\n【入场曲】抽取1张卡牌。\n这是正文内容，测试自动换行效果，[b]金色关键词[/b]和[i]斜体文本[/i]混排。',
  detail2: '【谢幕曲】给予自己的主战者1点伤害。',
  evolve: '进化时 获得+2/+2效果。', super_evolve: '超进化时 获得【疾驰】效果。',
  crest: '这个纹章给主战者带来祝福。', illustrator: '某位画师', diy: 'DIY：某作者',
};

const art = fs.readFileSync('/home/aspharos/Project/sv-byd-diy/img/test/02fba8dd2718e77a51e519ab0895f3b29e6540e3.jpg');
const bg = fs.readFileSync('./backgrounds/dragoncraft-2.jpg');
console.log('art bytes:', art.length, 'bg bytes:', bg.length);

function time(label, fn, n = 5) {
  fn(); // warm up
  const t0 = performance.now();
  let out;
  for (let i = 0; i < n; i++) out = fn();
  const ms = (performance.now() - t0) / n;
  console.log(`${label}: ${ms.toFixed(1)} ms (${out?.length ?? '-'} bytes)`);
  return out;
}

time('render_card(wb 782x1024)     ', () => render_card(JSON.stringify({ ...diyCfg, scale: 0.75, frame: 'follower_legend', number_size: 106 }), art));
time('render_diy_card(1920x1080)   ', () => render_diy_card(JSON.stringify(diyCfg), art, bg));
time('render_diy_card(no art)      ', () => render_diy_card(JSON.stringify(diyCfg), new Uint8Array(0), bg));
time('render_diy_card(no art,no bg)', () => render_diy_card(JSON.stringify(diyCfg), new Uint8Array(0), new Uint8Array(0)));
