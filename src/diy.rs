//! 效果图 (BYD-DIY style product image) renderer.
//!
//! Produces the 1920x1080 product image: title band on top, the official WB
//! card (rendered by the same pipeline as the 单卡图 mode) on the left and the
//! description panel on the right. Title band and description panel layout
//! constants are ported from sv-byd-diy's `ui/main.tscn` / `ui/card_panel.gd`.

use ab_glyph::{Font, ScaleFont};
use crate::card::{CardConfig, CrestBlock};
use crate::render::{fill_rect, blit_cover};
use crate::text::TextEngine;
use image::{DynamicImage, ImageFormat, RgbaImage};

pub const DIY_W: u32 = 1920;
pub const DIY_H: u32 = 1080;

// ---- card block (official WB card, reused from the 单卡图 pipeline) ----
/// The official card is rendered via `crate::render::render` at this scale and
/// blitted here. 0.75 => 587x768 on the 1920x1080 canvas.
const CARD_SCALE: f32 = 0.75;
const CARD_POS_X: f32 = 132.0;
const CARD_POS_Y: f32 = 211.0;

// ---- title band ----
const TITLE_NAME_X: f32 = 172.0;
const TITLE_NAME_Y: f32 = 82.0;
const TITLE_NAME_SIZE: f32 = 60.0;
const TITLE_SIDE_SIZE: f32 = 28.0;
const TITLE_GOLD: [u8; 4] = [213, 184, 137, 255];
const TITLE_CLASS_TITLE_RIGHT: f32 = 1287.0;
const TITLE_CLASS_TITLE_Y: f32 = 72.0;
const TITLE_CLASS_X: f32 = 1347.0; // rect left (Godot: offset_left=1347)
const TITLE_CLASS_Y: f32 = 74.0; // rect top (Godot: offset_top=74)
const TITLE_TYPE_TITLE_RIGHT: f32 = 1288.0;
const TITLE_TYPE_TITLE_Y: f32 = 120.0;
const TITLE_TYPE_X: f32 = 1302.0; // rect left (Godot: offset_left=1302)
const TITLE_TYPE_Y: f32 = 121.0; // rect top (Godot: offset_top=121)
const TITLE_ICON_X: f32 = 1303.0;
const TITLE_ICON_Y: f32 = 73.0;
const TITLE_ICON_S: f32 = 33.7;

// ---- description panel ----
const DETAIL_X: f32 = 730.0;
const DETAIL_Y: f32 = 217.0;
const DETAIL_W: f32 = 1059.0;
const DETAIL_H: f32 = 737.2;
const VB_X: f32 = 770.0;
const VB_Y: f32 = 266.6;
const VB_W: f32 = 969.2;
const TEXT_INSET: f32 = 12.0;
const SECTION_GAP: f32 = 20.0; // VBox separation 50 @0.4
const LINE_GAP: f32 = 8.0; // line_separation 20 @0.4
const PARA_GAP: f32 = 8.0; // paragraph_separation 20 @0.4
const DEFAULT_TEXT_SIZE: f32 = 32.4; // 81 @0.4
const EV_TEXT_TOP: f32 = 20.0; // content_margin_top 50 @0.4
const EV_TEXT_BOTTOM: f32 = 8.0; // texture margin 20 @0.4
// crest banner band
const CREST_BAND_DX: f32 = 8.0; // 20 @0.4
const CREST_BAND_DY: f32 = 20.0; // 50 @0.4
const CREST_BAND_W: f32 = 618.4; // (1566-20) @0.4
const CREST_BAND_H: f32 = 67.2; // (218-50) @0.4
const CREST_ICON_SIDE: f32 = 59.2; // ~148 @0.4
const CREST_TEXT_GAP: f32 = 4.0;
const CREST_BOTTOM: f32 = 12.0; // texture margin 30 @0.4
// signature rows (fixed on the detail panel)
const ILLU_X: f32 = 800.0; // 画师行左边缘（标签左对齐，名字紧跟其后）
const ILLU_TITLE_Y: f32 = 878.6;
const ILLU_SIZE: f32 = 28.0; // 画师行字号（比正文 32.4 略大）
const DIY_X: f32 = 732.0;
const DIY_Y: f32 = 1020.0; // 下移
const DIY_RIGHT: f32 = 1788.8;
const DIY_SIZE: f32 = 26.0;
const SPLIT_ALPHA: f32 = 0.5; // 分隔线透明度

