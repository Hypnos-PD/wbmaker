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
/// DIY keyword gold ([b]…[/b]), same tone as the BYD-DIY tool (#ffd676).
pub const KEYWORD_GOLD: [u8; 4] = [255, 214, 118, 255];
/// DIY bracket-keyword yellow (【…】『…』, [color=yellow]).
pub const KEYWORD_YELLOW: [u8; 4] = [255, 255, 0, 255];

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

    /// Draw one line of styled runs (no wrapping). `y` is the text top.
    pub fn draw_rich_line(
        &self,
        img: &mut RgbaImage,
        font: &FontArc,
        runs: &[RichRun],
        x: f32,
        y: f32,
        size: f32,
    ) -> f32 {
        let baseline = y + Self::scaled(font, size).ascent();
        let mut cx = x;
        let sf = Self::scaled(font, size);
        let mut prev: Option<GlyphId> = None;
        for run in runs {
            for ch in run.text.chars() {
                let gid = sf.glyph_id(ch);
                if let Some(p) = prev {
                    cx += sf.kern(p, gid);
                }
                if run.italic {
                    self.draw_glyph_italic(img, font, gid, size, cx, baseline, run.color);
                } else {
                    self.draw_glyph(img, font, gid, size, cx, baseline, run.color);
                }
                cx += sf.h_advance(gid);
                prev = Some(gid);
            }
        }
        cx - x
    }

    /// Rasterize a glyph into a straight-alpha buffer (white, coverage in alpha).
    fn rasterize_alpha(
        &self,
        font: &FontArc,
        gid: GlyphId,
        size: f32,
        x: f32,
        baseline_y: f32,
        buf: &mut RgbaImage,
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
                if px >= buf.width() || py >= buf.height() {
                    return;
                }
                let a = cov.clamp(0.0, 1.0);
                if a < COVERAGE_EPSILON {
                    return;
                }
                let alpha = (a * 255.0).round() as u8;
                let d = buf.get_pixel_mut(px, py);
                if alpha > d[3] {
                    d[0] = 255;
                    d[1] = 255;
                    d[2] = 255;
                    d[3] = alpha;
                }
            });
        }
    }

    /// Fake-italic glyph: rasterize upright, then shear rows and blend.
    fn draw_glyph_italic(
        &self,
        img: &mut RgbaImage,
        font: &FontArc,
        gid: GlyphId,
        size: f32,
        x: f32,
        baseline_y: f32,
        color: [u8; 4],
    ) {
        let glyph = gid.with_scale_and_position(PxScale::from(size), point(0.0, 0.0));
        let Some(outlined) = font.outline_glyph(glyph) else { return };
        let b = outlined.px_bounds();
        if b.width() <= 0.0 || b.height() <= 0.0 {
            return;
        }
        const SLANT: f32 = 0.22;
        let pad = (size * 0.1).ceil() as u32 + 2;
        let bw = b.width().ceil() as u32 + pad * 2;
        let bh = b.height().ceil() as u32 + pad * 2;
        let mut buf = RgbaImage::new(bw, bh);
        let bx = pad as f32 - b.min.x;
        let by = pad as f32 - b.min.y;
        self.rasterize_alpha(font, gid, size, bx, by, &mut buf);

        let dx = x + b.min.x - pad as f32;
        let dy = baseline_y + b.min.y - pad as f32;
        let ox = dx.round() as i64;
        let oy = dy.round() as i64;
        let mid = bh as f32 / 2.0;
        for (px, py, p) in buf.enumerate_pixels() {
            let a = p[3] as f32 / 255.0;
            if a <= 0.0 {
                continue;
            }
            let shear = (py as f32 - mid) * SLANT;
            let tx = (px as f32 + shear).round() as i64;
            let txx = ox + tx;
            let tyy = oy + py as i64;
            if txx < 0 || tyy < 0 || txx >= img.width() as i64 || tyy >= img.height() as i64 {
                continue;
            }
            blend_over(img, txx as u32, tyy as u32, color, a * (color[3] as f32 / 255.0));
        }
    }

    /// Wrap and draw rich text (DIY card description). Returns the total height
    /// used. `spit` is the section-split line drawn for `[img]…[/img]` runs.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_wrapped_rich(
        &self,
        img: &mut RgbaImage,
        font: &FontArc,
        text: &str,
        x: f32,
        y: f32,
        max_w: f32,
        size: f32,
        line_gap: f32,
        para_gap: f32,
        spit: Option<&RgbaImage>,
    ) -> f32 {
        let runs = parse_rich(text);
        let sf = Self::scaled(font, size);
        let line_h = sf.height() + line_gap;
        let mut cursor_y = y;

        // Tokenize into items: (text, color, italic, is_split, is_newline).
        // A word keeps its trailing space so inter-word gaps survive wrapping.
        let mut items: Vec<(String, [u8; 4], bool, bool, bool)> = Vec::new();
        for run in &runs {
            if run.split {
                items.push((String::new(), BODY, false, true, false));
                continue;
            }
            let mut j = 0;
            while j < run.text.len() {
                let ch = run.text[j..].chars().next().unwrap();
                if ch == '\n' {
                    items.push((String::new(), run.color, run.italic, false, true));
                    j += 1;
                    continue;
                }
                if ch == ' ' {
                    j += 1; // 行首/多余空格跳过（词间空格随单词携带）
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
                let mut word = run.text[start..stop].to_string();
                // 拉丁词携带一个尾随空格（若存在）并消费它，避免重复/丢空格
                if stop < run.text.len() && run.text[stop..].starts_with(' ') {
                    word.push(' ');
                    j = stop + 1;
                }
                items.push((word, run.color, run.italic, false, false));
            }
        }

        let measure_word = |word: &str, color: [u8; 4], italic: bool| -> f32 {
            let run = RichRun { text: word.to_string(), color, italic, split: false };
            let mut w = 0.0f32;
            let mut prev: Option<GlyphId> = None;
            for ch in run.text.chars() {
                let gid = sf.glyph_id(ch);
                if let Some(p) = prev {
                    w += sf.kern(p, gid);
                }
                w += sf.h_advance(gid);
                prev = Some(gid);
            }
            w
        };

        let mut cur: Vec<RichRun> = Vec::new();
        let mut cur_w: f32 = 0.0;

        for (word, color, italic, split, nl) in items {
            if split {
                if self.flush_rich_line(img, font, &mut cur, &mut cur_w, x, cursor_y, size) {
                    cursor_y += line_h;
                }
                if let Some(spit_img) = spit {
                    let sh = (spit_img.height() as f32).max(2.0);
                    for (px, py, p) in spit_img.enumerate_pixels() {
                        if p[3] == 0 {
                            continue;
                        }
                        let tx = x + px as f32 * (max_w / spit_img.width() as f32);
                        let ty = cursor_y + py as f32 * (sh / spit_img.height() as f32);
                        let txx = tx.round() as i64;
                        let tyy = ty.round() as i64;
                        if txx >= 0
                            && tyy >= 0
                            && (txx as u32) < img.width()
                            && (tyy as u32) < img.height()
                        {
                            blend_over(
                                img,
                                txx as u32,
                                tyy as u32,
                                [p[0], p[1], p[2], p[3]],
                                p[3] as f32 / 255.0,
                            );
                        }
                    }
                    cursor_y += sh + line_gap;
                }
                continue;
            }
            if nl {
                if self.flush_rich_line(img, font, &mut cur, &mut cur_w, x, cursor_y, size) {
                    cursor_y += line_h;
                }
                cursor_y += para_gap;
                continue;
            }
            let w = measure_word(&word, color, italic);
            if cur_w + w > max_w && !cur.is_empty() {
                if self.flush_rich_line(img, font, &mut cur, &mut cur_w, x, cursor_y, size) {
                    cursor_y += line_h;
                }
            }
            if let Some(last) = cur.last_mut() {
                if last.color == color && last.italic == italic {
                    last.text.push_str(&word);
                    cur_w += w;
                    continue;
                }
            }
            cur.push(RichRun { text: word, color, italic, split: false });
            cur_w += w;
        }
        if self.flush_rich_line(img, font, &mut cur, &mut cur_w, x, cursor_y, size) {
            cursor_y += line_h;
        }
        cursor_y - y
    }

    /// Draw a pending line of runs, then clear it. Returns whether a line was
    /// drawn (caller advances the cursor by the line height).
    #[allow(clippy::too_many_arguments)]
    fn flush_rich_line(
        &self,
        img: &mut RgbaImage,
        font: &FontArc,
        cur: &mut Vec<RichRun>,
        cur_w: &mut f32,
        x: f32,
        y: f32,
        size: f32,
    ) -> bool {
        if cur.is_empty() {
            return false;
        }
        self.draw_rich_line(img, font, cur, x, y, size);
        cur.clear();
        *cur_w = 0.0;
        true
    }

}

