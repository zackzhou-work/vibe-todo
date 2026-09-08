#![allow(unexpected_cfgs)]
#[cfg(target_os = "macos")]
#[allow(deprecated)]
pub fn set_window_always_on_top(pinned: bool) {
    use cocoa::base::id;
    use objc::{msg_send, sel, sel_impl};

    unsafe {
        let app = cocoa::appkit::NSApp();
        if !app.is_null() {
            let windows: id = msg_send![app, windows];
            let count: usize = msg_send![windows, count];
            for i in 0..count {
                let win: id = msg_send![windows, objectAtIndex: i];
                // 3 is NSFloatingWindowLevel, 0 is NSNormalWindowLevel
                let level: i64 = if pinned { 3 } else { 0 };
                let _: () = msg_send![win, setLevel: level];
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn set_window_always_on_top(_pinned: bool) {}