// wbm class (0=neutral..7=portal) -> sv-byd-diy asset name
const DIY_CLASSES: [&str; 8] = [
    "neutral",
    "forestcraft",
    "swordcraft",
    "runecraft",
    "dragoncraft",
    "abysscraft",
    "havencraft",
    "portalcraft",
];
const CREST_BORDERS: [&str; 4] = ["Crest", "Faith", "Accelerate", "Crystallize"];
/// Built-in crest icons: cost_0..cost_10 + 2 extra.
pub const CREST_BUILTIN: [&str; 13] = [
    "cost_0",
    "cost_1",
    "cost_2",
    "cost_3",
    "cost_4",
    "cost_5",
    "cost_6",
    "cost_7",
    "cost_8",
    "cost_9",
    "cost_10",
    "5d4245b4f4bd3b69d2e891b5e253dc0e",
    "89825586aad57905eec3bc02e2ec141f",
];

// ---- embedded assets ----

pub fn diy_background_bytes(cls: &str, gen: u8) -> Option<&'static [u8]> {
    let bytes = match (cls, gen) {
        ("neutral", 1) => include_bytes!("../assets/diy/backgrounds/neutral-1.jpg") as &[u8],
        ("neutral", 2) => include_bytes!("../assets/diy/backgrounds/neutral-2.jpg") as &[u8],
        ("forestcraft", 1) => include_bytes!("../assets/diy/backgrounds/forestcraft-1.jpg") as &[u8],
        ("forestcraft", 2) => include_bytes!("../assets/diy/backgrounds/forestcraft-2.jpg") as &[u8],
        ("swordcraft", 1) => include_bytes!("../assets/diy/backgrounds/swordcraft-1.jpg") as &[u8],
        ("swordcraft", 2) => include_bytes!("../assets/diy/backgrounds/swordcraft-2.jpg") as &[u8],
        ("runecraft", 1) => include_bytes!("../assets/diy/backgrounds/runecraft-1.jpg") as &[u8],
        ("runecraft", 2) => include_bytes!("../assets/diy/backgrounds/runecraft-2.jpg") as &[u8],
        ("dragoncraft", 1) => include_bytes!("../assets/diy/backgrounds/dragoncraft-1.jpg") as &[u8],
        ("dragoncraft", 2) => include_bytes!("../assets/diy/backgrounds/dragoncraft-2.jpg") as &[u8],
        ("abysscraft", 1) => include_bytes!("../assets/diy/backgrounds/abysscraft-1.jpg") as &[u8],
        ("abysscraft", 2) => include_bytes!("../assets/diy/backgrounds/abysscraft-2.jpg") as &[u8],
        ("havencraft", 1) => include_bytes!("../assets/diy/backgrounds/havencraft-1.jpg") as &[u8],
        ("havencraft", 2) => include_bytes!("../assets/diy/backgrounds/havencraft-2.jpg") as &[u8],
        ("portalcraft", 1) => include_bytes!("../assets/diy/backgrounds/portalcraft-1.jpg") as &[u8],
        ("portalcraft", 2) => include_bytes!("../assets/diy/backgrounds/portalcraft-2.jpg") as &[u8],
        _ => return None,
    };
    Some(bytes)
}

pub fn diy_title_class_bytes(cls: &str) -> Option<&'static [u8]> {
    let bytes = match cls {
        "neutral" => include_bytes!("../assets/diy/title_class/neutral.png") as &[u8],
        "forestcraft" => include_bytes!("../assets/diy/title_class/forestcraft.png") as &[u8],
        "swordcraft" => include_bytes!("../assets/diy/title_class/swordcraft.png") as &[u8],
        "runecraft" => include_bytes!("../assets/diy/title_class/runecraft.png") as &[u8],
        "dragoncraft" => include_bytes!("../assets/diy/title_class/dragoncraft.png") as &[u8],
        "abysscraft" => include_bytes!("../assets/diy/title_class/abysscraft.png") as &[u8],
        "havencraft" => include_bytes!("../assets/diy/title_class/havencraft.png") as &[u8],
        "portalcraft" => include_bytes!("../assets/diy/title_class/portalcraft.png") as &[u8],
        _ => return None,
    };
    Some(bytes)
}