/// A styled run of text produced by parsing the DIY card markup.
pub struct RichRun {
    pub text: String,
    pub color: [u8; 4],
    pub italic: bool,
    /// `[img]…[/img]` — rendered as a full-width split line by the caller.
    pub split: bool,
}

/// Parse the markup used by the 欧丝的印卡机 tool:
///   `[b]…[/b]`  → gold (#ffd676)
///   `[i]…[/i]`  → italic (sheared)
///   `【…】` `『…』` → inner text yellow, brackets stay white
///   `[img]…[/img]` → split-line run
pub fn parse_rich(text: &str) -> Vec<RichRun> {
    let mut runs: Vec<RichRun> = Vec::new();
    let mut color = BODY;
    let mut italic = false;
    let mut cur = String::new();

    let push = |runs: &mut Vec<RichRun>, cur: &mut String, color: [u8; 4], italic: bool| {
        if !cur.is_empty() {
            if let Some(last) = runs.last_mut() {
                if !last.split && last.color == color && last.italic == italic {
                    last.text.push_str(cur);
                    cur.clear();
                    return;
                }
            }
            runs.push(RichRun { text: std::mem::take(cur), color, italic, split: false });
        }
    };

    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let rest = &text[i..];
        let c = rest.chars().next().unwrap();
        match c {
            '[' => {
                if let Some(tag_end) = rest.find(']') {
                    let tag = &rest[1..tag_end];
                    match tag {
                        "b" => {
                            push(&mut runs, &mut cur, color, italic);
                            color = KEYWORD_GOLD;
                        }
                        "/b" => {
                            push(&mut runs, &mut cur, color, italic);
                            color = BODY;
                        }
                        "i" => {
                            push(&mut runs, &mut cur, color, italic);
                            italic = true;
                        }
                        "/i" => {
                            push(&mut runs, &mut cur, color, italic);
                            italic = false;
                        }
                        "img" => {
                            push(&mut runs, &mut cur, color, italic);
                            runs.push(RichRun { text: String::new(), color, italic, split: true });
                        }
                        _ => {
                            cur.push(c);
                        }
                    }
                    i += tag_end + 1;
                    if tag == "img" {
                        // skip to the closing [/img]
                        if let Some(end) = text[i..].find("[/img]") {
                            i += end + 6;
                        }
                    }
                    continue;
                }
                cur.push(c);
                i += 1;
            }
            '【' | '『' => {
                let closing = if c == '【' { '】' } else { '』' };
                push(&mut runs, &mut cur, color, italic);
                cur.push(c); // opening bracket, white
                i += c.len_utf8();
                loop {
                    if i >= bytes.len() {
                        break;
                    }
                    let sub = &text[i..];
                    let ch = sub.chars().next().unwrap();
                    if ch == closing {
                        break;
                    }
                    // digits and underscores stay white inside the brackets
                    let hc = if ch == '_' || "0123456789".contains(ch) {
                        BODY
                    } else {
                        KEYWORD_YELLOW
                    };
                    push(&mut runs, &mut cur, color, italic);
                    color = hc;
                    cur.push(ch);
                    i += ch.len_utf8();
                }
                push(&mut runs, &mut cur, color, italic);
                color = BODY;
                // closing bracket, white
                cur.push(closing);
                i += closing.len_utf8();
            }
            _ => {
                cur.push(c);
                i += c.len_utf8();
            }
        }
    }
    push(&mut runs, &mut cur, color, italic);
    runs
}

