//! wbmaker — a web card maker for Shadowverse: Worlds Beyond.
//!
//! The rendering core compiles to WebAssembly. Fonts are registered at runtime
//! (loaded by the browser) to keep the wasm bundle small; `render_card` takes a
//! JSON config and optional PNG art bytes and returns a PNG of the card.

mod card;
mod render;
mod text;

use wasm_bindgen::prelude::*;

/// Register a font (OTF/TTF bytes) under a key. Keys:
///   * `title_<lang>`  — card-name font for a language (chs/cht/jpn/kor/eng)
///   * `body_<lang>`   — body/skill-text font for a language
///   * `number`        — Mincho number font (shared)
/// Returns true on success.
#[wasm_bindgen]
pub fn register_font(key: &str, bytes: &[u8]) -> bool {
    text::register_font(key, bytes)
}

/// Render a card. `config_json` is a `CardConfig` serialized as JSON,
/// `art_png` is the uploaded illustration as PNG bytes (may be empty).
/// Returns the composed card as PNG bytes.
#[wasm_bindgen]
pub fn render_card(config_json: &str, art_png: &[u8]) -> Result<Vec<u8>, JsValue> {
    let config: card::CardConfig = serde_json::from_str(config_json)
        .map_err(|e| JsValue::from_str(&format!("config parse error: {e}")))?;
    let art = if art_png.is_empty() {
        None
    } else {
        Some(art_png)
    };
    render::render(&config, art).map_err(|e| JsValue::from_str(&e))
}

/// All available frame keys (kind_rarity), for building the UI selector.
#[wasm_bindgen]
pub fn list_frames() -> String {
    let frames = [
        "follower_bronze",
        "follower_silver",
        "follower_gold",
        "follower_legend",
        "follower_high_premium",
        "follower_style_101",
        "follower_style_101_no_status",
        "spell_bronze",
        "spell_silver",
        "spell_gold",
        "spell_legend",
        "spell_style_101",
        "spell_style_101_no_status",
        "amulet_bronze",
        "amulet_silver",
        "amulet_gold",
        "amulet_legend",
        "amulet_style_101",
    ];
    serde_json::to_string(&frames).unwrap_or_else(|_| "[]".to_string())
}

#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Register a minimal font set from the repo's assets (native test only).
    fn register_test_fonts() {
        let fonts_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/fonts");
        let kaimin = std::fs::read(format!("{fonts_dir}/MOC-KaiminTsuki-B.otf")).unwrap();
        text::register_font("title_chs", &kaimin);
        text::register_font("number", &kaimin);
    }

    #[test]
    fn render_smoke() {
        register_test_fonts();
        let cfg = r#"{
            "name": "测试随从",
            "language": "chs",
            "class": 4,
            "kind": 1,
            "rarity": 4,
            "frame": "follower_legend",
            "cost": "7",
            "atk": "5",
            "life": "6",
            "detail1": "【守护】\n【入场曲】抽取1张卡牌。",
            "text_size": 24,
            "scale": 1.0
        }"#;
        let out = render_card(cfg, &[]).expect("render");
        assert!(out.len() > 1000);
        let img = image::load_from_memory(&out).expect("decode png");
        assert_eq!((img.width(), img.height()), (782, 1024));
    }

    /// Regression test: the number glow must not composite as a solid white
    /// block. The scanline rasterizer leaves a faint residual coverage across
    /// the glyph box; `draw_number` clips it, so the white body should cover
    /// well under half of its bounding box.
    #[test]
    fn number_not_solid_block() {
        let fonts_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/fonts");
        let tsukushi = std::fs::read(format!("{fonts_dir}/FOT-TsukuAOldMin-Pr6-E.digits.otf")).unwrap();
        let nanum = std::fs::read(format!("{fonts_dir}/NanumGothic-ExtraBold.ttf")).unwrap();
        text::register_font("title_chs", &nanum);
        text::register_font("number", &tsukushi);
        let engine = text::TextEngine::for_language("chs").unwrap();
        let mut canvas = image::RgbaImage::from_pixel(200, 200, image::Rgba([60, 60, 60, 255]));
        engine.draw_number(&mut canvas, "7", 100.0, 100.0, 106.0, 200.0, 0.5, 0.0);
        let mut white = 0u32;
        let (mut minx, mut miny, mut maxx, mut maxy) = (u32::MAX, u32::MAX, 0u32, 0u32);
        for (x, y, p) in canvas.enumerate_pixels() {
            if p[0] > 200 && p[1] > 200 && p[2] > 200 {
                white += 1;
                minx = minx.min(x);
                miny = miny.min(y);
                maxx = maxx.max(x);
                maxy = maxy.max(y);
            }
        }
        let w = i64::from(maxx.saturating_sub(minx)) + 1;
        let h = i64::from(maxy.saturating_sub(miny)) + 1;
        assert!(
            i64::from(white) < (w * h) / 2,
            "number is a solid white block: {white} white in {w}x{h}"
        );
    }
}
