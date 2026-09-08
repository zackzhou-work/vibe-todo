use gpui::*;

pub const PIN_SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="17" x2="12" y2="22"/><path d="M5 17h14v-1.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V6h1a1 1 0 0 0 0-2H8a1 1 0 0 0 0 2h1v4.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24Z"/></svg>"#;

pub const HISTORY_SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>"#;

pub const PLUS_SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>"#;

pub const TRASH_SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/></svg>"#;

pub const ARROW_UP_SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="19" x2="12" y2="5"/><polyline points="5 12 12 5 19 12"/></svg>"#;

pub const ARROW_DOWN_SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="5" x2="12" y2="19"/><polyline points="19 12 12 19 5 12"/></svg>"#;

pub const ROTATE_CCW_SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="1 4 1 10 7 10"/><path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10"/></svg>"#;

pub const GRIP_SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="currentColor"><circle cx="9" cy="5" r="1.6"/><circle cx="9" cy="12" r="1.6"/><circle cx="9" cy="19" r="1.6"/><circle cx="15" cy="5" r="1.6"/><circle cx="15" cy="12" r="1.6"/><circle cx="15" cy="19" r="1.6"/></svg>"#;

pub const FLAME_SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="currentColor"><path d="M12 1.6c.6 2.9 2.3 5.3 4.4 7 2.2 1.8 3.3 3.9 3.3 6.1a7.7 7.7 0 0 1-15.4 0c0-1.3.5-2.6 1.2-3.4a2.9 2.9 0 0 0 2.9 2.9 2.9 2.9 0 0 0 2.9-2.9c0-1.6-.6-2.3-1.2-3.5C8.9 5.4 9.6 3.4 12 1.6z"/></svg>"#;

pub const CHECK_SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>"#;

pub fn icon_pin(color: Hsla) -> Svg {
    svg().data(PIN_SVG).text_color(color)
}

pub fn icon_history(color: Hsla) -> Svg {
    svg().data(HISTORY_SVG).text_color(color)
}

pub fn icon_plus(color: Hsla) -> Svg {
    svg().data(PLUS_SVG).text_color(color)
}

pub fn icon_trash(color: Hsla) -> Svg {
    svg().data(TRASH_SVG).text_color(color)
}

pub fn icon_arrow_up(color: Hsla) -> Svg {
    svg().data(ARROW_UP_SVG).text_color(color)
}

pub fn icon_arrow_down(color: Hsla) -> Svg {
    svg().data(ARROW_DOWN_SVG).text_color(color)
}

pub fn icon_rotate_ccw(color: Hsla) -> Svg {
    svg().data(ROTATE_CCW_SVG).text_color(color)
}

pub fn icon_check(color: Hsla) -> Svg {
    svg().data(CHECK_SVG).text_color(color)
}

pub fn icon_flame(color: Hsla) -> Svg {
    svg().data(FLAME_SVG).text_color(color)
}

pub fn icon_grip(color: Hsla) -> Svg {
    svg().data(GRIP_SVG).text_color(color)
}