/// Straight-alpha "over" blend of `color` at source coverage `sa` onto the
/// canvas. Unlike the old forced-opaque blend, the destination alpha is
/// preserved: over transparent areas the result stays (semi-)transparent, so
/// glows/shadows that spill past the card frame fade smoothly instead of
/// turning into solid black dots.
#[inline]
fn blend_over(img: &mut RgbaImage, x: u32, y: u32, color: [u8; 4], sa: f32) {
    if sa <= 0.0 {
        return;
    }
    let d = img.get_pixel_mut(x, y);
    let da = d[3] as f32 / 255.0;
    let out_a = sa + da * (1.0 - sa);
    if out_a <= 0.0 {
        d[0] = 0;
        d[1] = 0;
        d[2] = 0;
        d[3] = 0;
        return;
    }
    let w_src = sa / out_a;
    let w_dst = da * (1.0 - sa) / out_a;
    d[0] = (color[0] as f32 * w_src + d[0] as f32 * w_dst) as u8;
    d[1] = (color[1] as f32 * w_src + d[1] as f32 * w_dst) as u8;
    d[2] = (color[2] as f32 * w_src + d[2] as f32 * w_dst) as u8;
    d[3] = (out_a * 255.0).round() as u8;
}

/// Alpha-blend a single color onto an RGBA pixel.
#[inline]
fn blend_pixel(img: &mut RgbaImage, x: u32, y: u32, color: [u8; 4], a: f32) {
    blend_over(img, x, y, color, a * (color[3] as f32 / 255.0));
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
    let tint_a = color[3] as f32 / 255.0;
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
        blend_over(canvas, px as u32, py as u32, color, a * tint_a);
    }
}

#[cfg(test)]
mod tests {
    use super::{blend_over, number_pair_kern, parse_rich, BODY, KEYWORD_GOLD, KEYWORD_YELLOW};
    use image::{Rgba, RgbaImage};