pub fn diy_effect_bytes(key: &str) -> Option<&'static [u8]> {
    let bytes = match key {
        "title_bg" => include_bytes!("../assets/diy/effect/title_bg.png") as &[u8],
        "title_bottom" => include_bytes!("../assets/diy/effect/title_bottom.png") as &[u8],
        "card_detail_background" => {
            include_bytes!("../assets/diy/effect/card_detail_background.png") as &[u8]
        }
        "detail_spit" => include_bytes!("../assets/diy/effect/detail_spit.png") as &[u8],
        "evolve" => include_bytes!("../assets/diy/effect/evolve.png") as &[u8],
        "super_evolve" => include_bytes!("../assets/diy/effect/super_evolve.png") as &[u8],
        "detail_crest" => include_bytes!("../assets/diy/effect/detail_crest.png") as &[u8],
        "Crest" => include_bytes!("../assets/diy/effect/Crest.png") as &[u8],
        "Faith" => include_bytes!("../assets/diy/effect/Faith.png") as &[u8],
        "Accelerate" => include_bytes!("../assets/diy/effect/Accelerate.png") as &[u8],
        "Crystallize" => include_bytes!("../assets/diy/effect/Crystallize.png") as &[u8],
        _ => return None,
    };
    Some(bytes)
}

pub fn diy_crest_bytes(name: &str) -> Option<&'static [u8]> {
    let bytes = match name {
        "cost_0" => include_bytes!("../assets/diy/crests/cost_0.png") as &[u8],
        "cost_1" => include_bytes!("../assets/diy/crests/cost_1.png") as &[u8],
        "cost_2" => include_bytes!("../assets/diy/crests/cost_2.png") as &[u8],
        "cost_3" => include_bytes!("../assets/diy/crests/cost_3.png") as &[u8],
        "cost_4" => include_bytes!("../assets/diy/crests/cost_4.png") as &[u8],
        "cost_5" => include_bytes!("../assets/diy/crests/cost_5.png") as &[u8],
        "cost_6" => include_bytes!("../assets/diy/crests/cost_6.png") as &[u8],
        "cost_7" => include_bytes!("../assets/diy/crests/cost_7.png") as &[u8],
        "cost_8" => include_bytes!("../assets/diy/crests/cost_8.png") as &[u8],
        "cost_9" => include_bytes!("../assets/diy/crests/cost_9.png") as &[u8],
        "cost_10" => include_bytes!("../assets/diy/crests/cost_10.png") as &[u8],
        "5d4245b4f4bd3b69d2e891b5e253dc0e" => {
            include_bytes!("../assets/diy/crests/5d4245b4f4bd3b69d2e891b5e253dc0e.png") as &[u8]
        }
        "89825586aad57905eec3bc02e2ec141f" => {
            include_bytes!("../assets/diy/crests/89825586aad57905eec3bc02e2ec141f.png") as &[u8]
        }
        _ => return None,
    };
    Some(bytes)
}

// ---- rendering ----

fn decode(bytes: &[u8], what: &str) -> Result<RgbaImage, String> {
    image::load_from_memory(bytes)
        .map(|d| d.to_rgba8())
        .map_err(|e| format!("{what} decode failed: {e}"))
}

/// Vertically flip the top half of the panel texture onto the bottom half.
/// The texture is top-bottom symmetric except for the signature divider line,
/// whose mirror position is plain background — so the flipped copy has no line.
fn flipped_top_half(img: &RgbaImage) -> RgbaImage {
    let (w, h) = img.dimensions();
    let mut out = RgbaImage::new(w, h);
    for y in 0..h {
        let src_y = if y < h / 2 { y } else { h - 1 - y };
        for x in 0..w {
            out.put_pixel(x, y, *img.get_pixel(x, src_y));
        }
    }
    out
}

/// Stretch-blit `src` into the destination rect.
fn blit_stretch(canvas: &mut RgbaImage, src: &RgbaImage, dx: f32, dy: f32, dw: f32, dh: f32) {
    let dw = dw.round() as u32;
    let dh = dh.round() as u32;
    if dw == 0 || dh == 0 {
        return;
    }
    let resized = image::imageops::resize(src, dw, dh, image::imageops::FilterType::Lanczos3);
    image::imageops::overlay(canvas, &resized, dx.round() as i64, dy.round() as i64);
}

