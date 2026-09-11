//! Warm paper, and the two things allowed to leave it.
//!
//! The window is a single warm off-white from the chrome to the bottom edge —
//! no seams, no second surface — so an empty stretch reads as room rather than
//! as something unfilled. Both task sections live on that plane and are told
//! apart by a heading and a hairline, nothing else. Only the history popover
//! and the card under the cursor during a drag lift off it, and they do it by
//! casting a shadow.
//!
//! Colour is rationed to one job: red means priority. Done is ink on paper like
//! everything else — a tick drawn across the box, not a hue swapped into it.

use gpui::{linear_color_stop, linear_gradient, px, rgb, rgba, Background, BoxShadow, Hsla};

/// The whole window. Warm — R and G above B — because the cool grey it
/// replaced read as clinical at this size.
pub const GROUND: u32 = 0xFBFAF7;
/// The only white in the window: whatever is floating above the paper.
pub const RAISED: u32 = 0xFFFFFF;
pub const HAIRLINE: u32 = 0xEBE7E0;

pub const INK: u32 = 0x1F1D1A;
pub const INK_SOFT: u32 = 0x8B857A;
pub const INK_FAINT: u32 = 0xADA79B;

/// Priority, and the confirm step on destructive actions.
pub const PRIORITY: u32 = 0xD92D20;

/// A hover tint that reads on the warm ground disappears on white, so the
/// paper and the popover cannot share one.
///
/// `row_hover` is also what the hover actions are painted with — they slide in
/// over the title and have to hide it, so tray and row must be the same colour.
#[derive(Clone, Copy)]
pub struct Surface {
    pub row_hover: u32,
    pub action_bg: u32,
    pub action_bg_hover: u32,
    /// One step past hover. Pressing has to change something the instant the
    /// button goes down — gpui has no transforms, so colour is the only channel
    /// available for it.
    pub action_bg_active: u32,
}

pub const POPOVER: Surface = Surface {
    row_hover: 0xF6F4F0,
    action_bg: 0xEDEAE4,
    action_bg_hover: 0xE2DED6,
    action_bg_active: 0xD6D1C7,
};

pub const PAPER: Surface = Surface {
    row_hover: 0xF3F0EA,
    action_bg: 0xE9E5DD,
    action_bg_hover: 0xDED9CF,
    action_bg_active: 0xD1CBBF,
};

/// The hover tray's own background: transparent at its left edge, solid by the
/// time the first button starts, so a long title dissolves under it instead of
/// being cut off mid-character.
///
/// Both stops are `row_hover` and differ only in alpha. Fading to a generic
/// transparent instead interpolates through it and leaves a grey smear down the
/// middle of the ramp.
pub fn tray_fade(skin: Surface) -> Background {
    linear_gradient(
        // 0 is up, increasing clockwise: 90 runs left to right.
        90.,
        linear_color_stop(hsl(skin.row_hover).opacity(0.), 0.),
        linear_color_stop(hsl(skin.row_hover), TRAY_FADE_END),
    )
}

/// Where the ramp reaches full opacity, as a fraction of the tray's width. Set
/// so it lands on the first button's left edge — a button half-sunk in the ramp
/// shows the title faintly through itself.
const TRAY_FADE_END: f32 = 0.25;

/// The card under the cursor during a drag: a contact shadow to sit it down,
/// and a wider one to lift it off whatever it is passing over.
pub fn ghost_shadow() -> Vec<BoxShadow> {
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