    #[test]
    fn blend_over_preserves_transparency() {
        // 50% black over fully transparent: stays semi-transparent black,
        // NOT opaque black (this was the "black dots outside the frame" bug).
        let mut canvas = RgbaImage::from_pixel(1, 1, Rgba([0, 0, 0, 0]));
        blend_over(&mut canvas, 0, 0, [0, 0, 0, 255], 0.5);
        let p = canvas.get_pixel(0, 0);
        assert_eq!(p[0], 0);
        assert!((120..=134).contains(&p[3]), "alpha should be ~127, got {}", p[3]);

        // 50% black over opaque white: gray, still opaque.
        let mut canvas = RgbaImage::from_pixel(1, 1, Rgba([255, 255, 255, 255]));
        blend_over(&mut canvas, 0, 0, [0, 0, 0, 255], 0.5);
        let p = canvas.get_pixel(0, 0);
        assert!((124..=130).contains(&p[0]), "gray ~127, got {}", p[0]);
        assert_eq!(p[3], 255);

        // 25% black over 50%-transparent pixel: alpha rises, stays < 255.
        let mut canvas = RgbaImage::from_pixel(1, 1, Rgba([0, 0, 0, 128]));
        blend_over(&mut canvas, 0, 0, [0, 0, 0, 255], 0.25);
        let p = canvas.get_pixel(0, 0);
        assert!(p[3] > 128 && p[3] < 255, "alpha should be in (128,255), got {}", p[3]);
    }

    #[test]
    fn number_glow_no_opaque_black_on_transparent() {
        // Regression: the glow drawn onto a transparent canvas must fade out
        // smoothly — no fully opaque black pixels may appear (the old forced
        // d[3]=255 blend turned the whole glow tail into solid black dots).
        let fonts_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/fonts");
        let tsukushi = std::fs::read(format!("{fonts_dir}/FOT-TsukuAOldMin-Pr6-E.digits.otf")).unwrap();
        let nanum = std::fs::read(format!("{fonts_dir}/NanumGothic-ExtraBold.ttf")).unwrap();
        super::register_font("title_chs", &nanum);
        super::register_font("number", &tsukushi);
        let engine = super::TextEngine::for_language("chs").unwrap();
        let mut canvas = RgbaImage::from_pixel(300, 200, Rgba([0, 0, 0, 0]));
        // strong glow (radius grows past 1.0) with the number centered
        engine.draw_number(&mut canvas, "7", 150.0, 100.0, 110.0, 220.0, 6.0, 0.0);
        let mut dark = 0u32;
        let mut opaque_dark = 0u32;
        for (_, _, p) in canvas.enumerate_pixels() {
            if p[0] < 60 && p[1] < 60 && p[2] < 60 && p[3] > 0 {
                dark += 1;
                if p[3] == 255 {
                    opaque_dark += 1;
                }
            }
        }
        assert!(dark > 50, "expected a visible glow, got {dark} dark pixels");
        assert_eq!(opaque_dark, 0, "{opaque_dark} opaque black pixels in the glow");
    }

    #[test]
    fn rich_parse_basic() {
        // plain text stays white
        let runs = parse_rich("普通文本");
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].text, "普通文本");
        assert_eq!(runs[0].color, BODY);
    }

    #[test]
    fn rich_parse_bold_and_italic() {
        let runs = parse_rich("前[b]金色[/b]后[i]斜体[/i]");
        let mut it = runs.into_iter();
        let r1 = it.next().unwrap();
        assert_eq!(r1.text, "前");
        let r2 = it.next().unwrap();
        assert_eq!((r2.text.as_str(), r2.color), ("金色", KEYWORD_GOLD));
        let r3 = it.next().unwrap();
        assert_eq!(r3.text, "后");
        let r4 = it.next().unwrap();
        assert_eq!(r4.text, "斜体");
        assert!(r4.italic);
    }

    #[test]
    fn rich_parse_brackets() {
        let runs = parse_rich("【守护】");
        assert_eq!(runs.len(), 3);
        assert_eq!(runs[0].text, "【");
        assert_eq!(runs[0].color, BODY);
        assert_eq!(runs[1].text, "守护");
        assert_eq!(runs[1].color, KEYWORD_YELLOW);
        assert_eq!(runs[2].text, "】");
        assert_eq!(runs[2].color, BODY);
    }

    #[test]
    fn rich_parse_bracket_digits_stay_white() {
        // digits + underscore inside brackets stay white, so everything merges
        // into one white run.
        let runs = parse_rich("『12_』");
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].text, "『12_』");
        assert_eq!(runs[0].color, BODY);
    }

    #[test]
    fn rich_parse_img_split() {
        let runs = parse_rich("甲[img]res://x.png[/img]乙");
        assert_eq!(runs.len(), 3);
        assert!(runs[1].split);
        assert_eq!(runs[2].text, "乙");
    }

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
