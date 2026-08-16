//! Card configuration model (deserialized from the web UI as JSON).

use serde::Deserialize;

/// Render styles: official WB card or the BYD-DIY (欧丝的印卡机) product.
pub const STYLE_WB: &str = "wb";
#[allow(dead_code)]
pub const STYLE_DIY: &str = "diy";

/// Card types (matches WBArts `card.type`).
pub const KIND_FOLLOWER: u8 = 1;
#[allow(dead_code)]
pub const KIND_AMULET: u8 = 2;
#[allow(dead_code)]
pub const KIND_SPELL: u8 = 3;

/// DIY rarity 5 = peculiar (异画) — only exists in the DIY style.
#[allow(dead_code)]
pub const RARITY_PECULIAR: u8 = 5;

/// Normalized crop rectangle (each component in [0,1] relative to the original
/// art image). Produced by the web crop panel; `None` falls back to cover-fit.
#[derive(Deserialize, Clone)]
pub struct ArtCrop {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Deserialize, Clone)]
#[serde(default)]
pub struct CardConfig {
    // --- basic identity ---
    pub name: String,
    pub name_size: f32,       // card name font size (design units), default 60
    pub language: String,     // chs / cht / jpn / kor / eng
    pub class: u8,            // 0=neutral .. 7=nemesis (WBArts numbering)
    pub kind: u8,             // 1 follower / 2 amulet / 3 spell
    pub rarity: u8,           // 1 bronze / 2 silver / 3 gold / 4 legend
    pub frame: String,        // exact frame key, e.g. "follower_legend" / "spell_style_101"
    pub cost: String,         // cost text ("1" .. "10+")
    pub atk: String,
    pub life: String,

    // --- art crop ---
    pub crop: Option<ArtCrop>,

    // --- skill text sections ---
    // These are not drawn on the pure WB card image; they are rendered only in
    // the DIY style (正文/第二段正文/进化/超进化/纹章/署名).
    pub detail1: String,      // 正文 (sv-byd-diy only)
    pub detail2: String,      // 第二段正文 (sv-byd-diy only)
    pub evolve: String,       // 进化时 text (sv-byd-diy only)
    pub super_evolve: String, // 超进化 text (sv-byd-diy only)

    pub illustrator: String,  // 画师 (sv-byd-diy only)
    pub diy: String,          // DIY 作者 (sv-byd-diy only)

    pub show_detail2: bool,
    pub show_evolve: bool,
    pub show_super: bool,
    pub show_illustrator: bool,
    pub show_diy: bool,

    // --- text styling ---
    pub text_size: f32,       // skill text font size (sv-byd-diy only), default 24
    pub number_size: f32,     // cost/atk/def number size (design units), default 125
    pub number_spacing: f32,  // extra uniform letter spacing between digits, default 0
    pub number_shadow: f32,   // number glow shadow strength (>= 0), default 4.0
    pub cost_dx: f32,         // cost number x offset, default 0
    pub cost_dy: f32,         // cost number y offset, default 0
    pub atk_dx: f32,          // atk number x offset, default 0
    pub atk_dy: f32,          // atk number y offset, default 0
    pub def_dx: f32,          // def number x offset, default 0
    pub def_dy: f32,          // def number y offset, default 0
    pub bg_alpha: f32,        // text box background opacity 0..=1, default 0.55

    // --- output ---
    pub scale: f32,           // output scale multiplier; 1.0 => 782x1024

    // --- style ---
    pub style: String,        // "wb" (default) or "diy"

    // --- DIY-only (欧丝的印卡机 style) ---
    pub bg_type: u8,          // 1 = 一代(OG dark), 2 = byd (default)
    pub trait_text: String,   // 兵种类型 free text shown in the title band
    pub class_title: String,  // localized "职业" label (title band)
    pub type_title: String,   // localized "类型" label (title band)
    pub class_text: String,   // localized class name (title band)
    pub illus_title: String,  // localized "画师：" label (signature row)
    pub d1_size: f32,         // 正文 font size (px), default 32.4
    pub d2_size: f32,
    pub ev_size: f32,
    pub super_size: f32,
    /// 纹章块（能力面板动态添加，可多个）
    #[serde(default)]
    pub crests: Vec<CrestBlock>,
}

/// 一个纹章块（能力面板可添加多个）
#[derive(Deserialize, Clone, Default)]
pub struct CrestBlock {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub border: u8,
    #[serde(default = "default_crest_scale")]
    pub scale: f32,
    #[serde(default)]
    pub icon1: String,
    /// icon1 == "upload" 时对应的 PNG base64
    #[serde(default)]
    pub icon1_data: Option<String>,
    #[serde(default)]
    pub icon2: String,
    /// icon2 == "upload" 时对应的 PNG base64
    #[serde(default)]
    pub icon2_data: Option<String>,
    #[serde(default)]
    pub show_icon2: bool,
    #[serde(default = "default_crest_text_size")]
    pub size: f32,
}

fn default_crest_scale() -> f32 {
    1.0
}

fn default_crest_text_size() -> f32 {
    32.4
}

impl Default for CardConfig {
    fn default() -> Self {
        CardConfig {
            name: String::new(),
            name_size: 60.0,
            language: String::from("chs"),
            class: 0,
            kind: KIND_FOLLOWER,
            rarity: 1,
            frame: String::from("follower_bronze"),
            cost: String::from("1"),
            atk: String::from("1"),
            life: String::from("1"),
            crop: None,
            detail1: String::new(),
            detail2: String::new(),
            evolve: String::new(),
            super_evolve: String::new(),
            illustrator: String::new(),
            diy: String::new(),
            show_detail2: false,
            show_evolve: false,
            show_super: false,
            show_illustrator: false,
            show_diy: false,
            text_size: 24.0,
            number_size: 125.0,
            number_spacing: 0.0,
            number_shadow: 4.0,
            cost_dx: 0.0,
            cost_dy: 3.0,
            atk_dx: 12.0,
            atk_dy: -13.0,
            def_dx: -17.0,
            def_dy: -14.0,
            bg_alpha: 0.55,
            scale: 1.0,
            style: String::from(STYLE_WB),
            bg_type: 2,
            trait_text: String::new(),
            class_title: String::from("职业"),
            type_title: String::from("类型"),
            class_text: String::new(),
            illus_title: String::from("画师："),
            d1_size: 32.4,
            d2_size: 32.4,
            ev_size: 32.4,
            super_size: 32.4,
            crests: Vec::new(),
        }
    }
}
