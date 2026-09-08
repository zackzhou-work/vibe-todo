//! Two temperatures.
//!
//! The data model is a flow between two states, so the surfaces carry that
//! difference instead of a label: Inbox is cool and recessed, Ongoing is warm
//! paper and raised. You can tell the columns apart with the text blurred out.
//!
//! Ember is the only chromatic value in the app. It appears in exactly two
//! places — the priority edge bar and the ongoing count — so it still means
//! something when it shows up.

use gpui::{rgb, Hsla};

pub const INK: u32 = 0x1F1D1A;
pub const INK_SOFT: u32 = 0x8B857A;
pub const INK_FAINT: u32 = 0xADA79B;
pub const EMBER: u32 = 0xA8620A;
/// Priority, and the confirm step on destructive actions. Red rather than
/// ember because both need to be seen before anything else in the row.
pub const PRIORITY: u32 = 0xD92D20;

pub const INBOX_SURFACE: u32 = 0xF2F2F0;
pub const ONGOING_SURFACE: u32 = 0xFCFBF9;
pub const RULE: u32 = 0xE6E3DD;
/// The window chrome sits on its own plane, between the two column
/// temperatures, so the top of the window reads as one bar rather than two
/// mismatched halves.
pub const CHROME_SURFACE: u32 = 0xF7F6F3;
pub const POPOVER_SURFACE: u32 = 0xFFFFFF;

/// Per-column tints. Row hover has to sit on top of the column's own surface,
/// so it cannot be one shared value.
#[derive(Clone, Copy)]
pub struct ColumnSkin {
    pub surface: u32,
    pub row_hover: u32,
    pub action_bg: u32,
    pub action_bg_hover: u32,
    pub count_bg: u32,
    pub count_fg: u32,
}

pub const INBOX_SKIN: ColumnSkin = ColumnSkin {
    surface: INBOX_SURFACE,
    row_hover: 0xE9E9E5,
    action_bg: 0xDFDFDA,
    action_bg_hover: 0xD2D2CB,
    count_bg: 0xE2E2DD,
    count_fg: 0x6E6A62,
};

pub const ONGOING_SKIN: ColumnSkin = ColumnSkin {
    surface: ONGOING_SURFACE,
    row_hover: 0xF3F1EC,
    action_bg: 0xEBE8E1,
    action_bg_hover: 0xDFDBD2,
    count_bg: 0xF6E9DA,
    count_fg: EMBER,
};

pub fn hsl(color: u32) -> Hsla {
    rgb(color).into()
}
