# wbmaker — 网页端《影之诗：超凡世界》制卡器

基于 **Rust → WebAssembly** 的卡牌制作工具：前端是纯静态 HTML/CSS/JS，卡图渲染核心（立绘裁切 + 边框 + 职业图标 + 全部文字）由 Rust 编译成 wasm 完成。页面提供 **单卡图 / 效果图** 两种产物切换，共享的卡牌信息（卡名、职业、种类、稀有度、费用、攻防、立绘）在切换中保留。

## 功能

### 单卡图（默认模式，官方 WB 卡）

- **官方 WB 卡框**：沿用 WBArts / wbunpacker 的 `frame2d_*.png` 边框（随从/护符/法术 × 铜/银/金/虹，外加 `style_101` 等特殊框）。
- **权威版式**：渲染坐标与 `wbunpacker/config/render.toml` 一致（782×1024 画布）。
- **文字**：卡名 + 数字，白字黑投影、超宽自动缩小。
- **导出**：PNG（1x/2x/3x）。

### 效果图（DIY 模式，称号带与描述面板移植自「欧丝的印卡机」sv-byd-diy）

- 1920×1080 产物：顶部称号带 + 左侧卡牌 + 右侧描述面板，黑底导出（1x）。
- **卡牌部分完全复用单卡图渲染管线**（官方卡框/数字/立绘裁切，同一个 `render` 函数，按 `CARD_SCALE` 缩放后摆放在 `CARD_POS_X/Y`）。
- 职业背景（BYD）、称号带职业图标。
- 文字段落：正文 / 进化 / 超进化 / 纹章，各自独立开关与字号；`[b]` 金色、`[i]` 斜体、`【】『』` 黄色关键词、`[hr]` 插入分割线。
- 纹章区（能力面板，可添加多个）：名称、边框 4 选 1（纹章/信仰/激奏/结晶）、名称区域缩放、两枚纹章图标（14 个内置 + 本地上传）；名称与正文首字水平对齐，图标跟在名称之后。
- 画师（Noto Sans CJK 粗体）/ 脚注（Noto Sans CJK 粗体，自动加 ※ 前缀，默认「卡牌由WBArts驱动生成」）署名行、正文底透明度滑条。
- 立绘裁切与单卡图完全一致（590:711 取景框，归一化裁切矩形两种模式通用）。

## 目录结构

```
wbmaker/
├── Cargo.toml
├── src/
│   ├── lib.rs      # wasm-bindgen 入口（render_card / render_diy_card / list_frames / list_diy_crests）
│   ├── card.rs     # CardConfig（serde），含共享字段与 DIY 专属字段
│   ├── render.rs   # 官方卡合成管线 + 版式常量
│   ├── diy.rs      # BYD-DIY 合成管线（1920×1080）+ 内嵌素材表
│   └── text.rs     # ab_glyph 文字引擎（阴影/居中/换行/富文本/斜体/数字辉光）
├── assets/         # 编译时内嵌进 wasm
│   ├── frames/ icons/ fonts/   # 官方卡素材与字体（运行时加载）
│   └── diy/        # DIY 素材（预处理压缩后内嵌：边框/背景/图标/纹章）
├── tools/prep_diy_assets.py   # DIY 素材预处理脚本（从 sv-byd-diy 复制+压缩）
└── web/
    ├── index.html  # 双模式面板 + 切换按钮
    ├── style.css / topbar.css
    ├── app.js      # UI 逻辑 + wasm 调用
    ├── crests/     # 内置纹章缩略图（选择器用）
    ├── fonts/      # 运行时加载的字体（gitignore，build.sh 复制）
    ├── test-node.mjs
    └── pkg/        # 构建产物（wasm + js 胶水）
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
# 生成 /tmp/wbmaker_wb.png（官方卡）与 /tmp/wbmaker_diy.png（DIY 卡）
```

## 字体

- **卡名/正文**：按语言从游戏本地安装包 `ShadowverseWB_Data/data.unity3d`（Steam，非 CDN）解包得到的矢量字体：简中 `arweibeigbpro_bd.otf`、繁中 `DFT_W7-930.ttf`、日文 `MOC-KaiminTsuki-B.otf`、韩文 `NanumGothic-ExtraBold.ttf`、英文 `MOC-KaiminTsuki-B.otf`。DIY 模式的正文复用各语言标题字体。
- **数字**（费用/攻击/体力）：全语言统一用筑紫明朝数字 `FOT-TsukuAOldMin-Pr6-E.digits.otf`。
- 字体为**运行时按需加载**（浏览器 fetch `web/fonts/` 后注册给 wasm），不占 wasm 体积；DIY 素材预处理压缩后内嵌进 wasm。

## 与「欧丝的印卡机」的关系

DIY 模式的版式与素材移植自 sv-byd-diy（Godot 项目），并按 wbm 的权威口径修正了若干问题（英文卡名改用 MOC-KaiminTsuki-B、数字改用筑紫明朝、正文不再用魏碑、清理 `brone` 拼写与失效逻辑）。两端的卡牌字段一一对应，但暂不互通数据文件。

> [Cygames](https://www.cygames.co.jp/) 保留游戏内所有图像、音频及商标版权。本工具为非官方粉丝用途，边框/图标/字体素材来自游戏客户端解包；DIY 风格素材来自「欧丝的印卡机」。