/// ab_glyph 按字体高度(asc+desc)而非 units-per-em 缩放字号：Noto CJK 的
/// 高度为 1448、upm 为 1000，直接传 34 实际只渲染约 23.5px。这里把名义字号
/// 换算成"按 upm 缩放后等于名义值"的输入值。
fn true_size(font: &ab_glyph::FontArc, size: f32) -> f32 {
    let height = font.height_unscaled();
    let upm = font.units_per_em().unwrap_or(height);
    if upm > 0.0 {
        size * height / upm
    } else {
        size
    }
}

/// Resolve the crest icon image: user upload wins, then builtin index.
fn resolve_crest(spec: &str, upload: Option<&[u8]>) -> Result<Option<RgbaImage>, String> {
    if spec == "upload" {
        if let Some(bytes) = upload {
            return decode(bytes, "crest icon").map(Some);
        }
        return Ok(None);
    }
    if let Some(idx) = spec.strip_prefix("builtin:") {
        let n: usize = idx.parse().unwrap_or(usize::MAX);
        if let Some(name) = CREST_BUILTIN.get(n) {
            if let Some(bytes) = diy_crest_bytes(name) {
                return decode(bytes, "crest icon").map(Some);
            }
        }
    }
    Ok(None) // 空规格/未知规格 = 不绘制图标
}

