//! Full-card compositing pipeline (pure Rust, target-independent).
//!
//! Layout follows wbunpacker's authoritative `config/render.toml`
//! (782x1024 design space), which is also what WBArts's "game" card style uses.

use crate::card::{CardConfig, KIND_FOLLOWER};
use crate::text::TextEngine;
use image::{DynamicImage, GenericImageView, ImageFormat, RgbaImage};

// ---- design-space layout (782 x 1024) ----
pub const BASE_W: f32 = 782.0;
pub const BASE_H: f32 = 1024.0;

const ART_X: f32 = 98.0;
const ART_Y: f32 = 194.0;
const ART_W: f32 = 590.0;
const ART_H: f32 = 711.0;

const ICON_X: f32 = 366.0;
const ICON_Y: f32 = 878.0;
const ICON_W: f32 = 50.0;
const ICON_H: f32 = 50.0;

const NAME_CX: f32 = 420.0;
const NAME_CY: f32 = 148.0;
const NAME_MAXW: f32 = 450.0;

const COST_CX: f32 = 93.0;
const COST_CY: f32 = 137.0;

const ATK_CX: f32 = 92.0;
const ATK_CY: f32 = 920.0;
const DEF_CX: f32 = 698.0;
const DEF_CY: f32 = 918.0;

/// Resolve a frame asset key to its embedded PNG bytes.
pub fn frame_bytes(key: &str) -> Option<&'static [u8]> {
    let bytes = match key {
        "follower_bronze" => include_bytes!("../assets/frames/frame2d_follower_bronze.png") as &[u8],
        "follower_silver" => include_bytes!("../assets/frames/frame2d_follower_silver.png") as &[u8],
        "follower_gold" => include_bytes!("../assets/frames/frame2d_follower_gold.png") as &[u8],
        "follower_legend" => include_bytes!("../assets/frames/frame2d_follower_legend.png") as &[u8],
        "follower_high_premium" => {
            include_bytes!("../assets/frames/frame2d_follower_high_premium.png") as &[u8]
        }
        "follower_style_101" => include_bytes!("../assets/frames/frame2d_follower_style_101.png") as &[u8],
        "follower_style_101_no_status" => {
            include_bytes!("../assets/frames/frame2d_follower_style_101_no_status.png") as &[u8]
        }
        "spell_bronze" => include_bytes!("../assets/frames/frame2d_spell_bronze.png") as &[u8],
        "spell_silver" => include_bytes!("../assets/frames/frame2d_spell_silver.png") as &[u8],
        "spell_gold" => include_bytes!("../assets/frames/frame2d_spell_gold.png") as &[u8],
        "spell_legend" => include_bytes!("../assets/frames/frame2d_spell_legend.png") as &[u8],
        "spell_style_101" => include_bytes!("../assets/frames/frame2d_spell_style_101.png") as &[u8],
        "spell_style_101_no_status" => {
            include_bytes!("../assets/frames/frame2d_spell_style_101_no_status.png") as &[u8]
        }
        "amulet_bronze" => include_bytes!("../assets/frames/frame2d_amulet_bronze.png") as &[u8],
        "amulet_silver" => include_bytes!("../assets/frames/frame2d_amulet_silver.png") as &[u8],
        "amulet_gold" => include_bytes!("../assets/frames/frame2d_amulet_gold.png") as &[u8],
        "amulet_legend" => include_bytes!("../assets/frames/frame2d_amulet_legend.png") as &[u8],
        "amulet_style_101" => include_bytes!("../assets/frames/frame2d_amulet_style_101.png") as &[u8],
        _ => return None,
    };
    Some(bytes)
}

pub fn icon_bytes(class: u8) -> Option<&'static [u8]> {
    let bytes = match class {
        0 => include_bytes!("../assets/icons/card2d_class_icon_0.png") as &[u8],
        1 => include_bytes!("../assets/icons/card2d_class_icon_1.png") as &[u8],
        2 => include_bytes!("../assets/icons/card2d_class_icon_2.png") as &[u8],
        3 => include_bytes!("../assets/icons/card2d_class_icon_3.png") as &[u8],
        4 => include_bytes!("../assets/icons/card2d_class_icon_4.png") as &[u8],
        5 => include_bytes!("../assets/icons/card2d_class_icon_5.png") as &[u8],
        6 => include_bytes!("../assets/icons/card2d_class_icon_6.png") as &[u8],
        7 => include_bytes!("../assets/icons/card2d_class_icon_7.png") as &[u8],
        _ => return None,
    };
    Some(bytes)
}

