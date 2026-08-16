import init, { render_card, register_font, version } from './pkg/wbmaker.js';

let artBytes = null; // PNG bytes of uploaded art (full resolution)
let cropState = null; // normalized crop rect {x,y,w,h} applied at render time

const KIND_KEYS = { 1: 'follower', 2: 'amulet', 3: 'spell' };
const RARITY_KEYS = { 1: 'bronze', 2: 'silver', 3: 'gold', 4: 'legend' };
// Which specials are valid per kind (only style_101 is offered).
const SPECIAL_BY_KIND = {
  follower: ['', 'style_101'],
  spell: ['', 'style_101'],
  amulet: ['', 'style_101'],
};

// Per-language title fonts (extracted from the game's data.unity3d).
const FONT_MAP = {
  chs: 'arweibeigbpro_bd.otf',
  cht: 'DFT_W7-930.ttf',
  jpn: 'MOC-KaiminTsuki-B.otf',
  kor: 'NanumGothic-ExtraBold.ttf',
  eng: 'MOC-KaiminTsuki-B.otf',
};
const NUMBER_FONT = 'FOT-TsukuAOldMin-Pr6-E.digits.otf'; // 筑紫明朝（数字字体）

// Default card preloaded on startup: 90074110 卓越创造物Ω (Masterwork Artifact Ω).
const DEFAULT_CARD = {
  name: {
    chs: '卓越创造物Ω',
    eng: 'Masterwork Artifact Ω',
    jpn: 'イクシードアーティファクトΩ',
    kor: '익시드 아티팩트 오메가',
    cht: '卓絕的創造物Ω',
  },
  class: 7,   // 超越者 (Portalcraft)
  kind: 1,    // 随从 (Follower)
  rarity: 4,  // 虹 (Legend)
  cost: '10',
  atk: '10',
  life: '10',
  art: 'art/900741100.png',
};

// ---- Shared language setting (same key/values as WBArts) ----
const LANG_NAMES = { chs: "简体中文", eng: "English", jpn: "日本語", kor: "한국어", cht: "繁體中文" };
const LANG_SHORT = { chs: "简", eng: "EN", jpn: "日", kor: "韩", cht: "繁" };
const LANG_ORDER = ["chs", "eng", "jpn", "kor", "cht"];
const LANG_HTML = { chs: "zh-CN", eng: "en", jpn: "ja", kor: "ko", cht: "zh-TW" };

function detectSystemLang() {
  const lang = (navigator.language || "en").toLowerCase();
  if (lang.startsWith("zh")) return lang.includes("tw") || lang.includes("hk") || lang.includes("hant") ? "cht" : "chs";
  if (lang.startsWith("ja")) return "jpn";
  if (lang.startsWith("ko")) return "kor";
  return "eng";
}
let currentLang = localStorage.getItem("lang") || detectSystemLang();
if (!LANG_NAMES[currentLang]) currentLang = 'chs';