pub fn render_diy(
    config: &CardConfig,
    art_bytes: Option<&[u8]>,
    crest1_png: Option<&[u8]>,
    crest2_png: Option<&[u8]>,
) -> Result<Vec<u8>, String> {
    let mut canvas = RgbaImage::from_pixel(DIY_W, DIY_H, image::Rgba([0, 0, 0, 255]));

    let cls = DIY_CLASSES
        .get(config.class as usize)
        .copied()
        .ok_or_else(|| format!("bad class index {}", config.class))?;
    let bg_gen = if config.bg_type == 1 { 1 } else { 2 };

    // 1. class background (cover-fit, full canvas)
    let bg_bytes = diy_background_bytes(cls, bg_gen).ok_or("missing background asset")?;
    let bg = decode(bg_bytes, "background")?;
    blit_cover(&mut canvas, &bg, 0.0, 0.0, DIY_W as f32, DIY_H as f32);

    // 2. title band
    let title_bg = decode(diy_effect_bytes("title_bg").unwrap(), "title_bg")?;
    blit_stretch(&mut canvas, &title_bg, 0.0, 0.0, DIY_W as f32, DIY_H as f32);
    if bg_gen == 2 {
        let bottom = decode(diy_effect_bytes("title_bottom").unwrap(), "title_bottom")?;
        blit_stretch(&mut canvas, &bottom, 0.0, -4.8, 1920.0, 1115.2);
    }

    // 3. the card: fully reuse the official WB card pipeline (单卡图).
    let mut wb_config = config.clone();
    wb_config.scale = CARD_SCALE;
    let card_png = crate::render::render(&wb_config, art_bytes)?;
    let card_img = decode(&card_png, "card render")?;
    image::imageops::overlay(
        &mut canvas,
        &card_img,
        CARD_POS_X.round() as i64,
        CARD_POS_Y.round() as i64,
    );

    // 4. title band texts
    let engine = TextEngine::for_language(&config.language)?;
    let title_name: String = config.name.chars().filter(|c| *c != ' ').collect();
    if !title_name.is_empty() {
        engine.draw_plain(
            &mut canvas,
            &engine.title,
            &title_name,
            TITLE_NAME_X,
            TITLE_NAME_Y,
            TITLE_NAME_SIZE,
            crate::text::BODY,
            0.0,
        );
    }
    if let Some(ic) = diy_title_class_bytes(cls) {
        let icon = decode(ic, "title class icon")?;
        blit_stretch(&mut canvas, &icon, TITLE_ICON_X, TITLE_ICON_Y, TITLE_ICON_S, TITLE_ICON_S);
    }
    let (w, _) = engine.measure(&engine.title, &config.class_title, TITLE_SIDE_SIZE);
    engine.draw_plain(
        &mut canvas,
        &engine.title,
        &config.class_title,
        TITLE_CLASS_TITLE_RIGHT - w,
        TITLE_CLASS_TITLE_Y,
        TITLE_SIDE_SIZE,
        TITLE_GOLD,
        0.0,
    );
    engine.draw_plain(
        &mut canvas,
        &engine.title,
        &config.class_text,
        TITLE_CLASS_X,
        TITLE_CLASS_Y,
        TITLE_SIDE_SIZE,
        crate::text::BODY,
        0.0,
    );
    let (w, _) = engine.measure(&engine.title, &config.type_title, TITLE_SIDE_SIZE);
    engine.draw_plain(
        &mut canvas,
        &engine.title,
        &config.type_title,
        TITLE_TYPE_TITLE_RIGHT - w,
        TITLE_TYPE_TITLE_Y,
        TITLE_SIDE_SIZE,
        TITLE_GOLD,
        0.0,
    );
    engine.draw_plain(
        &mut canvas,
        &engine.title,
        &config.trait_text,
        TITLE_TYPE_X,
        TITLE_TYPE_Y,
        TITLE_SIDE_SIZE,
        crate::text::BODY,
        0.0,
    );

    // 5. description panel
    fill_rect(
        &mut canvas,
        DETAIL_X,
        DETAIL_Y,
        DETAIL_W,
        DETAIL_H,
        [0, 0, 0, (config.bg_alpha.clamp(0.0, 1.0) * 255.0) as u8],
    );
    let detail_bg = decode(
        diy_effect_bytes("card_detail_background").unwrap(),
        "card_detail_background",
    )?;
    // 贴图除"画师行上方分隔线"外上下轴对称：没有画师信息时取上半部分
    // 向下翻转的版本（该线无镜像对应，翻转过后的底部即无线条）。
    let detail_bg = if config.show_illustrator && !config.illustrator.trim().is_empty() {
        detail_bg
    } else {
        flipped_top_half(&detail_bg)
    };
    blit_stretch(&mut canvas, &detail_bg, DETAIL_X, DETAIL_Y, DETAIL_W, DETAIL_H);
    // 分割线贴图为不透明白线，按 SPLIT_ALPHA 降透明度后再绘制
    let spit_img: Option<RgbaImage> = match diy_effect_bytes("detail_spit") {
        Some(b) => {
            let mut im = decode(b, "detail_spit")?;
            for p in im.pixels_mut() {
                p[3] = ((p[3] as f32 * SPLIT_ALPHA).round() as u8).min(p[3]);
            }
            Some(im)
        }
        None => None,
    };

    let mut y = VB_Y;
    let text_x = VB_X + TEXT_INSET;
    let text_w = VB_W - TEXT_INSET * 2.0;
    let d1 = &config.detail1;
    let d2 = &config.detail2;
    let ev = &config.evolve;
    let sup = &config.super_evolve;
    let has_d1 = !d1.trim().is_empty();

    // d1
    draw_plain_section(&mut canvas, &engine, text_x, &mut y, text_w, &spit_img, d1, config.d1_size);
    // split + d2
    if config.show_detail2 {
        draw_split(&mut canvas, &spit_img, VB_X, &mut y, VB_W);
        draw_plain_section(&mut canvas, &engine, text_x, &mut y, text_w, &spit_img, d2, config.d2_size);
    }
    // split2 before evolve/super
    if has_d1 && (config.show_evolve || config.show_super) {
        draw_split(&mut canvas, &spit_img, VB_X, &mut y, VB_W);
    }
    // evolve
    if config.show_evolve {
        let h = measure_rich(&engine, ev, text_w, config.ev_size);
        let bh = h + EV_TEXT_TOP + EV_TEXT_BOTTOM;
        let banner = decode(diy_effect_bytes("evolve").unwrap(), "evolve banner")?;
        blit_stretch(&mut canvas, &banner, VB_X, y, VB_W, bh);
        if !ev.trim().is_empty() {
            engine.draw_wrapped_rich(
                &mut canvas,
                &engine.title,
                ev,
                text_x,
                y + EV_TEXT_TOP,
                text_w,
                config.ev_size,
                LINE_GAP,
                PARA_GAP,
                spit_img.as_ref(),
            );
        }
        y += bh + SECTION_GAP;
    }
    // super evolve
    if config.show_super {
        let h = measure_rich(&engine, sup, text_w, config.super_size);
        let bh = h + EV_TEXT_TOP + EV_TEXT_BOTTOM;
        let banner = decode(diy_effect_bytes("super_evolve").unwrap(), "super banner")?;
        blit_stretch(&mut canvas, &banner, VB_X, y, VB_W, bh);
        if !sup.trim().is_empty() {
            engine.draw_wrapped_rich(
                &mut canvas,
                &engine.title,
                sup,
                text_x,
                y + EV_TEXT_TOP,
                text_w,
                config.super_size,
                LINE_GAP,
                PARA_GAP,
                spit_img.as_ref(),
            );
        }
        y += bh + SECTION_GAP;
    }
    // crest blocks（能力面板可添加多个；兼容旧的单纹章字段）
    if !config.crests.is_empty() {
        for block in &config.crests {
            y += draw_crest_block(&mut canvas, &engine, block, &spit_img, crest1_png, crest2_png, text_x, text_w, y)?;
        }
    } else if config.show_crest {
        let legacy = CrestBlock {
            name: config.crest_name.clone(),
            text: config.crest.clone(),
            border: config.crest_border,
            scale: config.crest_scale,
            icon1: config.crest_icon1.clone(),
            icon2: config.crest_icon2.clone(),
            show_icon2: config.show_crest_icon2,
            size: config.crest_size,
        };
        y += draw_crest_block(&mut canvas, &engine, &legacy, &spit_img, crest1_png, crest2_png, text_x, text_w, y)?;
    }

    // 6. signature rows
    if config.show_illustrator && !config.illustrator.trim().is_empty() {
        // 画师行：Noto Sans CJK 对应语言版本（粗体），缺失时回退标题字体
        let illus_font = crate::text::font_by_key(&format!("illus_{}", config.language))
            .unwrap_or_else(|| engine.title.clone());
        // 标签左对齐，内容直接紧接其后，无需单独定位
        let label = if config.illus_title.is_empty() { "画师:" } else { &config.illus_title };
        let illus_scale = true_size(&illus_font, ILLU_SIZE);
        engine.draw_plain(
            &mut canvas,
            &illus_font,
            label,
            ILLU_X,
            ILLU_TITLE_Y,
            illus_scale,
            crate::text::BODY,
            0.0,
        );
        let (lw, _) = engine.measure(&illus_font, label, illus_scale);
        engine.draw_plain(
            &mut canvas,
            &illus_font,
            &config.illustrator,
            ILLU_X + lw,
            ILLU_TITLE_Y,
            illus_scale,
            crate::text::BODY,
            0.0,
        );
    }
    if config.show_diy && !config.diy.is_empty() {
        // 脚注：Noto Sans CJK 对应语言版本（常规），缺失时回退标题字体
        let footnote_font = crate::text::font_by_key(&format!("footnote_{}", config.language))
            .unwrap_or_else(|| engine.title.clone());
        // ※ 是必定前缀，不依赖用户填写
        let footnote_text = format!("※{}", config.diy);
        let footnote_scale = true_size(&footnote_font, DIY_SIZE);
        let (w, _) = engine.measure(&footnote_font, &footnote_text, footnote_scale);
        engine.draw_plain(
            &mut canvas,
            &footnote_font,
            &footnote_text,
            (DIY_RIGHT - w).max(DIY_X),
            DIY_Y,
            footnote_scale,
            crate::text::BODY,
            0.0,
        );
    }

    let mut out = Vec::new();
    DynamicImage::ImageRgba8(canvas)
        .write_to(&mut std::io::Cursor::new(&mut out), ImageFormat::Png)
        .map_err(|e| format!("png encode failed: {e}"))?;
    Ok(out)
}

