//! Warm paper, and one thing lifted off it.
//!
//! The window is a single warm off-white from the chrome to the bottom edge —
//! no seams, no second surface — so an empty stretch reads as room rather than
//! as something unfilled. The ongoing card is the only thing that leaves that
//! plane, and it does it by casting a shadow, not by being a different colour.
//!
//! Colour is rationed to two, one job each: blue means done, red means
//! priority. Nothing else is allowed a hue.

use gpui::{px, rgb, rgba, BoxShadow, Hsla};

/// The whole window. Warm — R and G above B — because the cool grey it
/// replaced read as clinical at this size.
pub const GROUND: u32 = 0xFBFAF7;
pub const CARD_SURFACE: u32 = 0xFFFFFF;
pub const HAIRLINE: u32 = 0xEBE7E0;

pub const INK: u32 = 0x1F1D1A;
pub const INK_SOFT: u32 = 0x8B857A;
pub const INK_FAINT: u32 = 0xADA79B;

/// Done. Fills a checkbox the moment it is ticked, and stays with the task in
/// the log.
pub const DONE: u32 = 0x3D7DE8;
/// Priority, and the confirm step on destructive actions.
pub const PRIORITY: u32 = 0xD92D20;

/// Row tints cannot be shared: the card is pure white and the list sits on the
/// warm ground, so a hover that reads on one is invisible on the other.
///
/// `row_hover` is also what the hover actions are painted with — they slide in
/// over the title and have to hide it, so tray and row must be the same colour.
#[derive(Clone, Copy)]
pub struct Surface {
    pub row_hover: u32,
    pub action_bg: u32,
    pub action_bg_hover: u32,
    pub field: u32,
}

pub const CARD: Surface = Surface {
    row_hover: 0xF6F4F0,
    action_bg: 0xEDEAE4,
    action_bg_hover: 0xE2DED6,
    field: 0xFFFFFF,
};

pub const LIST: Surface = Surface {
    row_hover: 0xF3F0EA,
    action_bg: 0xE9E5DD,
    action_bg_hover: 0xDED9CF,
    field: 0xFFFFFF,
};

/// The card is barely a different colour from the ground now, so this is what
/// separates it: a contact shadow to sit it down, and a wider one to lift it.
pub fn card_shadow() -> Vec<BoxShadow> {
    vec![
        BoxShadow::new(px(0.), px(1.), hsl_a(0x0000001A)).blur_radius(px(2.)),
        BoxShadow::new(px(0.), px(4.), hsl_a(0x00000017))
            .blur_radius(px(12.))
            .spread_radius(px(-2.)),
    ]
}

pub fn popover_shadow() -> Vec<BoxShadow> {
    vec![
        BoxShadow::new(px(0.), px(2.), hsl_a(0x00000017)).blur_radius(px(5.)),
        BoxShadow::new(px(0.), px(10.), hsl_a(0x00000024))
            .blur_radius(px(26.))
            .spread_radius(px(-6.)),
    ]
}

/// 0xRRGGBB
pub fn hsl(color: u32) -> Hsla {
    rgb(color).into()
}

/// 0xRRGGBBAA
pub fn hsl_a(color: u32) -> Hsla {
    rgba(color).into()
}
