//! Text rendering engine built on `ab_glyph`.
//!
//! Fonts are loaded at runtime (registered from JS) so the WebAssembly bundle
//! stays small; each language uses its own title font (for the card name) plus
//! a shared number font, matching the game's fonts extracted from `data.unity3d`.

use ab_glyph::{Font, FontArc, GlyphId, PxScale, ScaleFont, point};
use image::{Rgba, RgbaImage};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

pub const BODY: [u8; 4] = [255, 255, 255, 255];
pub const SHADOW: [u8; 4] = [0, 0, 0, 210];

/// Minimum rasterizer coverage we accept as "covered".
///
/// `ab_glyph_rasterizer` (font-rs scanline rasterizer) leaves a faint residual
/// winding-number coverage (~0.0..0.125) across almost the whole glyph bounding
/// box for many CJK/display fonts. That residual is not real ink; the actual
/// strokes land at coverage >= ~0.5. Clipping below this threshold keeps the
/// faint fill from being composited (it would otherwise turn a number into a
/// solid block once blurred and re-composited white).
const COVERAGE_EPSILON: f32 = 0.15;

/// Per-pair kerning for the number font (筑紫明朝), hand-tuned at size 125.
///
/// These digit pairs sit slightly tighter than the default; every other pair
/// uses zero. The value is scaled with the rendered size so it stays correct
/// when the number size or the output scale changes.
fn number_pair_kern(prev: char, cur: char, size: f32) -> f32 {
    const TIGHT: f32 = -12.0;
    const REF_SIZE: f32 = 125.0;
    let base = match (prev, cur) {
        ('0', '1')
        | ('1', '0')
        | ('1', '1')
        | ('1', '4')
        | ('1', '5')
        | ('1', '8')
        | ('1', '9')
        | ('2', '1')
        | ('3', '1')
        | ('4', '1')
        | ('5', '1')
        | ('6', '1')
        | ('7', '1')
        | ('8', '1')
        | ('9', '1') => TIGHT,
        _ => 0.0,
    };
    base * (size / REF_SIZE)
}

/// Global font registry, filled via `register_font` from JS.
static FONTS: OnceLock<Mutex<HashMap<String, FontArc>>> = OnceLock::new();

pub fn register_font(key: &str, bytes: &[u8]) -> bool {
    match FontArc::try_from_vec(bytes.to_vec()) {
        Ok(font) => {
            FONTS
                .get_or_init(|| Mutex::new(HashMap::new()))
                .lock()
                .unwrap()
                .insert(key.to_string(), font);
            true
        }
        Err(_) => false,
    }
}

fn get_font(key: &str) -> Option<FontArc> {
    FONTS.get()?.lock().unwrap().get(key).cloned()
}

pub struct TextEngine {
    /// Card name / title font (per language).
    pub title: FontArc,
    /// Number font (Mincho, closest vector to the game's 筑紫明朝).
    pub number: FontArc,
}

impl TextEngine {
    /// Build the engine for a language. Expects `title_<lang>` and `number` to
    /// have been registered.
    pub fn for_language(lang: &str) -> Result<Self, String> {
        let title = get_font(&format!("title_{lang}"))
            .ok_or_else(|| format!("title_{lang} font not registered"))?;
        let number = get_font("number").ok_or_else(|| "number font not registered".to_string())?;
        Ok(TextEngine { title, number })
    }

