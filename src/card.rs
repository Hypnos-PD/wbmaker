//! Card configuration model (deserialized from the web UI as JSON).

use serde::Deserialize;

/// Card types (matches WBArts `card.type`).
pub const KIND_FOLLOWER: u8 = 1;
#[allow(dead_code)]
pub const KIND_AMULET: u8 = 2;
#[allow(dead_code)]
pub const KIND_SPELL: u8 = 3;

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

    // --- skill text sections ---
    // No text is rendered onto the pure card image. All of these fields are
    // reserved for the sv-byd-diy-style export (正文/第二段正文/进化/超进化/
    // 纹章/署名) and are not drawn on the card.
    pub detail1: String,      // 正文 (sv-byd-diy only)
    pub detail2: String,      // 第二段正文 (sv-byd-diy only)
    pub evolve: String,       // 进化时 text (sv-byd-diy only)
    pub super_evolve: String, // 超进化 text (sv-byd-diy only)
    pub crest: String,        // 纹章 text (sv-byd-diy only)
    pub crest_name: String,   // crest name (sv-byd-diy only)
    pub illustrator: String,  // 画师 (sv-byd-diy only)
    pub diy: String,          // DIY 作者 (sv-byd-diy only)

    pub show_detail2: bool,
    pub show_evolve: bool,
    pub show_super: bool,
    pub show_crest: bool,
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
            detail1: String::new(),
            detail2: String::new(),
            evolve: String::new(),
            super_evolve: String::new(),
            crest: String::new(),
            crest_name: String::new(),
            illustrator: String::new(),
            diy: String::new(),
            show_detail2: false,
            show_evolve: false,
            show_super: false,
            show_crest: false,
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
        }
    }
}
