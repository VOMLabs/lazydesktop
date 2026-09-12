//! LazyDesktop — Native Git GUI for KDE Plasma
//!
//! GPUI-based frontend that calls Rust backend crates directly.

mod app;
mod commit_panel;
mod file_tree;
mod git_service;
mod sidebar;

use app::LazyDesktopApp;
use gpui::prelude::*;

fn main() {
    let app = gpui_platform::application();
    app.run(move |cx: &mut gpui::App| {
        gpui_component::init(cx);

        cx.open_window(gpui::WindowOptions::default(), |window, cx| {
            let app = cx.new(|cx| LazyDesktopApp::new(window, cx));
            cx.new(|cx| gpui_component::Root::new(app, window, cx))
        })
        .expect("Failed to open window");
        cx.activate(true);
    });
}
