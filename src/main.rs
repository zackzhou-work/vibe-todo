use gpui::*;
use vibe_todo::app::VibeTodoApp;

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        gpui_component::init(cx);

        let window_bounds =
            WindowBounds::Windowed(Bounds::centered(None, size(px(380.), px(520.)), cx));

        let options = WindowOptions {
            window_bounds: Some(window_bounds),
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
            window_min_size: Some(size(px(320.), px(280.))),
            ..Default::default()
        };

        cx.open_window(options, |window, cx| {
            cx.new(|cx| VibeTodoApp::new(window, cx))
        })
        .expect("Failed to open Vibe Todo window");
    });
}