// ---- UI translations (kept in sync with WBArts terminology) ----
const UI = {
  chs: {
    brand: "制卡器", backToWba: "返回 WBA",
    basicInfo: "基础信息", cardName: "卡名", cardNamePh: "卡牌名称",
    classLabel: "职业", specialFrame: "特殊框", kindLabel: "种类", rarityLabel: "稀有度",
    cost: "费用", attack: "攻击", defense: "体力",
    art: "立绘", artPh: "选择图片（PNG/JPG/WebP）",
    exportPng: "导出 PNG", reset: "重置",
    previewLoading: "预览加载中…",
    renderFailed: "渲染失败：", exportFailed: "导出失败：", imageReadFailed: "图片读取失败：",
    fontLoadFailed: "字体加载失败: ", fontRegFailed: "字体注册失败: ",
    normal: "普通", langTitle: "切换语言 / Language",
    cropTitle: "裁切立绘", cropHint: "滚轮缩放 · 拖拽平移",
    cropZoomIn: "放大", cropZoomOut: "缩小", cropReset: "复位", cropCancel: "取消", cropConfirm: "确认",
  },
  eng: {
    brand: "Card Maker", backToWba: "Back to WBA",
    basicInfo: "Basic Info", cardName: "Card Name", cardNamePh: "Card name",
    classLabel: "Class", specialFrame: "Special Frame", kindLabel: "Type", rarityLabel: "Rarity",
    cost: "Cost", attack: "Attack", defense: "Defense",
    art: "Art", artPh: "Choose image (PNG/JPG/WebP)",
    exportPng: "Export PNG", reset: "Reset",
    previewLoading: "Loading preview…",
    renderFailed: "Render failed: ", exportFailed: "Export failed: ", imageReadFailed: "Failed to read image: ",
    fontLoadFailed: "Failed to load font: ", fontRegFailed: "Failed to register font: ",
    normal: "Normal", langTitle: "Switch language / 语言",
    cropTitle: "Crop Art", cropHint: "Scroll to zoom · Drag to pan",
    cropZoomIn: "Zoom In", cropZoomOut: "Zoom Out", cropReset: "Reset", cropCancel: "Cancel", cropConfirm: "Apply",
  },
  jpn: {
    brand: "カードメーカー", backToWba: "WBAに戻る",
    basicInfo: "基本情報", cardName: "カード名", cardNamePh: "カード名",
    classLabel: "クラス", specialFrame: "特殊フレーム", kindLabel: "種類", rarityLabel: "レアリティ",
    cost: "コスト", attack: "攻撃力", defense: "体力",
    art: "イラスト", artPh: "画像を選択（PNG/JPG/WebP）",
    exportPng: "PNG 出力", reset: "リセット",
    previewLoading: "プレビュー読込中…",
    renderFailed: "描画失敗：", exportFailed: "出力失敗：", imageReadFailed: "画像読込失敗：",
    fontLoadFailed: "フォント読込失敗: ", fontRegFailed: "フォント登録失敗: ",
    normal: "通常", langTitle: "言語切替 / Language",
    cropTitle: "イラストをトリミング", cropHint: "ホイールで拡大縮小 · ドラッグで移動",
    cropZoomIn: "拡大", cropZoomOut: "縮小", cropReset: "リセット", cropCancel: "キャンセル", cropConfirm: "確定",
  },
  kor: {
    brand: "카드 메이커", backToWba: "WBA로 돌아가기",
    basicInfo: "기본 정보", cardName: "카드 이름", cardNamePh: "카드 이름",
    classLabel: "클래스", specialFrame: "특수 프레임", kindLabel: "종류", rarityLabel: "레어도",
    cost: "코스트", attack: "공격력", defense: "생명력",
    art: "일러스트", artPh: "이미지 선택（PNG/JPG/WebP）",
    exportPng: "PNG 내보내기", reset: "초기화",
    previewLoading: "미리보기 로딩 중…",
    renderFailed: "렌더링 실패：", exportFailed: "내보내기 실패：", imageReadFailed: "이미지 읽기 실패：",
    fontLoadFailed: "폰트 로드 실패: ", fontRegFailed: "폰트 등록 실패: ",
    normal: "일반", langTitle: "언어 전환 / Language",
    cropTitle: "일러스트 자르기", cropHint: "휠로 확대/축소 · 드래그로 이동",
    cropZoomIn: "확대", cropZoomOut: "축소", cropReset: "초기화", cropCancel: "취소", cropConfirm: "확인",
  },
  cht: {
    brand: "製卡器", backToWba: "返回 WBA",
    basicInfo: "基礎資訊", cardName: "卡名", cardNamePh: "卡牌名稱",
    classLabel: "職業", specialFrame: "特殊框", kindLabel: "種類", rarityLabel: "稀有度",
    cost: "費用", attack: "攻擊", defense: "體力",
    art: "立繪", artPh: "選擇圖片（PNG/JPG/WebP）",
    exportPng: "匯出 PNG", reset: "重設",
    previewLoading: "預覽載入中…",
    renderFailed: "渲染失敗：", exportFailed: "匯出失敗：", imageReadFailed: "圖片讀取失敗：",
    fontLoadFailed: "字體載入失敗: ", fontRegFailed: "字體註冊失敗: ",
    normal: "普通", langTitle: "切換語言 / Language",
    cropTitle: "裁切立繪", cropHint: "滾輪縮放 · 拖曳平移",
    cropZoomIn: "放大", cropZoomOut: "縮小", cropReset: "復位", cropCancel: "取消", cropConfirm: "確認",
  },
};