/// Height (px) a wrapped rich text block will occupy. Mirrors draw_wrapped_rich.
fn measure_rich(engine: &TextEngine, text: &str, max_w: f32, size: f32) -> f32 {
    if text.trim().is_empty() {
        return 0.0;
    }
    // Quick estimate via draw into a scratch image is wasteful; instead we
    // measure with the same tokenizer: lines * (font height + line gap).
    let sf = ab_glyph::Font::as_scaled(&engine.title, ab_glyph::PxScale::from(size));
    let line_h = sf.height() + LINE_GAP;
    let mut lines = 1usize;
    let mut cur_w = 0.0f32;
    let runs = crate::text::parse_rich(text);
    for run in &runs {
        if run.split {
            lines += 1;
            cur_w = 0.0;
            continue;
        }
        let mut j = 0;
        while j < run.text.len() {
            let ch = run.text[j..].chars().next().unwrap();
            if ch == '\n' {
                lines += 1;
                cur_w = 0.0;
                j += 1;
                continue;
            }
            if ch == ' ' {
                j += 1;
                continue;
            }
            let start = j;
            let stop = if ch.is_ascii() {
                j += 1;
                while j < run.text.len() {
                    let c = run.text[j..].chars().next().unwrap();
                    if c == ' ' || c == '\n' || !c.is_ascii() {
                        break;
                    }
                    j += 1;
                }
                j
            } else {
                j += ch.len_utf8();
                j
            };
            let has_trail = stop < run.text.len() && run.text[stop..].starts_with(' ');
            let word: String = run.text[start..stop].chars().collect();
            let mut w = 0.0f32;
            let mut prev: Option<ab_glyph::GlyphId> = None;
            for c in word.chars() {
                let gid = sf.glyph_id(c);
                if let Some(p) = prev {
                    w += sf.kern(p, gid);
                }
                w += sf.h_advance(gid);
                prev = Some(gid);
            }
            if has_trail {
                w += sf.h_advance(sf.glyph_id(' '));
            }
            if cur_w + w > max_w && cur_w > 0.0 {
                lines += 1;
                cur_w = 0.0;
            }
            cur_w += w;
        }
    }
    lines as f32 * line_h
}