    fn scaled<'a>(font: &'a FontArc, size: f32) -> ab_glyph::PxScaleFont<&'a FontArc> {
        font.as_scaled(PxScale::from(size))
    }

    /// Advance width and line height of a string at `size` px.
    pub fn measure(&self, primary: &FontArc, text: &str, size: f32) -> (f32, f32) {
        let sf = Self::scaled(primary, size);
        let mut w = 0.0f32;
        let mut prev: Option<GlyphId> = None;
        for ch in text.chars() {
            let gid = sf.glyph_id(ch);
            if let Some(p) = prev {
                w += sf.kern(p, gid);
            }
            w += sf.h_advance(gid);
            prev = Some(gid);
        }
        (w, size)
    }

    fn draw_glyph(
        &self,
        img: &mut RgbaImage,
        font: &FontArc,
        gid: GlyphId,
        size: f32,
        x: f32,
        baseline_y: f32,
        color: [u8; 4],
    ) {
        let glyph = gid.with_scale_and_position(PxScale::from(size), point(x, baseline_y));
        if let Some(outlined) = font.outline_glyph(glyph) {
            let b = outlined.px_bounds();
            let ox = b.min.x.round() as i32;
            let oy = b.min.y.round() as i32;
            outlined.draw(|gx, gy, cov| {
                let px = ox + gx as i32;
                let py = oy + gy as i32;
                if px < 0 || py < 0 {
                    return;
                }
                let (px, py) = (px as u32, py as u32);
                if px >= img.width() || py >= img.height() {
                    return;
                }
                let a = cov.clamp(0.0, 1.0);
                if a < COVERAGE_EPSILON {
                    return;
                }
                blend_pixel(img, px, py, color, a);
            });
        }
    }

    /// Rasterize a glyph into `img` as a straight-alpha white glyph: RGB is
    /// kept white and coverage is written into the alpha channel. This is used
    /// for the number buffer, whose alpha is later blurred and re-tinted by
    /// `composite_tint` (which reads only alpha + `color`). Unlike
    /// `blend_pixel`, this does not bake coverage into RGB and force alpha to
    /// 255, so faint residual coverage stays faint instead of going opaque.
    fn draw_glyph_alpha(
        &self,
        img: &mut RgbaImage,
        font: &FontArc,
        gid: GlyphId,
        size: f32,
        x: f32,
        baseline_y: f32,
    ) {
        let glyph = gid.with_scale_and_position(PxScale::from(size), point(x, baseline_y));
        if let Some(outlined) = font.outline_glyph(glyph) {
            let b = outlined.px_bounds();
            let ox = b.min.x.round() as i32;
            let oy = b.min.y.round() as i32;
            outlined.draw(|gx, gy, cov| {
                let px = ox + gx as i32;
                let py = oy + gy as i32;
                if px < 0 || py < 0 {
                    return;
                }
                let (px, py) = (px as u32, py as u32);
                if px >= img.width() || py >= img.height() {
                    return;
                }
                let a = cov.clamp(0.0, 1.0);
                if a < COVERAGE_EPSILON {
                    return;
                }
                let alpha = (a * 255.0).round() as u8;
                let d = img.get_pixel_mut(px, py);
                // Take the max alpha in case overlapping strokes share a pixel.
                if alpha > d[3] {
                    d[0] = 255;
                    d[1] = 255;
                    d[2] = 255;
                    d[3] = alpha;
                }
            });
        }
    }

    fn draw_text_at(
        &self,
        img: &mut RgbaImage,
        primary: &FontArc,
        text: &str,
        x: f32,
        baseline_y: f32,
        size: f32,
        color: [u8; 4],
    ) {
        let sf = Self::scaled(primary, size);
        let mut cx = x;
        let mut prev: Option<GlyphId> = None;
        for ch in text.chars() {
            let gid = sf.glyph_id(ch);
            if let Some(p) = prev {
                cx += sf.kern(p, gid);
            }
            self.draw_glyph(img, primary, gid, size, cx, baseline_y, color);
            cx += sf.h_advance(gid);
            prev = Some(gid);
        }
    }

    /// Plain left-aligned text with shadow, top-left origin.
    pub fn draw_plain(
        &self,
        img: &mut RgbaImage,
        primary: &FontArc,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        color: [u8; 4],
        shadow_off: f32,
    ) {
        let baseline = y + Self::scaled(primary, size).ascent();
        if shadow_off > 0.0 {
            let off = shadow_off.max(1.0);
            for (dx, dy) in [(-off, 0.0), (off, 0.0), (0.0, -off), (0.0, off), (off, off)] {
                self.draw_text_at(img, primary, text, x + dx, baseline + dy, size, SHADOW);
            }
        }
        self.draw_text_at(img, primary, text, x, baseline, size, color);
    }

    /// Centered text with shadow, replicating wbunpacker's label drawing.
    /// Returns the (possibly shrunk) font size actually used.
    pub fn draw_label(
        &self,
        img: &mut RgbaImage,
        primary: &FontArc,
        text: &str,
        center_x: f32,
        center_y: f32,
        mut size: f32,
        max_width: f32,
        shadow_off: f32,
    ) -> f32 {
        let mut w = self.measure(primary, text, size).0;
        while size > 24.0 && w > max_width {
            size -= 2.0;
            w = self.measure(primary, text, size).0;
        }
        let adjusted_cx = center_x - ((max_width - w) * 0.08).max(0.0);
        let h = self.measure(primary, text, size).1;
        let x = adjusted_cx - w / 2.0;
        let y = center_y - h / 2.0;
        self.draw_plain(img, primary, text, x, y, size, BODY, shadow_off);
        size
    }

    /// Centered number with a smooth glow shadow (proper blur, no mesh) and a
    /// white number on top, using the Mincho number font (筑紫明朝).
    /// `shadow` is the glow strength (>= 0; beyond 1.0 the glow widens instead
    /// of getting more opaque); `spacing` is extra uniform letter spacing.
    pub fn draw_number(
        &self,
        img: &mut RgbaImage,
        text: &str,
        center_x: f32,
        center_y: f32,
        mut size: f32,
        max_width: f32,
        shadow: f32,
        spacing: f32,
    ) -> f32 {
        let font = &self.number;
        let mut w = self.measure_spaced(font, text, size, spacing).0;
        while size > 24.0 && w > max_width {
            size -= 2.0;
            w = self.measure_spaced(font, text, size, spacing).0;
        }

        // Opacity caps at full strength; past 1.0 the blur radius grows so a
        // stronger glow spreads further instead of just saturating.
        let strength = shadow.max(0.0);
        let sa = (strength.min(1.0) * 255.0) as u8;
        // Cap the radius so an absurd shadow value can't blow up the blur cost.
        let radius = (2.0 + (strength - 1.0).max(0.0) * 2.0)
            .round()
            .max(1.0)
            .min(12.0) as u32;
        let pad = if sa > 0 {
            (size * 0.07).ceil() as u32 + 1 + radius * 2
        } else {
            1
        };
        let buf = self.render_number_buffer(text, size, spacing, pad);
        let (bw, bh) = buf.dimensions();
        let dx = center_x - bw as f32 / 2.0;
        let dy = center_y - bh as f32 / 2.0;

        if sa > 0 {
            // Two box-blur passes approximate a Gaussian; smooth, no mesh.
            let mut blurred = box_blur_alpha(&buf, radius);
            blurred = box_blur_alpha(&blurred, radius);
            composite_tint(img, &blurred, dx, dy, [0, 0, 0, sa]);
        }
        composite_tint(img, &buf, dx, dy, BODY);
        size
    }

    /// Measure width/height with extra letter spacing and per-pair kerning.
    fn measure_spaced(&self, font: &FontArc, text: &str, size: f32, spacing: f32) -> (f32, f32) {
        let sf = Self::scaled(font, size);
        let mut w = 0.0f32;
        let mut prev: Option<(GlyphId, char)> = None;
        for ch in text.chars() {
            let gid = sf.glyph_id(ch);
            if let Some((p, pc)) = prev {
                w += sf.kern(p, gid);
                w += spacing + number_pair_kern(pc, ch, size);
            }
            w += sf.h_advance(gid);
            prev = Some((gid, ch));
        }
        (w, size)
    }

    /// Render the number text (white) into a new buffer with padding and
    /// letter spacing, using the number font.
    fn render_number_buffer(&self, text: &str, size: f32, spacing: f32, pad: u32) -> RgbaImage {
        let font = &self.number;
        let sf = Self::scaled(font, size);
        let ascent = sf.ascent();
        let descent = -sf.descent();
        let (w, _) = self.measure_spaced(font, text, size, spacing);
        let bw = (w.ceil() as u32).max(1) + pad * 2;
        let bh = ((ascent + descent).ceil() as u32).max(1) + pad * 2;
        let mut buf = RgbaImage::new(bw, bh);
        let baseline = pad as f32 + ascent;
        let mut cx = pad as f32;
        let mut prev: Option<(GlyphId, char)> = None;
        let chars: Vec<char> = text.chars().collect();
        for &ch in &chars {
            let gid = sf.glyph_id(ch);
            if let Some((p, pc)) = prev {
                cx += sf.kern(p, gid);
                cx += spacing + number_pair_kern(pc, ch, size);
            }
            self.draw_glyph_alpha(&mut buf, font, gid, size, cx, baseline);
            cx += sf.h_advance(gid);
            prev = Some((gid, ch));
        }
        buf
    }

}