const CLASS_LABELS = {
  chs: ["中立", "精灵", "皇家护卫", "巫师", "龙族", "梦魇", "主教", "超越者"],
  eng: ["Neutral", "Forestcraft", "Swordcraft", "Runecraft", "Dragoncraft", "Abysscraft", "Havencraft", "Portalcraft"],
  jpn: ["ニュートラル", "エルフ", "ロイヤル", "ウィッチ", "ドラゴン", "ナイトメア", "ビショップ", "ネメシス"],
  kor: ["중립", "엘프", "로얄", "위치", "드래곤", "나이트메어", "비숍", "네메시스"],
  cht: ["中立", "精靈", "皇家護衛", "巫師", "龍族", "夢魘", "主教", "超越者"],
};
const KIND_LABELS = {
  chs: ["随从", "护符", "法术"],
  eng: ["Follower", "Amulet", "Spell"],
  jpn: ["フォロワー", "アミュレット", "スペル"],
  kor: ["추종자", "부적", "마법"],
  cht: ["隨從", "護符", "法術"],
};
const RARITY_LABELS = {
  chs: ["铜", "银", "金", "虹"],
  eng: ["Bronze", "Silver", "Gold", "Legendary"],
  jpn: ["ブロンズ", "シルバー", "ゴールド", "レジェンド"],
  kor: ["브론즈", "실버", "골드", "레전드"],
  cht: ["銅", "銀", "金", "虹"],
};

function t(key) {
  return UI[currentLang]?.[key] || UI.chs[key] || key;
}

const loadedFonts = {}; // filename -> Uint8Array (cached bytes)
const registeredFonts = new Set(); // registry keys already registered

async function loadFontFile(filename) {
  if (loadedFonts[filename]) return loadedFonts[filename];
  // Fetching the same file concurrently would duplicate the download; let a
  // single in-flight promise serve all waiters.
  if (!loadFontFile.inflight) loadFontFile.inflight = new Map();
  if (loadFontFile.inflight.has(filename)) return loadFontFile.inflight.get(filename);
  const p = (async () => {
    const resp = await fetch('fonts/' + filename);
    if (!resp.ok) throw new Error(t('fontLoadFailed') + filename);
    const buf = new Uint8Array(await resp.arrayBuffer());
    loadedFonts[filename] = buf;
    loadFontFile.inflight.delete(filename);
    return buf;
  })();
  loadFontFile.inflight.set(filename, p);
  return p;
}

// Start downloading a language's fonts immediately (no wasm needed yet) so the
// big title font downloads in parallel with the wasm module.
function startFontDownload(lang) {
  const titleFile = FONT_MAP[lang] || FONT_MAP.chs;
  return Promise.all([loadFontFile(titleFile), loadFontFile(NUMBER_FONT)]);
}

// Register already-downloaded font bytes (requires the wasm to be initialized).
function registerFonts(lang, bufs) {
  const keys = [`title_${lang}`, 'number'];
  for (let i = 0; i < keys.length; i++) {
    // Registering re-parses the whole font each time — skip when already done.
    if (registeredFonts.has(keys[i])) continue;
    if (!register_font(keys[i], bufs[i])) {
      throw new Error(t('fontRegFailed') + keys[i]);
    }
    registeredFonts.add(keys[i]);
  }
}

