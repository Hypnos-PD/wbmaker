// Node smoke test: renders both WB and BYD-DIY cards.
// Run:  node test-node.mjs
import { initSync, register_font, render_card, render_diy_card, version, list_frames, list_diy_crests } from './pkg/wbmaker.js';
import fs from 'node:fs';

const wasm = fs.readFileSync('./pkg/wbmaker_bg.wasm');
initSync(wasm);

console.log('version:', version());
console.log('frames:', list_frames());
console.log('crests:', list_diy_crests());

register_font('title_chs', fs.readFileSync('./fonts/arweibeigbpro_bd.otf'));
register_font('title_eng', fs.readFileSync('./fonts/MOC-KaiminTsuki-B.otf'));
register_font('number', fs.readFileSync('./fonts/FOT-TsukuAOldMin-Pr6-E.digits.otf'));

const wbCfg = {
  name: '不屈的剑斗士',
  language: 'chs',
  class: 4,
  kind: 1,
  rarity: 4,
  frame: 'follower_legend',
  cost: '10+',
  atk: '7',
  life: '5',
  number_size: 106,
  scale: 1,
};

const png = render_card(JSON.stringify(wbCfg), new Uint8Array(0));
fs.writeFileSync('/tmp/wbmaker_wb.png', png);
console.log('wb rendered', png.length, 'bytes -> /tmp/wbmaker_wb.png');

const diyCfg = {
  name: '不屈的剑斗士',
  language: 'chs',
  class: 4,      // 龙族
  kind: 1,       // 随从
  rarity: 4,     // 虹
  cost: '10',
  atk: '7',
  life: '5',
  style: 'diy',
  bg_type: 2,
  trait_text: '士兵',
  class_title: '职业',
  type_title: '类型',
  class_text: '龙族',
  crest_border: 0,
  crest_scale: 1.0,
  d1_size: 32.4,
  d2_size: 32.4,
  ev_size: 32.4,
  super_size: 32.4,
  crest_size: 32.4,
  crest_icon1: 'builtin:0',
  crest_icon2: 'builtin:1',
  show_crest_icon2: true,
  show_detail2: true,
  show_evolve: true,
  show_super: true,
  show_crest: true,
  show_illustrator: true,
  show_diy: true,
  bg_alpha: 0.3,
  detail1: '【守护】\n【入场曲】抽取1张卡牌。\n这是正文内容，测试自动换行效果，[b]金色关键词[/b]和[i]斜体文本[/i]混排。',
  detail2: '【谢幕曲】给予自己的主战者1点伤害。',
  evolve: '进化时 获得+2/+2效果。',
  super_evolve: '超进化时 获得【疾驰】效果。',
  crest: '这个纹章给主战者带来祝福。',
  crest_name: '龙之纹章',
  illustrator: '某位画师',
  diy: 'DIY：某作者',
};

const art = fs.readFileSync('/home/aspharos/Project/sv-byd-diy/img/test/02fba8dd2718e77a51e519ab0895f3b29e6540e3.jpg');
const crest1 = fs.readFileSync('/home/aspharos/Project/sv-byd-diy/img/test/luna_crest.jpg');
const diyPng = render_diy_card(JSON.stringify(diyCfg), art, crest1, new Uint8Array(0));
fs.writeFileSync('/tmp/wbmaker_diy.png', diyPng);
console.log('diy rendered', diyPng.length, 'bytes -> /tmp/wbmaker_diy.png');