/// Alpha-blend a single color onto an RGBA pixel.
#[inline]
fn blend_pixel(img: &mut RgbaImage, x: u32, y: u32, color: [u8; 4], a: f32) {
    let d = img.get_pixel_mut(x, y);
    let sa = a * (color[3] as f32 / 255.0);
    if sa <= 0.0 {
        return;
    }
    if sa >= 1.0 {
        d[0] = color[0];
        d[1] = color[1];
        d[2] = color[2];
        d[3] = 255;
        return;
    }
    let da = 1.0 - sa;
    d[0] = (color[0] as f32 * sa + d[0] as f32 * da) as u8;
    d[1] = (color[1] as f32 * sa + d[1] as f32 * da) as u8;
    d[2] = (color[2] as f32 * sa + d[2] as f32 * da) as u8;
    d[3] = 255;
}

/// 2D box blur on the alpha channel (RGB kept white). Two passes approximate a
/// Gaussian, giving a smooth glow without the "mesh" artifact of offset copies.
fn box_blur_alpha(img: &RgbaImage, radius: u32) -> RgbaImage {
    let (w, h) = img.dimensions();
    let mut out = RgbaImage::new(w, h);
    let r = radius as i64;
    for y in 0..h {
        for x in 0..w {
            let mut sum = 0u64;
            let mut n = 0u64;
            for dy in -r..=r {
                let py = y as i64 + dy;
                if py < 0 || py >= h as i64 {
                    continue;
                }
                for dx in -r..=r {
                    let px = x as i64 + dx;
                    if px < 0 || px >= w as i64 {
                        continue;
                    }
                    sum += img.get_pixel(px as u32, py as u32)[3] as u64;
                    n += 1;
                }
            }
            out.put_pixel(x, y, Rgba([255, 255, 255, (sum / n) as u8]));
        }
    }
    out
}