async function loadFonts(lang) {
  const bufs = await startFontDownload(lang);
  registerFonts(lang, bufs);
}

const $ = (sel) => document.querySelector(sel);
const form = () => document.getElementById('cardForm');

function field(name) { return form().elements[name]; }

function collectConfig() {
  const kind = field('kind').value;
  const kindName = KIND_KEYS[kind] || 'follower';
  const rarityName = RARITY_KEYS[field('rarity').value] || 'bronze';
  const special = field('special').value;
  const frame = `${kindName}_${special || rarityName}`;
  return {
    name: field('name').value,
    language: currentLang,
    class: parseInt(field('class').value, 10) || 0,
    kind: parseInt(kind, 10) || 1,
    rarity: parseInt(field('rarity').value, 10) || 1,
    frame,
    cost: field('cost').value,
    atk: field('atk').value,
    life: field('life').value,
  };
}

// Render-time config: base fields plus the current crop rect (never exported).
function buildRenderConfig() {
  const cfg = collectConfig();
  if (cropState) cfg.crop = cropState;
  return cfg;
}

function renderCard(config, art) {
  return render_card(JSON.stringify(config), art || new Uint8Array(0));
}

async function renderPreview() {
  try {
    await loadFonts(currentLang);
    const cfg = buildRenderConfig();
    const png = renderCard(cfg, artBytes);
    const blob = new Blob([png], { type: 'image/png' });
    const url = URL.createObjectURL(blob);
    const img = document.getElementById('preview');
    const old = img.src;
    img.onload = () => { if (old) URL.revokeObjectURL(old); };
    img.src = url;
    document.getElementById('previewHint').style.display = 'none';
  } catch (e) {
    document.getElementById('previewHint').style.display = '';
    document.getElementById('previewHint').textContent = t('renderFailed') + e.message;
  }
}

function downloadBlob(blob, filename) {
  const a = document.createElement('a');
  a.href = URL.createObjectURL(blob);
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  setTimeout(() => { URL.revokeObjectURL(a.href); a.remove(); }, 500);
}