pub fn render(config: &CardConfig, art_bytes: Option<&[u8]>) -> Result<Vec<u8>, String> {
    let s = config.scale.clamp(0.5, 4.0);
    let cw = (BASE_W * s).round() as u32;
    let ch = (BASE_H * s).round() as u32;
    let mut canvas = RgbaImage::new(cw, ch);

    // 1. art (cover-fit into the art window)
    if let Some(bytes) = art_bytes {
        let art = image::load_from_memory(bytes)
            .map_err(|e| format!("art decode failed: {e}"))?
            .to_rgba8();
        blit_cover(&mut canvas, &art, ART_X * s, ART_Y * s, ART_W * s, ART_H * s);
    } else {
        fill_rect(
            &mut canvas,
            ART_X * s,
            ART_Y * s,
            ART_W * s,
            ART_H * s,
            [42, 42, 46, 255],
        );
    }

    // 2. frame (full canvas)
    let frame_key = config.frame.as_str();
    let frame = frame_bytes(frame_key)
        .ok_or_else(|| format!("unknown frame: {frame_key}"))?;
    let frame_img = image::load_from_memory(frame)
        .map_err(|e| format!("frame decode failed: {e}"))?
        .to_rgba8();
    let frame_rs = resize_lanczos(&frame_img, cw, ch);
    image::imageops::overlay(&mut canvas, &frame_rs, 0, 0);

    // 3. class icon
    if let Some(icon) = icon_bytes(config.class) {
        let icon_img = image::load_from_memory(icon)
            .map_err(|e| format!("icon decode failed: {e}"))?
            .to_rgba8();
        let iw = (ICON_W * s).round() as u32;
        let ih = (ICON_H * s).round() as u32;
        let icon_rs = resize_lanczos(&icon_img, iw, ih);
        image::imageops::overlay(
            &mut canvas,
            &icon_rs,
            (ICON_X * s).round() as i64,
            (ICON_Y * s).round() as i64,
        );
    }

    // 4. name + cost / atk / def (per-language fonts)
    let engine = TextEngine::for_language(&config.language)?;
    // Card name is drawn with no shadow/outline (no 黑边).
    if !config.name.is_empty() {
        engine.draw_label(
            &mut canvas,
            &engine.title,
            &config.name,
            NAME_CX * s,
            NAME_CY * s,
            config.name_size * s,
            NAME_MAXW * s,
            0.0,
        );
    }
    // Numbers use the 筑紫明朝 vector font directly, with a smooth glow shadow.
    let number_size = config.number_size * s;
    let nshadow = config.number_shadow;
    let spacing = config.number_spacing * s;
    if !config.cost.is_empty() {
        engine.draw_number(
            &mut canvas,
            &config.cost,
            COST_CX * s + config.cost_dx * s,
            COST_CY * s + config.cost_dy * s,
            number_size,
            number_size * 2.0,
            nshadow,
            spacing,
        );
    }
    if config.kind == KIND_FOLLOWER {
        if !config.atk.is_empty() {
            engine.draw_number(
                &mut canvas,
                &config.atk,
                ATK_CX * s + config.atk_dx * s,
                ATK_CY * s + config.atk_dy * s,
                number_size,
                number_size * 2.0,
                nshadow,
                spacing,
            );
        }
        if !config.life.is_empty() {
            engine.draw_number(
                &mut canvas,
                &config.life,
                DEF_CX * s + config.def_dx * s,
                DEF_CY * s + config.def_dy * s,
                number_size,
                number_size * 2.0,
                nshadow,
                spacing,
            );
        }
    }

    // 5. encode PNG
    let mut out = Vec::new();
    DynamicImage::ImageRgba8(canvas)
        .write_to(&mut std::io::Cursor::new(&mut out), ImageFormat::Png)
        .map_err(|e| format!("png encode failed: {e}"))?;
    Ok(out)
}

// ---- image helpers ----

fn resize_lanczos(img: &RgbaImage, w: u32, h: u32) -> RgbaImage {
    DynamicImage::ImageRgba8(img.clone())
        .resize_exact(w, h, image::imageops::FilterType::Lanczos3)
        .to_rgba8()
}

/// Cover-fit `src` into the destination rectangle (scale to fill, crop, center).
fn blit_cover(canvas: &mut RgbaImage, src: &RgbaImage, dx: f32, dy: f32, dw: f32, dh: f32) {
    let (sw, sh) = src.dimensions();
    if sw == 0 || sh == 0 {
        return;
    }
    let dw = dw.round() as u32;
    let dh = dh.round() as u32;
    let scale = (dw as f32 / sw as f32).max(dh as f32 / sh as f32);
    let nw = (sw as f32 * scale).round() as u32;
    let nh = (sh as f32 * scale).round() as u32;
    let resized = resize_lanczos(src, nw, nh);
    let cx = (nw.saturating_sub(dw)) / 2;
    let cy = (nh.saturating_sub(dh)) / 2;
    let cropped = resized.view(cx, cy, dw, dh).to_image();
    image::imageops::overlay(canvas, &cropped, dx.round() as i64, dy.round() as i64);
}

/// Alpha-blend a filled rectangle.
fn fill_rect(img: &mut RgbaImage, x: f32, y: f32, w: f32, h: f32, color: [u8; 4]) {
    let x0 = x.round() as i64;
    let y0 = y.round() as i64;
    let x1 = (x + w).round() as i64;
    let y1 = (y + h).round() as i64;
    let a = color[3] as f32 / 255.0;
    if a <= 0.0 {
        return;
    }
    for py in y0.max(0)..y1.min(img.height() as i64) {
        for px in x0.max(0)..x1.min(img.width() as i64) {
            let p = img.get_pixel_mut(px as u32, py as u32);
            let da = 1.0 - a;
            p[0] = (color[0] as f32 * a + p[0] as f32 * da) as u8;
            p[1] = (color[1] as f32 * a + p[1] as f32 * da) as u8;
            p[2] = (color[2] as f32 * a + p[2] as f32 * da) as u8;
            p[3] = 255;
        }
    }
}
