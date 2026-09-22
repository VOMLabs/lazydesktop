//! LazyDesktop — Native Git GUI for KDE Plasma
//!
//! GPUI-based frontend that calls Rust backend crates directly.

mod app;
mod commit_panel;
mod diff_view;
mod file_tree;
mod git_service;
mod settings_view;
mod sidebar;

use app::LazyDesktopApp;
use config::paths::{settings_path, themes_dir};
use config::settings::Settings;
use config::themes::{scan_themes, Theme as ThemeFile};
use gpui::prelude::*;
use lazydesktop_app::theme::Palette;

fn main() {
    let app = gpui_platform::application();
    app.run(move |cx: &mut gpui::App| {
        gpui_component::init(cx);

        // Apply the persisted theme (if any) before the window opens so the
        // whole component kit matches from the first frame.
        Palette::install(startup_palette(cx), cx);

        cx.open_window(gpui::WindowOptions::default(), |window, cx| {
            let app = cx.new(|cx| LazyDesktopApp::new(window, cx));
            cx.new(|cx| gpui_component::Root::new(app, window, cx))
        })
        .expect("Failed to open window");
        cx.activate(true);
    });
}

/// Resolve the startup palette from the persisted `appearance/theme` setting,
/// falling back to the system appearance.
fn startup_palette(cx: &gpui::App) -> Palette {
    let saved = Settings::load(&settings_path())
        .ok()
        .and_then(|s| s.get("appearance/theme").map(str::to_owned));

    match saved.as_deref() {
        Some("Dark") => Palette::dark(),
        Some("Light") => Palette::light(),
        Some(other) => scan_themes(&themes_dir())
            .iter()
            .find(|e| e.name == other)
            .and_then(|e| ThemeFile::load(&e.path).ok())
            .map(|theme| Palette::from_theme(&theme))
            .unwrap_or_else(|| system_palette(cx)),
        _ => system_palette(cx),
    }
}

/// Follow the OS appearance into the matching palette.
fn system_palette(cx: &gpui::App) -> Palette {
    match cx.window_appearance() {
        gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark => Palette::dark(),
        _ => Palette::light(),
    }
}