async function exportPng() {
  try {
    await loadFonts(currentLang);
    const cfg = buildRenderConfig();
    const png = renderCard(cfg, artBytes);
    const name = (cfg.name || 'card').replace(/[\\/:*?"<>|]/g, '_');
    downloadBlob(new Blob([png], { type: 'image/png' }), `${name}.png`);
  } catch (e) {
    alert(t('exportFailed') + e.message);
  }
}

function updateKindUI() {
  const kind = field('kind').value;
  const kindName = KIND_KEYS[kind];
  // follower-only fields
  document.querySelectorAll('.follower-only').forEach((el) => {
    el.style.display = kindName === 'follower' ? '' : 'none';
  });
  // special options
  const special = field('special');
  const valid = SPECIAL_BY_KIND[kindName] || [''];
  const current = special.value;
  special.innerHTML = valid
    .map((s) => {
      const label = s === '' ? t('normal') : s;
      return `<option value="${s}">${label}</option>`;
    })
    .join('');
  if (valid.includes(current)) special.value = current;
}

// Preload the default card (90074110 卓越创造物Ω) into the form, including art.
async function loadDefaultCard() {
  cropState = null;
  field('name').value = DEFAULT_CARD.name[currentLang] || DEFAULT_CARD.name.chs;
  field('class').value = String(DEFAULT_CARD.class);
  field('kind').value = String(DEFAULT_CARD.kind);
  field('rarity').value = String(DEFAULT_CARD.rarity);
  field('special').value = '';
  field('cost').value = DEFAULT_CARD.cost;
  field('atk').value = DEFAULT_CARD.atk;
  field('life').value = DEFAULT_CARD.life;
  updateKindUI();

  // Load the default card art (falls back to no art if the file is missing).
  try {
    const resp = await fetch(DEFAULT_CARD.art);
    if (resp.ok) {
      artBytes = new Uint8Array(await resp.arrayBuffer());
      document.getElementById('artLabel').textContent = DEFAULT_CARD.art.split('/').pop();
    } else {
      artBytes = null;
      document.getElementById('artLabel').textContent = t('artPh');
    }
  } catch (e) {
    artBytes = null;
    document.getElementById('artLabel').textContent = t('artPh');
  }
}

// ---- i18n: translate static labels & rebuild translated selects ----

function rebuildSelects() {
  const cls = document.getElementById('class');
  if (cls) {
    const cur = cls.value;
    cls.innerHTML = CLASS_LABELS[currentLang].map((name, i) => `<option value="${i}">${name}</option>`).join('');
    cls.value = cur;
  }
  const kind = document.getElementById('kind');
  if (kind) {
    const cur = kind.value;
    kind.innerHTML = KIND_LABELS[currentLang].map((name, i) => `<option value="${i + 1}">${name}</option>`).join('');
    kind.value = cur;
  }
  const rarity = document.getElementById('rarity');
  if (rarity) {
    const cur = rarity.value;
    rarity.innerHTML = RARITY_LABELS[currentLang].map((name, i) => `<option value="${i + 1}">${name}</option>`).join('');
    rarity.value = cur;
  }
  updateKindUI();
}

function applyI18n() {
  document.querySelectorAll('[data-i18n]').forEach((el) => {
    el.textContent = t(el.getAttribute('data-i18n'));
  });
  document.querySelectorAll('[data-i18n-ph]').forEach((el) => {
    el.setAttribute('placeholder', t(el.getAttribute('data-i18n-ph')));
  });
  rebuildSelects();
}

// ---- Topbar language dropdown (shares localStorage "lang" with WBArts) ----

function buildLangDropdown() {
  const container = document.getElementById('langDropdown');
  if (!container) return;
  container.innerHTML = '';
  const btn = document.createElement('button');
  btn.className = 'dd-btn';
  btn.type = 'button';
  btn.title = t('langTitle');
  btn.innerHTML = '🌐 <span id="langShortLabel">' + (LANG_SHORT[currentLang] || currentLang) + '</span>';
  container.appendChild(btn);

  const menu = document.createElement('div');
  menu.className = 'dd-menu';
  LANG_ORDER.forEach((l) => {
    const item = document.createElement('button');
    item.className = 'dd-item' + (l === currentLang ? ' active' : '');
    item.type = 'button';
    item.textContent = LANG_NAMES[l];
    item.addEventListener('click', () => switchLang(l));
    menu.appendChild(item);
  });
  container.appendChild(menu);
}

function updateLangUI() {
  const label = document.getElementById('langShortLabel');
  if (label) label.textContent = LANG_SHORT[currentLang] || currentLang;
  const menu = document.querySelector('#langDropdown .dd-menu');
  if (menu) {
    menu.querySelectorAll('.dd-item').forEach((b) => {
      b.classList.toggle('active', b.textContent === LANG_NAMES[currentLang]);
    });
  }
  const btn = document.querySelector('#langDropdown .dd-btn');
  if (btn) btn.title = t('langTitle');
  document.documentElement.lang = LANG_HTML[currentLang] || 'zh-CN';
  applyI18n();
}

function switchLang(lang) {
  if (!LANG_NAMES[lang]) return;
  currentLang = lang;
  localStorage.setItem('lang', lang);
  updateLangUI();
  renderPreview();
}

async function handleArtFile(file) {
  if (!file) {
    artBytes = null;
    cropState = null;
    document.getElementById('artLabel').textContent = t('artPh');
    renderPreview();
    return;
  }
  try {
    const bmp = await createImageBitmap(file);
    const canvas = document.createElement('canvas');
    canvas.width = bmp.width;
    canvas.height = bmp.height;
    const ctx = canvas.getContext('2d');
    ctx.drawImage(bmp, 0, 0);
    bmp.close();
    const blob = await new Promise((r) => canvas.toBlob(r, 'image/png'));
    const buf = new Uint8Array(await blob.arrayBuffer());
    const crop = await openCropPanel(buf);
    if (!crop) return; // cancelled — keep the previous art
    artBytes = buf;
    cropState = crop;
    document.getElementById('artLabel').textContent = file.name;
    renderPreview();
  } catch (e) {
    alert(t('imageReadFailed') + e.message);
  }
}

// ---- Crop panel (one-shot: zoom + pan over a fixed 590:711 viewport) ----

const cropView = {
  iw: 0, ih: 0,   // image natural size (px)
  vw: 0, vh: 0,   // viewport size (px)
  k0: 0, maxK: 0, // cover-fit scale and zoom ceiling
  k: 0, ox: 0, oy: 0, // current scale and offset
  url: null,      // object URL of the art being cropped
  resolve: null,
};

const cropImageEl = () => document.getElementById('cropImage');
const cropViewportEl = () => document.getElementById('cropViewport');

function applyCropTransform() {
  cropImageEl().style.transform =
    `translate(${cropView.ox}px, ${cropView.oy}px) scale(${cropView.k})`;
}

// Keep the image covering the viewport (no letterbox) while panning.
function clampCropOffset() {
  const { vw, vh, iw, ih, k } = cropView;
  cropView.ox = Math.min(0, Math.max(vw - iw * k, cropView.ox));
  cropView.oy = Math.min(0, Math.max(vh - ih * k, cropView.oy));
}

function resetCrop() {
  const { vw, vh, iw, ih, k0 } = cropView;
  cropView.k = k0;
  cropView.ox = (vw - iw * k0) / 2;
  cropView.oy = (vh - ih * k0) / 2;
  applyCropTransform();
}

// Zoom by `factor`, keeping the image point under (mx, my) fixed.
function cropZoomAt(mx, my, factor) {
  const { k0, maxK } = cropView;
  const next = Math.min(maxK, Math.max(k0, cropView.k * factor));
  if (next === cropView.k) return;
  const ix = (mx - cropView.ox) / cropView.k;
  const iy = (my - cropView.oy) / cropView.k;
  cropView.k = next;
  cropView.ox = mx - ix * next;
  cropView.oy = my - iy * next;
  clampCropOffset();
  applyCropTransform();
}

// Map the viewport back into image coordinates → normalized crop rect.
function getCrop() {
  const { vw, vh, iw, ih, k, ox, oy } = cropView;
  const denomX = k * iw;
  const denomY = k * ih;
  if (!denomX || !denomY) return null;
  return {
    x: Math.min(1, Math.max(0, -ox / denomX)),
    y: Math.min(1, Math.max(0, -oy / denomY)),
    w: Math.min(1, Math.max(0, vw / denomX)),
    h: Math.min(1, Math.max(0, vh / denomY)),
  };
}

function closeCropPanel() {
  document.getElementById('cropOverlay').hidden = true;
  if (cropView.url) { URL.revokeObjectURL(cropView.url); cropView.url = null; }
  cropImageEl().removeAttribute('src');
}

function openCropPanel(pngBytes) {
  return new Promise((resolve) => {
    const overlay = document.getElementById('cropOverlay');
    const img = cropImageEl();
    const url = URL.createObjectURL(new Blob([pngBytes], { type: 'image/png' }));
    cropView.url = url;
    cropView.resolve = resolve;
    img.onload = () => {
      cropView.iw = img.naturalWidth;
      cropView.ih = img.naturalHeight;
      img.style.width = cropView.iw + 'px';
      img.style.height = cropView.ih + 'px';
      // Show the overlay before measuring so the viewport has a real layout
      // (a hidden element reports clientWidth/clientHeight == 0, which would
      // zero the scale and make the crop a NaN).
      overlay.hidden = false;
      const viewport = cropViewportEl();
      cropView.vw = viewport.clientWidth;
      cropView.vh = viewport.clientHeight;
      cropView.k0 = Math.max(cropView.vw / cropView.iw, cropView.vh / cropView.ih);
      cropView.maxK = cropView.k0 * 8;
      resetCrop();
    };
    img.src = url;
  });
}

function bindCropPanel() {
  const viewport = cropViewportEl();
  let dragging = false;
  let lastX = 0;
  let lastY = 0;

  viewport.addEventListener('wheel', (e) => {
    e.preventDefault();
    const rect = viewport.getBoundingClientRect();
    cropZoomAt(e.clientX - rect.left, e.clientY - rect.top, Math.exp(-e.deltaY * 0.0015));
  }, { passive: false });

  viewport.addEventListener('pointerdown', (e) => {
    dragging = true;
    lastX = e.clientX;
    lastY = e.clientY;
    viewport.classList.add('dragging');
    viewport.setPointerCapture(e.pointerId);
  });
  viewport.addEventListener('pointermove', (e) => {
    if (!dragging) return;
    cropView.ox += e.clientX - lastX;
    cropView.oy += e.clientY - lastY;
    lastX = e.clientX;
    lastY = e.clientY;
    clampCropOffset();
    applyCropTransform();
  });
  const endDrag = () => {
    dragging = false;
    viewport.classList.remove('dragging');
  };
  viewport.addEventListener('pointerup', endDrag);
  viewport.addEventListener('pointercancel', endDrag);

  document.getElementById('cropZoomIn').addEventListener('click', () =>
    cropZoomAt(cropView.vw / 2, cropView.vh / 2, 1.25));
  document.getElementById('cropZoomOut').addEventListener('click', () =>
    cropZoomAt(cropView.vw / 2, cropView.vh / 2, 1 / 1.25));
  document.getElementById('cropReset').addEventListener('click', resetCrop);
  document.getElementById('cropCancel').addEventListener('click', () => {
    closeCropPanel();
    cropView.resolve(null);
  });
  document.getElementById('cropConfirm').addEventListener('click', () => {
    const crop = getCrop();
    closeCropPanel();
    cropView.resolve(crop);
  });
}

function bindEvents() {
  let debounceTimer = null;
  const scheduleRender = () => {
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(renderPreview, 250);
  };

  form().addEventListener('input', scheduleRender);
  form().addEventListener('change', scheduleRender);
  field('kind').addEventListener('change', () => {
    updateKindUI();
    scheduleRender();
  });

  document.getElementById('artInput').addEventListener('change', (e) => {
    handleArtFile(e.target.files[0]);
  });

  document.getElementById('btnExportPng').addEventListener('click', exportPng);
  document.getElementById('btnReset').addEventListener('click', async () => {
    artBytes = null;
    cropState = null;
    document.getElementById('artInput').value = '';
    await loadDefaultCard();
    renderPreview();
  });

  bindCropPanel();

  // Keep the shared language setting in sync across tabs.
  window.addEventListener('storage', (e) => {
    if (e.key === 'lang' && e.newValue && LANG_NAMES[e.newValue]) {
      currentLang = e.newValue;
      updateLangUI();
      renderPreview();
    }
  });
}

async function main() {
  // Kick off the heavy downloads (fonts + default art) immediately so they run
  // in parallel with the wasm module loading.
  const fontDownload = startFontDownload(currentLang);
  document.documentElement.lang = LANG_HTML[currentLang] || 'zh-CN';
  buildLangDropdown();
  updateLangUI();
  const cardPromise = loadDefaultCard();
  try {
    await init();
    const ver = document.getElementById('ver');
    if (ver) ver.textContent = 'v' + version();
  } catch (e) {
    console.error(e);
  }
  await cardPromise;
  registerFonts(currentLang, await fontDownload);
  bindEvents();
  renderPreview();
}

main();
