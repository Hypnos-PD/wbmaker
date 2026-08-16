// Shared font loading (chunk manifest + on-demand chunks), used by the render
// worker. Each realm that imports this module gets its own caches and its own
// wasm instance, so fonts are fetched/registered per realm (HTTP cache makes
// the worker's copies cheap).

import { register_font } from './pkg/wbmaker.js';

// Per-language title fonts (extracted from the game's data.unity3d).
export const FONT_MAP = {
  chs: 'arweibeigbpro_bd.otf',
  cht: 'DFT_W7-930.ttf',
  jpn: 'MOC-KaiminTsuki-B.otf',
  kor: 'NanumGothic-ExtraBold.ttf',
  eng: 'MOC-KaiminTsuki-B.otf',
};
export const NUMBER_FONT = 'FOT-TsukuAOldMin-Pr6-E.digits.otf'; // 筑紫明朝（数字字体）
// 署名行（画师/脚注）字体：Noto Sans CJK 各语言版本（按需分块）
export const SIGNATURE_FONT = {
  chs: 'NotoSansSC-Regular.otf',
  cht: 'NotoSansTC-Regular.otf',
  jpn: 'NotoSansJP-Regular.otf',
  kor: 'NotoSansKR-Regular.otf',
  eng: 'NotoSansSC-Regular.otf',
};

const loadedFonts = {}; // filename -> Uint8Array (cached bytes)
const registeredFonts = new Set(); // registry keys already registered

export async function loadFontFile(filename) {
  if (loadedFonts[filename]) return loadedFonts[filename];
  // Fetching the same file concurrently would duplicate the download; let a
  // single in-flight promise serve all waiters.
  if (!loadFontFile.inflight) loadFontFile.inflight = new Map();
  if (loadFontFile.inflight.has(filename)) return loadFontFile.inflight.get(filename);
  const p = (async () => {
    // 分块清单里的路径已含 fonts/ 前缀
    const url = filename.startsWith('fonts/') ? filename : 'fonts/' + filename;
    const resp = await fetch(url);
    if (!resp.ok) throw new Error('Font load failed: ' + filename);
    const buf = new Uint8Array(await resp.arrayBuffer());
    loadedFonts[filename] = buf;
    loadFontFile.inflight.delete(filename);
    return buf;
  })();
  loadFontFile.inflight.set(filename, p);
  return p;
}

// ---- 字体分块按需加载（unicode-range 分包，参照 shadowverse-wb.com 的做法） ----

let fontChunksManifest = null;

export async function loadFontChunksManifest() {
  if (fontChunksManifest) return fontChunksManifest;
  try {
    const resp = await fetch('font-chunks.json');
    fontChunksManifest = resp.ok ? await resp.json() : {};
  } catch (e) {
    fontChunksManifest = {};
  }
  return fontChunksManifest;
}

// 递归收集配置里的全部字符串（用于计算需要哪些字体块）
export function collectTextStrings(cfg) {
  const out = [];
  const walk = (v) => {
    if (typeof v === 'string') out.push(v);
    else if (Array.isArray(v)) v.forEach(walk);
    else if (v && typeof v === 'object') Object.values(v).forEach(walk);
  };
  walk(cfg);
  return out;
}

const registeredChunks = {}; // prefix -> Set(已注册的分块文件)

// 按需拉取+注册 prefix 的分块字体；缺清单或失败时回退整字体（prefix 无后缀键）
async function ensureChunkFonts(prefix, manifestKey, texts, fallbackFile) {
  const manifest = await loadFontChunksManifest();
  const entry = manifest[manifestKey];
  const done = registeredChunks[prefix] || (registeredChunks[prefix] = new Set());
  if (!entry || !entry.files || !entry.files.length) {
    if (!registeredFonts.has(prefix)) {
      const buf = await loadFontFile(fallbackFile);
      if (!register_font(prefix, buf)) throw new Error('Font register failed: ' + prefix);
      registeredFonts.add(prefix);
    }
    return;
  }
  // 计算文本用到的块
  const needed = new Set();
  for (const text of texts) {
    for (const ch of text) {
      const cp = ch.codePointAt(0);
      for (const f of entry.files) {
        for (const [lo, hi] of f.ranges) {
          if (cp >= lo && cp <= hi) { needed.add(f.file); break; }
        }
      }
    }
  }
  for (const file of needed) {
    if (done.has(file)) continue;
    try {
      const buf = await loadFontFile(file);
      const m = file.match(/(\d+)\.otf$/);
      const idx = m ? m[1] : String(done.size);
      if (!register_font(`${prefix}_${idx}`, buf)) throw new Error('register failed');
      done.add(file);
    } catch (e) {
      // 分块失败：回退整字体
      console.warn('chunk font failed, fallback to whole font:', file, e);
      const buf = await loadFontFile(fallbackFile);
      if (!registeredFonts.has(prefix)) {
        if (!register_font(prefix, buf)) throw new Error('Font register failed: ' + prefix);
        registeredFonts.add(prefix);
      }
      return;
    }
  }
}

// 确保某语言渲染所需字体都已注册（数字/标题/署名字体按需分块）。
export async function ensureFonts(lang, cfg) {
  const texts = collectTextStrings(cfg);
  // 署名行只需要画师/脚注文本的字符块；※ 是渲染器自动加在脚注前的前缀，
  // 不在配置文本里，需要显式补上，否则它所在的分块不会被拉取
  const sigTexts = ['※', cfg.illus_title || '', cfg.illustrator || '', cfg.diy || ''];
  // 数字字体（整文件，仅 124KB）
  const numberBuf = await loadFontFile(NUMBER_FONT);
  if (!registeredFonts.has('number')) {
    if (!register_font('number', numberBuf)) throw new Error('Font register failed: number');
    registeredFonts.add('number');
  }
  // 标题字体：按需分块
  await ensureChunkFonts(`title_${lang}`, lang, texts, FONT_MAP[lang] || FONT_MAP.chs);
  // 署名字体：画师与脚注（常规体）
  const sigFile = SIGNATURE_FONT[lang] || SIGNATURE_FONT.chs;
  await ensureChunkFonts(`illus_${lang}`, `${lang}-sig`, sigTexts, sigFile);
  await ensureChunkFonts(`footnote_${lang}`, `${lang}-sig`, sigTexts, sigFile);
}
