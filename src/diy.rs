//! BYD-DIY (欧丝的印卡机) style renderer.
//!
//! Produces the 1920x1080 product image: title band on top, the card on the
//! left and the description panel on the right. Layout constants are ported
//! from sv-byd-diy's `ui/main.tscn` / `ui/card_panel.gd` (scales folded in).

use ab_glyph::ScaleFont;
use crate::card::{CardConfig, KIND_FOLLOWER};
use crate::render::{blit_crop, blit_cover, fill_rect};
use crate::text::TextEngine;
use image::{DynamicImage, ImageFormat, RgbaImage};

pub const DIY_W: u32 = 1920;
pub const DIY_H: u32 = 1080;

// ---- card block ----
const CARD_X: f32 = 132.0;
const CARD_Y: f32 = 211.0;
const ART_X: f32 = 183.0;
const ART_Y: f32 = 331.0;
const ART_W: f32 = 464.0;
const ART_H: f32 = 560.0;
// card-name label center (anchors 0.516304/0.133697 of the 552x733 card)
const NAME_CX: f32 = 417.0;
const NAME_CY: f32 = 309.0;
const NAME_SIZE: f32 = 45.0;
const NUM_SIZE: f32 = 110.0;
const COST_CX: f32 = 197.0;
const COST_CY: f32 = 309.5;
const ATK_CX: f32 = 202.0;
const ATK_CY: f32 = 875.0;
const HP_CX: f32 = 629.0;
const HP_CY: f32 = 875.0;
const JEWEL_X: f32 = 399.0;
const JEWEL_Y: f32 = 850.0;
const JEWEL_W: f32 = 34.1;
const JEWEL_H: f32 = 37.4;

// ---- title band ----
const TITLE_NAME_X: f32 = 172.0;
const TITLE_NAME_Y: f32 = 82.0;
const TITLE_NAME_SIZE: f32 = 60.0;
const TITLE_SIDE_SIZE: f32 = 28.0;
const TITLE_GOLD: [u8; 4] = [213, 184, 137, 255];
const TITLE_CLASS_TITLE_RIGHT: f32 = 1287.0;
const TITLE_CLASS_TITLE_Y: f32 = 72.0;
const TITLE_CLASS_CX: f32 = 1375.0;
const TITLE_CLASS_CY: f32 = 90.0;
const TITLE_TYPE_TITLE_RIGHT: f32 = 1288.0;
const TITLE_TYPE_TITLE_Y: f32 = 120.0;
const TITLE_TYPE_CX: f32 = 1322.0;
const TITLE_TYPE_CY: f32 = 137.0;
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
const ILLU_TITLE_X: f32 = 735.8;
const ILLU_TITLE_Y: f32 = 874.6;
const ILLU_X: f32 = 923.4;
const DIY_X: f32 = 732.0;
const DIY_Y: f32 = 960.2;
const DIY_RIGHT: f32 = 1788.8;
const DIY_SIZE: f32 = 30.0; // 75 @0.4

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
// wbm kind (1 follower / 2 amulet / 3 spell) -> sv-byd-diy kind name
fn diy_kind(kind: u8) -> &'static str {
    match kind {
        2 => "amulet",
        3 => "spell",
        _ => "unit",
    }
}
const DIY_RARES: [&str; 5] = ["brone", "silver", "gold", "legend", "peculiar"];
const CREST_BORDERS: [&str; 4] = ["Crest", "Faith", "Accelerate", "Crystallize"];
/// Built-in crest icons, index 0 = default (luna), then cost_0..cost_10, then 2 extra.
pub const CREST_BUILTIN: [&str; 14] = [
    "default_crest",
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

macro_rules! frame_key {
    ($kind:literal, $rare:literal) => {
        concat!("../assets/diy/frames/", $kind, "_", $rare, ".png")
    };
}

pub fn diy_frame_bytes(kind: &str, rare: &str) -> Option<&'static [u8]> {
    let bytes = match (kind, rare) {
        ("unit", "brone") => include_bytes!(frame_key!("unit", "brone")) as &[u8],
        ("unit", "silver") => include_bytes!(frame_key!("unit", "silver")) as &[u8],
        ("unit", "gold") => include_bytes!(frame_key!("unit", "gold")) as &[u8],
        ("unit", "legend") => include_bytes!(frame_key!("unit", "legend")) as &[u8],
        ("unit", "peculiar") => include_bytes!(frame_key!("unit", "peculiar")) as &[u8],
        ("spell", "brone") => include_bytes!(frame_key!("spell", "brone")) as &[u8],
        ("spell", "silver") => include_bytes!(frame_key!("spell", "silver")) as &[u8],
        ("spell", "gold") => include_bytes!(frame_key!("spell", "gold")) as &[u8],
        ("spell", "legend") => include_bytes!(frame_key!("spell", "legend")) as &[u8],
        ("spell", "peculiar") => include_bytes!(frame_key!("spell", "peculiar")) as &[u8],
        ("amulet", "brone") => include_bytes!(frame_key!("amulet", "brone")) as &[u8],
        ("amulet", "silver") => include_bytes!(frame_key!("amulet", "silver")) as &[u8],
        ("amulet", "gold") => include_bytes!(frame_key!("amulet", "gold")) as &[u8],
        ("amulet", "legend") => include_bytes!(frame_key!("amulet", "legend")) as &[u8],
        ("amulet", "peculiar") => include_bytes!(frame_key!("amulet", "peculiar")) as &[u8],
        _ => return None,
    };
    Some(bytes)
}

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

