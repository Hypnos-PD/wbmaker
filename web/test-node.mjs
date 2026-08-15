// Node smoke test: instantiates the wasm, registers fonts, renders a card.
// Run:  node test-node.mjs
import { initSync, register_font, render_card, version, list_frames } from './pkg/wbmaker.js';
import fs from 'node:fs';

const wasm = fs.readFileSync('./pkg/wbmaker_bg.wasm');
initSync(wasm);

console.log('version:', version());
console.log('frames:', list_frames());

// Register fonts for 简体中文 (chs) + the shared number font.
register_font('title_chs', fs.readFileSync('./fonts/arweibeigbpro_bd.otf'));
register_font('number', fs.readFileSync('./fonts/FOT-TsukuAOldMin-Pr6-E.otf'));

const cfg = {
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

const png = render_card(JSON.stringify(cfg), new Uint8Array(0));
fs.writeFileSync('/tmp/wbmaker_test.png', png);
console.log('rendered', png.length, 'bytes -> /tmp/wbmaker_test.png');
