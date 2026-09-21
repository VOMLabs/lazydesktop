//! Settings view — appearance, git identity, data locations.
//!
//! Mirrors the Qt settings dialog (`src/mainwindow.cpp`):
//! - Appearance: theme selection, persisted under `appearance/theme` in the
//!   INI settings file (QSettings-compatible).
//! - Git Identity: `user.name` / `user.email` read from and saved to the
//!   global git config.
//! - Data Locations: read-only display of config paths.
//!
//! Theme changes emit [`SettingsEvent::ThemeChanged`] so the app root can
//! apply the chosen colors (background/foreground).

use gpui::prelude::*;
use gpui::*;
use gpui_component::button::Button;
use gpui_component::input::{Input, InputState};
use gpui_component::*;

use config::paths::{settings_path, themes_dir};
use config::settings::Settings;
use config::themes::{builtin_dark_theme, scan_themes};
use git_cmd::git;
use lazydesktop_app::color::parse_hex_color;

/// Events emitted by the settings view.
#[derive(Clone, Debug)]
pub enum SettingsEvent {
    /// The theme changed. `None` means "System Default" (no override); `Some`
    /// carries the parsed (background, foreground) colors to apply.
    ThemeChanged(Option<(Rgba, Rgba)>),
}

impl EventEmitter<SettingsEvent> for SettingsView {}

pub struct SettingsView {
    /// Selected theme value: `"system"` or a theme name.
    selected_theme: String,
    /// Theme options as (value, display) pairs — builtin first, then custom.
    theme_options: Vec<(String, String)>,
    user_name: Entity<InputState>,
    user_email: Entity<InputState>,
    identity_status: Option<String>,
}

impl SettingsView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let settings = Settings::load(&settings_path()).unwrap_or_default();
        let selected_theme = settings.get_or("appearance/theme", "system").to_string();

        // Always offer the builtin theme first, then custom themes on disk.
        let mut theme_options = vec![("Dark".to_string(), "Dark".to_string())];
        theme_options.extend(
            scan_themes(&themes_dir())
                .into_iter()
                .map(|e| (e.name.clone(), e.name.clone())),
        );

        let stored_name = git::config_get_global("user.name").unwrap_or_default();
        let stored_email = git::config_get_global("user.email").unwrap_or_default();

        let user_name = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Your Name")
                .default_value(stored_name)
        });
        let user_email = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("you@example.com")
                .default_value(stored_email)
        });

        Self {
            selected_theme,
            theme_options,
            user_name,
            user_email,
            identity_status: None,
        }
    }

    /// Persist the chosen theme and emit the applied colors.
    fn select_theme(&mut self, value: &str, cx: &mut Context<Self>) {
        self.selected_theme = value.to_string();

        let mut settings = Settings::load(&settings_path()).unwrap_or_default();
        settings.set("appearance/theme", value);
        let _ = settings.save(); // non-fatal on failure

        cx.emit(SettingsEvent::ThemeChanged(self.theme_colors(value)));
        cx.notify();
    }

    /// Resolve a theme value to (background, foreground) colors, or `None`
    /// for the system default or when a theme is missing required colors.
    fn theme_colors(&self, value: &str) -> Option<(Rgba, Rgba)> {
        if value == "system" {
            return None;
        }

        let (bg_hex, fg_hex) = if value == "Dark" {
            (
                builtin_dark_theme().color("background").map(str::to_owned),
                builtin_dark_theme().color("foreground").map(str::to_owned),
            )
        } else {
            let theme = scan_themes(&themes_dir())
                .iter()
                .find(|e| e.name == value)
                .and_then(|e| config::themes::Theme::load(&e.path).ok());
            match theme {
                Some(t) => (
                    t.color("background").map(str::to_owned),
                    t.color("foreground").map(str::to_owned),
                ),
                None => (None, None),
            }
        };

        match (bg_hex, fg_hex) {
            (Some(bg), Some(fg)) => match (parse_hex_color(&bg), parse_hex_color(&fg)) {
                (Some(b), Some(f)) => Some((rgb(b), rgb(f))),
                _ => None,
            },
            _ => None,
        }
    }

    /// Save git identity (user.name / user.email) to global config.
    fn save_identity(&mut self, cx: &mut Context<Self>) {
        let name = self.user_name.read(cx).value().to_string();
        let email = self.user_email.read(cx).value().to_string();

        let name_ok = git::config_set_global("user.name", &name)
            .map(|r| r.success())
            .unwrap_or(false);
        let email_ok = git::config_set_global("user.email", &email)
            .map(|r| r.success())
            .unwrap_or(false);

        self.identity_status = match (name_ok, email_ok) {
            (true, true) => Some("Saved ✓".to_string()),
            (true, false) => Some("Saved name, failed to save email".to_string()),
            (false, true) => Some("Failed to save name".to_string()),
            (false, false) => Some("Failed to save identity".to_string()),
        };
        cx.notify();
    }
}

