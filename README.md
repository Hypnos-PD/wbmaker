# wbmaker — 网页端《影之诗：超凡世界》制卡器

基于 **Rust → WebAssembly** 的卡牌制作工具：前端是纯静态 HTML/CSS/JS，卡图渲染核心（立绘裁切 + 官方边框 + 职业图标 + 全部文字）由 Rust 编译成 wasm 完成。

## 功能

- **官方 WB 卡框**：沿用 WBArts / wbunpacker 的 `frame2d_*.png` 边框（随从/护符/法术 × 铜/银/金/虹，外加 `style_101`、`high_premium` 等特殊框）。
- **权威版式**：渲染坐标与 `wbunpacker/config/render.toml` 一致（782×1024 画布）。
- **文字**：卡名（魏碑 `dfweibeiw7-gb.ttc`）+ 数字（`Junicode-Bold.ttf`），白字黑投影、超宽自动缩小。
- **正文关键词高亮**：`【关键词】`、`『关键词』`、`[b]关键词[/b]` 显示为金色；`_数字` 保持白色。
- **完整字段**：正文 / 第二段正文 / 进化 / 超进化 / 纹章 / 画师 / DIY 作者 / 正文底透明度 / 字号。
- **立绘上传**：任意 PNG/JPG/WebP，浏览器转码后交给 wasm 做 cover 裁切。
- **导出**：PNG（1x/2x/3x）、JSON（可回读）。

## 目录结构

```
wbmaker/
├── Cargo.toml
├── src/
│   ├── lib.rs      # wasm-bindgen 入口（render_card / list_frames / version）
│   ├── card.rs     # CardConfig（serde）
│   ├── render.rs   # 合成管线 + 版式常量
│   └── text.rs     # ab_glyph 文字引擎（阴影/居中/换行/关键词高亮）
├── assets/         # 从 WBArts 复制的边框/图标/字体（编译时内嵌进 wasm）
│   ├── frames/
│   ├── icons/
│   └── fonts/
└── web/
    ├── index.html
    ├── style.css
    ├── app.js       # UI 逻辑 + wasm 调用
    ├── test-node.mjs
    └── pkg/         # 构建产物（wasm + js 胶水）
```

## 构建

依赖：Rust（`wasm32-unknown-unknown` target）+ `wasm-bindgen-cli`。

```bash
./build.sh
```

等价于：

```bash
cargo build --target wasm32-unknown-unknown --release
wasm-bindgen target/wasm32-unknown-unknown/release/wbmaker.wasm \
  --out-dir web/pkg --target web
```

## 本地运行

```bash
python3 -m http.server -d web 8000
# 浏览器打开 http://localhost:8000
```

> 需通过 HTTP 访问（ES Module + wasm fetch）；直接双击 `file://` 打开可能因 CORS 无法加载 wasm。

## 无浏览器冒烟测试

```bash
cd web && node test-node.mjs
# 生成 /tmp/wbmaker_test.png
```

## 字体与数字

制卡器使用从游戏本地安装包 `ShadowverseWB_Data/data.unity3d`（Steam，非 CDN）解包得到的**真实字体**，**每种语言一套**（卡名 + 正文）：

| 语言 | 卡名/标题字体 | 正文字体 |
|------|--------------|---------|
| 简中 chs | 文鼎粗魏碑体GBPro `arweibeigbpro_bd.otf` | Noto Sans CJK SC |
| 繁中 cht | 華康魏碑體 `DFT_W7-930.ttf` | Noto Sans CJK TC |
| 日文 jpn | 解ミン 月 B `MOC-KaiminTsuki-B.otf` | Noto Sans CJK JP |
| 韩文 kor | 나눔고딕 `NanumGothic-ExtraBold.ttf` | NanumGothic |
| 英文 eng | Noto Sans CJK JP | Noto Sans CJK JP |

- **数字**（费用/攻击/体力）：全语言统一用明朝体 `MOC-KaiminTsuki-B.otf`（对应游戏里的筑紫明朝/HG明朝E SDF 数字），白字+投影，字号 106 可调。
- 字体为**运行时按需加载**（浏览器 fetch `web/fonts/` 后注册给 wasm），wasm 本体只有 ~2.5MB。
- 每个字体会对缺的字自动回退到该语言的正文 Noto 字体（覆盖完整 CJK）。

> 游戏里的 SDF 字体（筑紫明朝、HG明朝E、Rodin）只有预光栅化图集；矢量源字体在本地 `data.unity3d`（上述 8 个）。数字用明朝矢量字体直接光栅化，比"SDF 图集放大"清晰。

> [Cygames](https://www.cygames.co.jp/) 保留游戏内所有图像、音频及商标版权。本工具为非官方粉丝用途，边框/图标/字体素材来自游戏客户端解包。