fn draw_split(
    canvas: &mut RgbaImage,
    spit_img: &Option<RgbaImage>,
    x: f32,
    y: &mut f32,
    w: f32,
) {
    if let Some(img) = spit_img {
        let sh = (img.height() as f32 * 1.03).max(2.0);
        blit_stretch(canvas, img, x, *y, w * 1.03, sh);
        *y += sh + SECTION_GAP;
    } else {
        *y += 2.0 + SECTION_GAP;
    }
}

/// Draw one crest block: banner band（名字优先、图标跟在名字后面）+ 描述文字。
/// 名字与描述的首字与正文首字水平对齐（x = text_x）。返回块占用的总高度。
#[allow(clippy::too_many_arguments)]
fn draw_crest_block(
    canvas: &mut RgbaImage,
    engine: &TextEngine,
    block: &CrestBlock,
    spit_img: &Option<RgbaImage>,
    crest1_png: Option<&[u8]>,
    crest2_png: Option<&[u8]>,
    text_x: f32,
    text_w: f32,
    y: f32,
) -> Result<f32, String> {
    let v = block.scale.clamp(0.1, 1.5);
    let band_h = CREST_BAND_H * v;
    let text_h = measure_rich(engine, &block.text, text_w, block.size);
    let sec_h = CREST_BAND_DY + band_h + CREST_TEXT_GAP + text_h + CREST_BOTTOM;
    let banner_key = CREST_BORDERS
        .get(block.border as usize)
        .copied()
        .unwrap_or("Crest");
    // section background: detail_crest.png fills the whole section rect
    let sec_banner = decode(
        diy_effect_bytes("detail_crest").unwrap(),
        "crest section banner",
    )?;
    blit_stretch(canvas, &sec_banner, VB_X, y, VB_W, sec_h);
    // icon band: border texture stretched over the band rect（与文字同宽）
    let band_y = y + CREST_BAND_DY;
    let band_banner = decode(diy_effect_bytes(banner_key).unwrap(), "crest band banner")?;
    blit_stretch(canvas, &band_banner, VB_X, band_y, VB_W, band_h);

    // 名字优先：从 text_x 开始，与正文首字对齐；图标跟在名字后面
    let name_size = block.size;
    let mut nx = text_x;
    if !block.name.is_empty() {
        engine.draw_plain(
            canvas,
            &engine.title,
            &block.name,
            nx,
            band_y + (band_h - name_size) / 2.0,
            name_size,
            crate::text::BODY,
            0.0,
        );
        let (nw, _) = engine.measure(&engine.title, &block.name, name_size);
        nx += nw + 10.0;
    }
    let icon_side = (CREST_ICON_SIDE * v).clamp(8.0, band_h);
    let icon1 = resolve_crest(&block.icon1, crest1_png)?;
    if let Some(ic) = icon1 {
        blit_stretch(
            canvas,
            &ic,
            nx,
            band_y + (band_h - icon_side) / 2.0,
            icon_side,
            icon_side,
        );
        nx += icon_side + 4.0;
    }
    if block.show_icon2 {
        let icon2 = resolve_crest(&block.icon2, crest2_png)?;
        if let Some(ic) = icon2 {
            blit_stretch(
                canvas,
                &ic,
                nx,
                band_y + (band_h - icon_side) / 2.0,
                icon_side,
                icon_side,
            );
        }
    }
    if !block.text.trim().is_empty() {
        engine.draw_wrapped_rich(
            canvas,
            &engine.title,
            &block.text,
            text_x,
            y + CREST_BAND_DY + band_h + CREST_TEXT_GAP,
            text_w,
            block.size,
            LINE_GAP,
            PARA_GAP,
            spit_img.as_ref(),
        );
    }
    Ok(sec_h + SECTION_GAP)
}