pub fn diy_jewel_bytes(cls: &str) -> Option<&'static [u8]> {
    let bytes = match cls {
        "neutral" => include_bytes!("../assets/diy/class/neutral.png") as &[u8],
        "forestcraft" => include_bytes!("../assets/diy/class/forestcraft.png") as &[u8],
        "swordcraft" => include_bytes!("../assets/diy/class/swordcraft.png") as &[u8],
        "runecraft" => include_bytes!("../assets/diy/class/runecraft.png") as &[u8],
        "dragoncraft" => include_bytes!("../assets/diy/class/dragoncraft.png") as &[u8],
        "abysscraft" => include_bytes!("../assets/diy/class/abysscraft.png") as &[u8],
        "havencraft" => include_bytes!("../assets/diy/class/havencraft.png") as &[u8],
        "portalcraft" => include_bytes!("../assets/diy/class/portalcraft.png") as &[u8],
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
        "cost_cover" => include_bytes!("../assets/diy/effect/cost_cover.png") as &[u8],
        "ap_cover" => include_bytes!("../assets/diy/effect/ap_cover.png") as &[u8],
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
        "default_crest" => include_bytes!("../assets/diy/crests/default_crest.png") as &[u8],
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

/// Stretch-blit `src` into the destination rect.
fn blit_stretch(canvas: &mut RgbaImage, src: &RgbaImage, dx: f32, dy: f32, dw: f32, dh: f32) {
    let dw = dw.round() as u32;
    let dh = dh.round() as u32;
    if dw == 0 || dh == 0 {
        return;
    }
    let resized = image::imageops::resize(
        src,
        dw,
        dh,
        image::imageops::FilterType::Lanczos3,
    );
    image::imageops::overlay(canvas, &resized, dx.round() as i64, dy.round() as i64);
}

fn frame_off(kind: &str) -> (f32, f32) {
    match kind {
        "spell" => (0.0, 12.9),
        "amulet" => (0.0, 2.2),
        _ => (0.0, 0.0),
    }
}

/// Resolve the crest icon image: user upload wins, then builtin index.
fn resolve_crest(
    spec: &str,
    upload: Option<&[u8]>,
) -> Result<Option<RgbaImage>, String> {
    if spec == "upload" {
        if let Some(bytes) = upload {
            return decode(bytes, "crest icon").map(Some);
        }
        return Ok(None);
    }
    if let Some(idx) = spec.strip_prefix("builtin:") {
        let n: usize = idx.parse().unwrap_or(0);
        if let Some(name) = CREST_BUILTIN.get(n) {
            if let Some(bytes) = diy_crest_bytes(name) {
                return decode(bytes, "crest icon").map(Some);
            }
        }
        return Ok(None);
    }
    // default: the luna crest
    if let Some(bytes) = diy_crest_bytes("default_crest") {
        return decode(bytes, "crest icon").map(Some);
    }
    Ok(None)
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
    let kind = diy_kind(config.kind);
    let rare = DIY_RARES
        .get((config.rarity.saturating_sub(1)) as usize)
        .copied()
        .ok_or_else(|| format!("bad rarity index {}", config.rarity))?;
    let is_unit = config.kind == KIND_FOLLOWER;
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

    // 3. card art (crop or cover into the art window)
    if let Some(bytes) = art_bytes {
        let art = decode(bytes, "art")?;
        match &config.crop {
            Some(crop) => blit_crop(&mut canvas, &art, ART_X, ART_Y, ART_W, ART_H, crop),
            None => blit_cover(&mut canvas, &art, ART_X, ART_Y, ART_W, ART_H),
        }
    } else {
        fill_rect(&mut canvas, ART_X, ART_Y, ART_W, ART_H, [42, 42, 46, 255]);
    }

    // 4. frame + gem covers + class jewel
    let frame_bytes = diy_frame_bytes(kind, rare).ok_or("missing frame asset")?;
    let frame = decode(frame_bytes, "frame")?;
    let (fx, fy) = frame_off(kind);
    image::imageops::overlay(&mut canvas, &frame, (CARD_X + fx).round() as i64, (CARD_Y + fy).round() as i64);
    if is_unit {
        let cost_cover = decode(diy_effect_bytes("cost_cover").unwrap(), "cost_cover")?;
        blit_stretch(&mut canvas, &cost_cover, CARD_X, CARD_Y, 552.0, 733.0);
        let ap_cover = decode(diy_effect_bytes("ap_cover").unwrap(), "ap_cover")?;
        blit_stretch(&mut canvas, &ap_cover, CARD_X, CARD_Y, 552.0, 733.0);
    }
    if let Some(jb) = diy_jewel_bytes(cls) {
        let jewel = decode(jb, "class jewel")?;
        blit_stretch(&mut canvas, &jewel, JEWEL_X, JEWEL_Y, JEWEL_W, JEWEL_H);
    }

    // 5. fonts
    let engine = TextEngine::for_language(&config.language)?;

    // 6. card name (white, no shadow) and numbers (colored glow + white core)
    if !config.name.is_empty() {
        engine.draw_label(
            &mut canvas,
            &engine.title,
            &config.name,
            NAME_CX,
            NAME_CY,
            NAME_SIZE + config.name_size_offset,
            900.0,
            0.0,
        );
    }
    if !config.cost.is_empty() {
        engine.draw_number_glow(
            &mut canvas,
            &config.cost,
            COST_CX,
            COST_CY,
            NUM_SIZE,
            150.0,
            [0, 57, 0, 255],
            0.33,
        );
    }
    if is_unit {
        if !config.atk.is_empty() {
            engine.draw_number_glow(
                &mut canvas,
                &config.atk,
                ATK_CX,
                ATK_CY,
                NUM_SIZE,
                240.0,
                [0, 29, 0, 255],
                0.77,
            );
        }
        if !config.life.is_empty() {
            engine.draw_number_glow(
                &mut canvas,
                &config.life,
                HP_CX,
                HP_CY,
                NUM_SIZE,
                240.0,
                [9, 56, 115, 255],
                0.44,
            );
        }
    }

    // 7. title band texts (drawn over the card)
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
    let (w, _) = engine.measure(&engine.title, &config.class_text, TITLE_SIDE_SIZE);
    engine.draw_plain(
        &mut canvas,
        &engine.title,
        &config.class_text,
        TITLE_CLASS_CX - w / 2.0,
        TITLE_CLASS_CY,
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
    let (w, _) = engine.measure(&engine.title, &config.trait_text, TITLE_SIDE_SIZE);
    engine.draw_plain(
        &mut canvas,
        &engine.title,
        &config.trait_text,
        TITLE_TYPE_CX - w / 2.0,
        TITLE_TYPE_CY,
        TITLE_SIDE_SIZE,
        crate::text::BODY,
        0.0,
    );

    // 8. description panel
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
    blit_stretch(&mut canvas, &detail_bg, DETAIL_X, DETAIL_Y, DETAIL_W, DETAIL_H);
    let spit_img = diy_effect_bytes("detail_spit").map(|b| decode(b, "detail_spit")).transpose()?;

    let mut y = VB_Y;
    let text_x = VB_X + TEXT_INSET;
    let text_w = VB_W - TEXT_INSET * 2.0;
    let d1 = &config.detail1;
    let d2 = &config.detail2;
    let ev = &config.evolve;
    let sup = &config.super_evolve;
    let cre = &config.crest;
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
    // crest
    if config.show_crest {
        let v = config.crest_scale.clamp(0.1, 1.5);
        let band_h = CREST_BAND_H * v;
        let text_h = measure_rich(&engine, cre, text_w, config.crest_size);
        let sec_h = CREST_BAND_DY + band_h + CREST_TEXT_GAP + text_h + CREST_BOTTOM;
        let banner_key = CREST_BORDERS
            .get(config.crest_border as usize)
            .copied()
            .unwrap_or("Crest");
        // section background: detail_crest.png fills the whole section rect
        let sec_banner = decode(
            diy_effect_bytes("detail_crest").unwrap(),
            "crest section banner",
        )?;
        blit_stretch(&mut canvas, &sec_banner, VB_X, y, VB_W, sec_h);
        // icon band: border texture stretched over the band rect
        let band_x = VB_X + CREST_BAND_DX;
        let band_y = y + CREST_BAND_DY;
        let band_banner = decode(diy_effect_bytes(banner_key).unwrap(), "crest band banner")?;
        blit_stretch(
            &mut canvas,
            &band_banner,
            band_x,
            band_y,
            CREST_BAND_W,
            band_h,
        );
        let icon_side = (CREST_ICON_SIDE * v).clamp(8.0, band_h);
        let mut ix = band_x + 32.0;
        let icon1 = resolve_crest(&config.crest_icon1, crest1_png)?;
        if let Some(ic) = icon1 {
            blit_stretch(
                &mut canvas,
                &ic,
                ix,
                band_y + (band_h - icon_side) / 2.0,
                icon_side,
                icon_side,
            );
            ix += icon_side + 4.0;
        }
        if config.show_crest_icon2 {
            let icon2 = resolve_crest(&config.crest_icon2, crest2_png)?;
            if let Some(ic) = icon2 {
                blit_stretch(
                    &mut canvas,
                    &ic,
                    ix,
                    band_y + (band_h - icon_side) / 2.0,
                    icon_side,
                    icon_side,
                );
                ix += icon_side + 4.0;
            }
        }
        if !config.crest_name.is_empty() {
            let name_size = config.crest_size;
            engine.draw_plain(
                &mut canvas,
                &engine.title,
                &config.crest_name,
                ix + 8.0,
                band_y + (band_h - name_size) / 2.0,
                name_size,
                crate::text::BODY,
                0.0,
            );
        }
        if !cre.trim().is_empty() {
            engine.draw_wrapped_rich(
                &mut canvas,
                &engine.title,
                cre,
                text_x,
                y + CREST_BAND_DY + band_h + CREST_TEXT_GAP,
                text_w,
                config.crest_size,
                LINE_GAP,
                PARA_GAP,
                spit_img.as_ref(),
            );
        }
    }

    // 9. signature rows
    if config.show_illustrator {
        engine.draw_plain(
            &mut canvas,
            &engine.title,
            "Illus. ",
            ILLU_TITLE_X,
            ILLU_TITLE_Y,
            DEFAULT_TEXT_SIZE,
            TITLE_GOLD,
            0.0,
        );
        engine.draw_plain(
            &mut canvas,
            &engine.title,
            &config.illustrator,
            ILLU_X,
            ILLU_TITLE_Y,
            DEFAULT_TEXT_SIZE,
            crate::text::BODY,
            0.0,
        );
    }
    if config.show_diy && !config.diy.is_empty() {
        let (w, _) = engine.measure(&engine.title, &config.diy, DIY_SIZE);
        engine.draw_plain(
            &mut canvas,
            &engine.title,
            &config.diy,
            (DIY_RIGHT - w).max(DIY_X),
            DIY_Y,
            DIY_SIZE,
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
            if cur_w + w > max_w && cur_w > 0.0 {
                lines += 1;
                cur_w = 0.0;
            }
            cur_w += w + sf.h_advance(sf.glyph_id(' '));
        }
    }
    let text_h = if text.trim().is_empty() {
        0.0
    } else {
        lines as f32 * line_h
    };
    text_h
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
    fn class_and_kind_mapping() {
        assert_eq!(DIY_CLASSES[0], "neutral");
        assert_eq!(DIY_CLASSES[7], "portalcraft");
        assert_eq!(DIY_CLASSES[5], "abysscraft");
        assert_eq!(diy_kind(1), "unit");
        assert_eq!(diy_kind(2), "amulet");
        assert_eq!(diy_kind(3), "spell");
        assert_eq!(DIY_RARES[4], "peculiar");
    }

    #[test]
    fn all_diy_assets_exist() {
        for kind in ["unit", "spell", "amulet"] {
            for rare in DIY_RARES {
                assert!(diy_frame_bytes(kind, rare).is_some(), "{kind}_{rare}");
            }
        }
        for cls in DIY_CLASSES {
            for gen in 1..=2 {
                assert!(diy_background_bytes(cls, gen).is_some(), "{cls}-{gen}");
            }
            assert!(diy_title_class_bytes(cls).is_some(), "title_class {cls}");
            assert!(diy_jewel_bytes(cls).is_some(), "jewel {cls}");
        }
        for key in [
            "title_bg",
            "title_bottom",
            "card_detail_background",
            "detail_spit",
            "cost_cover",
            "ap_cover",
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
}