/// Alpha-blend a white glyph buffer tinted with `color` onto the canvas.
fn composite_tint(
    canvas: &mut RgbaImage,
    glyph: &RgbaImage,
    dx: f32,
    dy: f32,
    color: [u8; 4],
) {
    let ox = dx.round() as i64;
    let oy = dy.round() as i64;
    for (x, y, p) in glyph.enumerate_pixels() {
        let a = p[3] as f32 / 255.0;
        if a <= 0.0 {
            continue;
        }
        let px = ox + x as i64;
        let py = oy + y as i64;
        if px < 0 || py < 0 || px >= canvas.width() as i64 || py >= canvas.height() as i64 {
            continue;
        }
        let d = canvas.get_pixel_mut(px as u32, py as u32);
        let sa = a * (color[3] as f32 / 255.0);
        let da = 1.0 - sa;
        d[0] = (color[0] as f32 * sa + d[0] as f32 * da) as u8;
        d[1] = (color[1] as f32 * sa + d[1] as f32 * da) as u8;
        d[2] = (color[2] as f32 * sa + d[2] as f32 * da) as u8;
        d[3] = 255;
    }
}

#[cfg(test)]
mod tests {
    use super::number_pair_kern;

    #[test]
    fn number_pair_kerning_table() {
        let tight = ["01", "10", "11", "14", "15", "18", "19", "21", "31", "41", "51", "61", "71", "81", "91"];
        for pair in tight {
            let mut c = pair.chars();
            let (a, b) = (c.next().unwrap(), c.next().unwrap());
            assert_eq!(number_pair_kern(a, b, 125.0), -12.0, "pair {pair}");
        }
        // A pair that is not in the table stays at zero.
        assert_eq!(number_pair_kern('1', '2', 125.0), 0.0);
        assert_eq!(number_pair_kern('2', '2', 125.0), 0.0);
        // The tuned value is relative to size 125 and scales proportionally.
        assert_eq!(number_pair_kern('0', '1', 250.0), -24.0);
    }
}