/// Plain (no banner) rich text section, advancing the VBox cursor.
#[allow(clippy::too_many_arguments)]
fn draw_plain_section(
    canvas: &mut RgbaImage,
    engine: &TextEngine,
    x: f32,
    y: &mut f32,
    max_w: f32,
    spit_img: &Option<RgbaImage>,
    text: &str,
    size: f32,
) {
    if text.trim().is_empty() {
        return;
    }
    let h = engine.draw_wrapped_rich(
        canvas,
        &engine.title,
        text,
        x,
        *y,
        max_w,
        size,
        LINE_GAP,
        PARA_GAP,
        spit_img.as_ref(),
    );
    *y += h + SECTION_GAP;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_mapping() {
        assert_eq!(DIY_CLASSES[0], "neutral");
        assert_eq!(DIY_CLASSES[7], "portalcraft");
        assert_eq!(DIY_CLASSES[5], "abysscraft");
    }

    #[test]
    fn all_diy_assets_exist() {
        for cls in DIY_CLASSES {
            for gen in 1..=2 {
                assert!(diy_background_bytes(cls, gen).is_some(), "{cls}-{gen}");
            }
            assert!(diy_title_class_bytes(cls).is_some(), "title_class {cls}");
        }
        for key in [
            "title_bg",
            "title_bottom",
            "card_detail_background",
            "detail_spit",
            "evolve",
            "super_evolve",
            "detail_crest",
            "Crest",
            "Faith",
            "Accelerate",
            "Crystallize",
        ] {
            assert!(diy_effect_bytes(key).is_some(), "effect {key}");
        }
        for name in CREST_BUILTIN {
            assert!(diy_crest_bytes(name).is_some(), "crest {name}");
        }
    }

    #[test]
    fn render_diy_smoke() {
        // Register a minimal font set (native test only, reads repo assets).
        let fonts_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/fonts");
        let kaimin = std::fs::read(format!("{fonts_dir}/MOC-KaiminTsuki-B.otf")).unwrap();
        crate::text::register_font("title_chs", &kaimin);
        crate::text::register_font("number", &kaimin);
        let cfg = CardConfig {
            name: "测试随从".into(),
            language: "chs".into(),
            class: 4,
            kind: 1,
            rarity: 4,
            frame: "follower_legend".into(),
            cost: "7".into(),
            atk: "5".into(),
            life: "6".into(),
            detail1: "【守护】\n【入场曲】抽取1张卡牌。".into(),
            show_crest: true,
            crest: "纹章 1".into(),
            crest_name: "试制纹章".into(),
            show_diy: true,
            diy: "DIY：某人".into(),
            ..Default::default()
        };
        let out = render_diy(&cfg, None, None, None).expect("render diy");
        assert!(out.len() > 1000);
        let img = image::load_from_memory(&out).expect("decode");
        assert_eq!((img.width(), img.height()), (1920, 1080));
    }
}

#[cfg(test)]
mod flip_tests {
    use super::*;

    #[test]
    fn flipped_top_half_removes_signature_line() {
        let bytes = diy_effect_bytes("card_detail_background").unwrap();
        let im = decode(bytes, "bg").unwrap();
        let flipped = flipped_top_half(&im);
        // 画师行上方分隔线（原图 634-638 行）在翻转版中应变为其镜像行
        // （99-103，纯背景色），即不再有亮线
        for y in 634..=638 {
            for x in (0..flipped.width()).step_by(16) {
                let p = flipped.get_pixel(x, y);
                assert!(p[0] < 60 && p[1] < 60 && p[2] < 60, "row {y} px {x} 仍有亮线: {p:?}");
            }
        }
        // 上半部分原样保留；下半部分是上半部分的镜像
        for (a, b) in [(0u32, 0u32), (100, 100), (636, 100), (368, 368)] {
            assert_eq!(flipped.get_pixel(50, a), im.get_pixel(50, b), "row {a} should mirror {b}");
        }
    }
}
