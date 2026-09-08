use gpui::*;
use vibe_todo::app::{Cancel, DeleteHovered, NewTask, ToggleHistory, TogglePin, VibeTodoApp};
use vibe_todo::db::Database;
use vibe_todo::window_level::set_window_always_on_top;

/// Narrower than this and the hover tray covers most of a title; shorter and
/// the ongoing card has nothing left to show. It is where resizing stops, not a
/// layout limit.
const MIN_WIDTH: f32 = 260.;
const MIN_HEIGHT: f32 = 220.;

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        gpui_component::init(cx);

        cx.bind_keys([
            KeyBinding::new("cmd-n", NewTask, Some("VibeTodo")),
            KeyBinding::new("cmd-shift-p", TogglePin, Some("VibeTodo")),
            KeyBinding::new("cmd-shift-h", ToggleHistory, Some("VibeTodo")),
            KeyBinding::new("escape", Cancel, Some("VibeTodo")),
            // Bound outside the editing context on purpose: in a text field
            // cmd-backspace means "delete to the start of the line", and a
            // binding here would eat it and delete the hovered task instead.
            KeyBinding::new("cmd-backspace", DeleteHovered, Some("VibeTodo && !editing")),
        ]);

        let db = Database::new().expect("Failed to initialize SQLite database");
        let saved = db.load_window_state();

        let restored = saved
            .filter(|state| state.width >= MIN_WIDTH && state.height >= MIN_HEIGHT)
            .map(|state| Bounds {
                origin: point(px(state.x), px(state.y)),
                size: size(px(state.width), px(state.height)),
            })
            .filter(|bounds| is_reachable(bounds, cx));
        let window_bounds =
            restored.unwrap_or_else(|| Bounds::centered(None, size(px(380.), px(520.)), cx));
        let is_pinned = saved.is_some_and(|state| state.is_pinned);

        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(window_bounds)),
            titlebar: Some(TitlebarOptions {
                title: None,
                appears_transparent: true,
                traffic_light_position: Some(point(px(14.), px(11.))),
            }),
            focus: true,
            show: true,
            kind: WindowKind::Normal,
            is_movable: true,
            is_resizable: true,
            window_min_size: Some(size(px(MIN_WIDTH), px(MIN_HEIGHT))),
            ..Default::default()
        };

        cx.open_window(options, |window, cx| {
            cx.new(|cx| VibeTodoApp::new(db, is_pinned, window, cx))
        })
        .expect("Failed to open Vibe Todo window");

        // Raising the level walks the app's open windows, so it can only run
        // once there is one.
        if is_pinned {
            set_window_always_on_top(true);
        }
    });
}

/// Geometry saved on a display that is no longer attached would put the window
/// somewhere the user cannot see or drag it back from.
fn is_reachable(bounds: &Bounds<Pixels>, cx: &App) -> bool {
    cx.displays()
        .iter()
        .any(|display| display.bounds().intersects(bounds))
}