impl Render for SettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selected_theme = self.selected_theme.clone();
        let theme_options = self.theme_options.clone();
        let identity_status = self.identity_status.clone();

        div()
            .flex()
            .flex_col()
            .size_full()
            .p_6()
            // Note: interactive methods (scroll, click) need an id first —
            // they live on `StatefulInteractiveElement`, which is implemented
            // for `Stateful<Div>` (the type returned by `.id()`).
            .id("settings-scroll")
            .overflow_y_scroll()
            .child(div().text_xl().font_bold().mb_4().child("Settings"))
            // ── Appearance ──────────────────────────────────────────────
            .child(section_header("Appearance"))
            .child(
                div().flex().flex_col().gap_0p5().children(
                    std::iter::once(("system".to_string(), "System Default".to_string()))
                        .chain(theme_options)
                        .map(|(value, display)| {
                            let is_selected = value == selected_theme;
                            div()
                                .flex()
                                .items_center()
                                .gap_3()
                                .px_3()
                                .py_2()
                                .rounded_md()
                                .when(is_selected, |this| this.bg(rgb(0x0969da)))
                                .id(format!("theme-{value}"))
                                .cursor_pointer()
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.select_theme(&value, cx);
                                }))
                                .child(div().text_sm().child(display))
                                .child(div().flex_1())
                                .child(div().text_sm().child(if is_selected { "●" } else { "" }))
                        }),
                ),
            )
            .child(div().h_6())
            // ── Git identity ────────────────────────────────────────────
            .child(section_header("Git Identity"))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .max_w(px(420.0))
                    .child(labelled_input("User Name:", self.user_name.clone()))
                    .child(labelled_input("Email:", self.user_email.clone()))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(Button::new("save-identity").label("Save").on_click(
                                cx.listener(|this, _, _, cx| {
                                    this.save_identity(cx);
                                }),
                            ))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(match identity_status {
                                        Some(ref s) if s.starts_with("Saved") => rgb(0x3fb950),
                                        Some(_) => rgb(0xf85149),
                                        None => rgb(0x8c959f),
                                    })
                                    .child(identity_status.unwrap_or_else(|| {
                                        "Stored in global git config".to_string()
                                    })),
                            ),
                    ),
            )
            .child(div().h_6())
            // ── Data locations ──────────────────────────────────────────
            .child(section_header("Data Locations"))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(path_row(
                        "Settings:",
                        &settings_path().display().to_string(),
                    ))
                    .child(path_row("Themes:", &themes_dir().display().to_string()))
                    .child(path_row(
                        "Projects:",
                        &config::paths::projects_path().display().to_string(),
                    )),
            )
    }
}

fn section_header(text: impl Into<gpui::SharedString>) -> Div {
    div()
        .text_base()
        .font_bold()
        .mt_4()
        .mb_2()
        .child(text.into())
}

fn labelled_input(label: impl Into<gpui::SharedString>, input_state: Entity<InputState>) -> Div {
    div()
        .flex()
        .items_center()
        .gap_3()
        .child(
            div()
                .text_sm()
                .w(px(112.0))
                .flex_shrink_0()
                .child(label.into()),
        )
        .child(div().flex_1().child(Input::new(&input_state)))
}

fn path_row(label: impl Into<gpui::SharedString>, value: &str) -> Div {
    div()
        .flex()
        .items_center()
        .gap_3()
        .child(
            div()
                .text_sm()
                .text_color(rgb(0x8c959f))
                .w(px(112.0))
                .flex_shrink_0()
                .child(label.into()),
        )
        .child(
            div()
                .text_sm()
                .font_family("monospace")
                .child(value.to_string()),
        )
}
