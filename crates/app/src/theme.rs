//! Shared shade-based UI palette for the application.
//!
//! This is the single source of truth for every color the UI draws. The
//! [`Palette`] is stored as a gpui global so any render call can read the
//! current set of tokens, and [`Palette::install`] also projects the palette
//! into the gpui-component theme (buttons, inputs, checkboxes, lists, dialogs,
//! popovers …) so shared widgets pick up the same shades instead of the
//! component kit's default shadcn light palette.
//!
//! Split out from the GPUI element machinery (like `color.rs` / `diff.rs`) so
//! it can be unit-tested in the library target.

use std::rc::Rc;

use gpui::{rgb, rgba, App, Global, Rgba};
use gpui_component::{Theme, ThemeMode, ThemeSet};

use crate::color::parse_hex_color;

/// A complete set of app-drawn color tokens, organized by role so surfaces and
/// text keep a deliberate, restrained hierarchy (Zed-inspired: layered
/// surfaces, subtle contrast, muted structure, one calm accent).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    /// Whether this palette targets a dark background.
    pub dark: bool,
    // --- surfaces (darkest → lightest) ---
    /// Application background.
    pub bg: Rgba,
    /// Sidebars, panels, toolbars — one step above the background.
    pub panel: Rgba,
    /// Elevated surfaces: badges, action bars, popovers.
    pub elevated: Rgba,
    /// Text inputs and editable fields.
    pub input: Rgba,
    /// Floating surfaces: menus, popovers, dialogs.
    pub popover: Rgba,
    // --- interactive states ---
    /// Row / control hover.
    pub hover: Rgba,
    /// Pressed / armed state.
    pub active: Rgba,
    /// Selected row background (neutral).
    pub selected: Rgba,
    /// Keyboard focus ring (translucent accent).
    pub focus_ring: Rgba,
    // --- structure ---
    /// Panel and control borders.
    pub border: Rgba,
    /// Hairline separators between sections.
    pub separator: Rgba,
    // --- text ---
    /// Primary text.
    pub text_primary: Rgba,
    /// Secondary text (labels, counts).
    pub text_secondary: Rgba,
    /// Muted text (metadata, placeholders).
    pub text_muted: Rgba,
    // --- accent ---
    /// Primary accent (brand blue).
    pub accent: Rgba,
    pub accent_hover: Rgba,
    pub accent_active: Rgba,
    /// Text on top of the accent.
    pub accent_foreground: Rgba,
    /// Subtle accent-tinted background for the current / selected item.
    pub accent_selected: Rgba,
    // --- semantic ---
    pub success: Rgba,
    pub error: Rgba,
    pub warning: Rgba,
    /// Diff hunk-header blue.
    pub hunk: Rgba,
    // --- file status ---
    pub status_modified: Rgba,
    pub status_added: Rgba,
    pub status_deleted: Rgba,
    pub status_renamed: Rgba,
    pub status_untracked: Rgba,
    // --- diff ---
    pub diff_add_fg: Rgba,
    pub diff_del_fg: Rgba,
    pub diff_ctx_fg: Rgba,
    pub diff_add_bg: Rgba,
    pub diff_del_bg: Rgba,
    pub diff_gutter: Rgba,
}

impl Global for Palette {}

/// Opaque 0xRRGGBB.
fn opa(hex: u32) -> Rgba {
    rgb(hex)
}

/// 0xRRGGBB plus a hex alpha byte (0x00..=0xff).
fn with_alpha(hex: u32, alpha: u32) -> Rgba {
    rgba((hex << 8) | alpha)
}

impl Palette {
    /// The default dark palette (brand identity, Zed-inspired surfaces).
    pub fn dark() -> Self {
        Self {
            dark: true,
            bg: opa(0x181a1f),
            panel: opa(0x1d2027),
            elevated: opa(0x232730),
            input: opa(0x20242d),
            popover: opa(0x232730),
            hover: opa(0x232833),
            active: opa(0x2a3040),
            selected: opa(0x2b3447),
            focus_ring: with_alpha(0x568af2, 0x66),
            border: opa(0x2b303b),
            separator: opa(0x252a34),
            text_primary: opa(0xd7dae0),
            text_secondary: opa(0x9da4b1),
            text_muted: opa(0x7a8190),
            accent: opa(0x568af2),
            accent_hover: opa(0x6b99f4),
            accent_active: opa(0x4777e8),
            accent_foreground: opa(0xffffff),
            accent_selected: with_alpha(0x568af2, 0x16),
            success: opa(0x4f9e6a),
            error: opa(0xe06c75),
            warning: opa(0xdfae6b),
            hunk: opa(0x7aa2f7),
            status_modified: opa(0xd9a95f),
            status_added: opa(0x4f9e6a),
            status_deleted: opa(0xe06c75),
            status_renamed: opa(0x568af2),
            status_untracked: opa(0x858b9c),
            diff_add_fg: opa(0x6ccd84),
            diff_del_fg: opa(0xe06c75),
            diff_ctx_fg: opa(0xbfc6d0),
            diff_add_bg: with_alpha(0x4f9e6a, 0x14),
            diff_del_bg: with_alpha(0xe06c75, 0x14),
            diff_gutter: opa(0x5b6274),
        }
    }

    /// The default light palette.
    pub fn light() -> Self {
        Self {
            dark: false,
            bg: opa(0xf6f7f9),
            panel: opa(0xeef0f3),
            elevated: opa(0xe6e9ee),
            input: opa(0xffffff),
            popover: opa(0xffffff),
            hover: opa(0xe7eaef),
            active: opa(0xdbe0e8),
            selected: opa(0xd9e1f4),
            focus_ring: with_alpha(0x2f6fe0, 0x4d),
            border: opa(0xd7dce4),
            separator: opa(0xe1e5ec),
            text_primary: opa(0x1c1f26),
            text_secondary: opa(0x4c5566),
            text_muted: opa(0x667085),
            accent: opa(0x2f6fe0),
            accent_hover: opa(0x2457c0),
            accent_active: opa(0x2a5ccc),
            accent_foreground: opa(0xffffff),
            accent_selected: with_alpha(0x2f6fe0, 0x14),
            success: opa(0x2f8f5b),
            error: opa(0xc33f54),
            warning: opa(0xa97e2d),
            hunk: opa(0x2f6fe0),
            status_modified: opa(0xb07d2d),
            status_added: opa(0x2f8f5b),
            status_deleted: opa(0xc33f54),
            status_renamed: opa(0x2f6fe0),
            status_untracked: opa(0x7d8698),
            diff_add_fg: opa(0x1f8448),
            diff_del_fg: opa(0xc2363e),
            diff_ctx_fg: opa(0x4c5566),
            diff_add_bg: with_alpha(0x2f8f5b, 0x14),
            diff_del_bg: with_alpha(0xc33f54, 0x14),
            diff_gutter: opa(0x98a1b3),
        }
    }

    /// Derive a palette from a theme definition file.
    ///
    /// The mode is chosen from the perceived luminance of the theme's
    /// `background` color (if present), so both dark and light custom themes
    /// resolve to the matching tuned token set. Falls back to the dark palette
    /// when the background is missing or unparseable.
    pub fn from_theme(theme: &config::themes::Theme) -> Self {
        let Some(bg) = theme.color("background").and_then(parse_hex_color) else {
            return Self::dark();
        };
        if relative_luminance(bg) < 0.5 {
            Self::dark()
        } else {
            Self::light()
        }
    }

    /// Read the palette installed for the current app, defaulting to dark.
    pub fn current(cx: &App) -> Self {
        cx.try_global::<Palette>()
            .copied()
            .unwrap_or_else(Self::dark)
    }

    /// Install this palette as the app's global token set and project it into
    /// the gpui-component theme so shared widgets match the app-drawn layers.
    pub fn install(self, cx: &mut App) {
        cx.set_global(self);
        apply_component_theme(&self, cx);
    }
}

/// Perceived luminance of an 0xRRGGBB color, 0.0 (black) ..= 1.0 (white).
fn relative_luminance(hex: u32) -> f32 {
    let r = ((hex >> 16) & 0xff) as f32 / 255.0;
    let g = ((hex >> 8) & 0xff) as f32 / 255.0;
    let b = (hex & 0xff) as f32 / 255.0;
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

// --- gpui-component projection -------------------------------------------------

/// Project the palette onto the gpui-component theme so `Button`, `Input`,
/// `Checkbox`, lists, popovers and dialogs share the same shade language.
/// Builds a `ThemeConfig` from the palette directly (no registry mutation) and
/// applies it with the component's [Theme::apply_config] + [Theme::change]
/// flow so semantic tokens follow along.
fn apply_component_theme(p: &Palette, cx: &mut App) {
    let mode = if p.dark {
        ThemeMode::Dark
    } else {
        ThemeMode::Light
    };

    let colors = component_colors(p);
    let json = serde_json::json!({
        "name": "LazyDesktop",
        "themes": [{
            "name": "LazyDesktop",
            "mode": mode.name(),
            "colors": colors,
        }],
    });

    let mut fallback = || Theme::change(mode, None, cx);

    let Ok(set) = serde_json::from_str::<ThemeSet>(&json.to_string()) else {
        return fallback();
    };
    let Some(config) = set.themes.into_iter().next() else {
        return fallback();
    };

    Theme::global_mut(cx).apply_config(&Rc::new(config));
    Theme::change(mode, None, cx);
}

fn val(s: String) -> serde_json::Value {
    serde_json::Value::String(s)
}

/// Build the gpui-component `ThemeConfigColors` object (dotted token keys)
/// from the palette. Keys not present fall back to the component defaults.
fn component_colors(p: &Palette) -> serde_json::Map<String, serde_json::Value> {
    let mut colors = serde_json::Map::new();

    // Surfaces + structure.
    colors.insert("background".into(), val(hex6(p.bg)));
    colors.insert("foreground".into(), val(hex6(p.text_primary)));
    colors.insert("border".into(), val(hex6(p.border)));
    colors.insert("overlay".into(), val(hex6(p.bg)));
    colors.insert("caret".into(), val(hex6(p.text_primary)));
    colors.insert("ring".into(), val(hex8(p.focus_ring)));
    colors.insert("input.border".into(), val(hex6(p.border)));
    colors.insert("selection.background".into(), val(hex6(p.accent)));
    colors.insert("muted.background".into(), val(hex6(p.elevated)));
    colors.insert("muted.foreground".into(), val(hex6(p.text_muted)));

    // Buttons.
    colors.insert("button.background".into(), val(hex6(p.elevated)));
    colors.insert("button.foreground".into(), val(hex6(p.text_primary)));
    colors.insert("button.hover.background".into(), val(hex6(p.hover)));
    colors.insert("button.active.background".into(), val(hex6(p.active)));
    colors.insert("button.primary.background".into(), val(hex6(p.accent)));
    colors.insert(
        "button.primary.foreground".into(),
        val(hex6(p.accent_foreground)),
    );
    colors.insert(
        "button.primary.hover.background".into(),
        val(hex6(p.accent_hover)),
    );
    colors.insert(
        "button.primary.active.background".into(),
        val(hex6(p.accent_active)),
    );
    colors.insert("button.secondary.background".into(), val(hex6(p.elevated)));
    colors.insert(
        "button.secondary.foreground".into(),
        val(hex6(p.text_primary)),
    );
    colors.insert(
        "button.secondary.hover.background".into(),
        val(hex6(p.hover)),
    );
    colors.insert(
        "button.secondary.active.background".into(),
        val(hex6(p.active)),
    );
    colors.insert("button.danger.background".into(), val(hex6(p.error)));
    colors.insert(
        "button.danger.foreground".into(),
        val(hex6(p.accent_foreground)),
    );
    colors.insert(
        "button.danger.hover.background".into(),
        val(hex8(tinted(p.error, 0xd0))),
    );
    colors.insert(
        "button.danger.active.background".into(),
        val(hex8(tinted(p.error, 0xb8))),
    );

    // Semantic role colors.
    colors.insert("primary.background".into(), val(hex6(p.accent)));
    colors.insert("primary.foreground".into(), val(hex6(p.accent_foreground)));
    colors.insert("primary.hover.background".into(), val(hex6(p.accent_hover)));
    colors.insert(
        "primary.active.background".into(),
        val(hex6(p.accent_active)),
    );
    colors.insert("secondary.background".into(), val(hex6(p.elevated)));
    colors.insert("secondary.foreground".into(), val(hex6(p.text_primary)));
    colors.insert("secondary.hover.background".into(), val(hex6(p.hover)));
    colors.insert("secondary.active.background".into(), val(hex6(p.active)));
    colors.insert("accent.background".into(), val(hex6(p.accent)));
    colors.insert("accent.foreground".into(), val(hex6(p.accent_foreground)));
    colors.insert("danger.background".into(), val(hex6(p.error)));
    colors.insert("danger.foreground".into(), val(hex6(p.accent_foreground)));
    colors.insert(
        "danger.hover.background".into(),
        val(hex8(tinted(p.error, 0xd0))),
    );
    colors.insert(
        "danger.active.background".into(),
        val(hex8(tinted(p.error, 0xb8))),
    );
    colors.insert("success.background".into(), val(hex6(p.success)));
    colors.insert("success.foreground".into(), val(hex6(p.accent_foreground)));
    colors.insert("warning.background".into(), val(hex6(p.warning)));
    colors.insert("warning.foreground".into(), val(hex6(opa(0x1b1e24))));
    colors.insert("info.background".into(), val(hex6(p.hunk)));
    colors.insert("info.foreground".into(), val(hex6(opa(0x1b1e24))));

    // Lists, popovers, sidebar.
    colors.insert("list.background".into(), val(hex6(p.panel)));
    colors.insert("list.hover.background".into(), val(hex6(p.hover)));
    colors.insert("list.active.background".into(), val(hex6(p.selected)));
    colors.insert("list.active.border".into(), val(hex6(p.accent)));
    colors.insert("popover.background".into(), val(hex6(p.popover)));
    colors.insert("popover.foreground".into(), val(hex6(p.text_primary)));
    colors.insert("sidebar.background".into(), val(hex6(p.panel)));
    colors.insert("sidebar.foreground".into(), val(hex6(p.text_primary)));
    colors.insert("sidebar.border".into(), val(hex6(p.border)));
    colors.insert("sidebar.accent.background".into(), val(hex6(p.hover)));
    colors.insert(
        "sidebar.accent.foreground".into(),
        val(hex6(p.text_primary)),
    );

    // Scrollbars, window bars, tabs.
    colors.insert("scrollbar.background".into(), val(hex6(p.bg)));
    colors.insert("scrollbar.thumb.background".into(), val(hex6(p.border)));
    colors.insert(
        "scrollbar.thumb.hover.background".into(),
        val(hex6(p.text_muted)),
    );
    colors.insert("title_bar.background".into(), val(hex6(p.bg)));
    colors.insert("title_bar.border".into(), val(hex6(p.border)));
    colors.insert("status_bar.background".into(), val(hex6(p.panel)));
    colors.insert("status_bar.border".into(), val(hex6(p.border)));
    colors.insert("tab.background".into(), val(hex6(p.panel)));
    colors.insert("tab.active.background".into(), val(hex6(p.elevated)));
    colors.insert("tab.active.foreground".into(), val(hex6(p.text_primary)));
    colors.insert("tab.foreground".into(), val(hex6(p.text_secondary)));

    // Base palette (spinners, markers, status glyphs).
    colors.insert("base.red".into(), val(hex6(p.error)));
    colors.insert("base.green".into(), val(hex6(p.success)));
    colors.insert("base.blue".into(), val(hex6(p.accent)));
    colors.insert("base.yellow".into(), val(hex6(p.warning)));
    colors.insert("base.magenta".into(), val(hex6(opa(0xc678dd))));
    colors.insert("base.cyan".into(), val(hex6(opa(0x56b6c2))));

    colors
}

/// Keep a color but force an alpha byte (0x00..=0xff).
fn tinted(c: Rgba, alpha: u32) -> Rgba {
    rgba((rgb_u32(c) << 8) | alpha)
}

/// Opaque 0xRRGGBB from a color (alpha ignored).
fn rgb_u32(c: Rgba) -> u32 {
    ((c.r * 255.0).round() as u32) << 16
        | ((c.g * 255.0).round() as u32) << 8
        | (c.b * 255.0).round() as u32
}

fn hex6(c: Rgba) -> String {
    format!(
        "#{:02x}{:02x}{:02x}",
        (c.r * 255.0).round() as u8,
        (c.g * 255.0).round() as u8,
        (c.b * 255.0).round() as u8
    )
}

fn hex8(c: Rgba) -> String {
    format!(
        "#{:02x}{:02x}{:02x}{:02x}",
        (c.r * 255.0).round() as u8,
        (c.g * 255.0).round() as u8,
        (c.b * 255.0).round() as u8,
        (c.a * 255.0).round() as u8
    )
}

/// Approximate WCAG contrast ratio between two non-transparent colors.
#[cfg(test)]
fn contrast(a: Rgba, b: Rgba) -> f32 {
    let lum = |c: Rgba| {
        let f = |v: f32| {
            if v <= 0.03928 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * f(c.r) + 0.7152 * f(c.g) + 0.0722 * f(c.b)
    };
    let (hi, lo) = if lum(a) > lum(b) {
        (lum(a), lum(b))
    } else {
        (lum(b), lum(a))
    };
    (hi + 0.05) / (lo + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_and_light_palettes_are_distinct_surfaces() {
        let dark = Palette::dark();
        let light = Palette::light();

        // Layered surfaces must be distinguishable.
        assert_ne!(dark.bg, dark.panel);
        assert_ne!(dark.panel, dark.elevated);
        assert_ne!(dark.panel, dark.input);
        assert_ne!(light.bg, light.panel);
        assert_ne!(light.panel, light.elevated);
        assert_ne!(light.panel, dark.panel);
        // Modes differ.
        assert_ne!(dark.bg, light.bg);
        assert_ne!(dark.text_primary, light.text_primary);
        assert!(dark.dark);
        assert!(!light.dark);
    }

    #[test]
    fn interactive_states_keep_a_layered_hierarchy() {
        for p in [Palette::dark(), Palette::light()] {
            assert_ne!(p.hover, p.panel);
            assert_ne!(p.active, p.hover);
            assert_ne!(p.selected, p.hover);
            assert_ne!(p.accent_selected, p.selected);
        }
    }

    #[test]
    fn text_contrast_is_readable_in_both_modes() {
        // Primary ≥ AA normal; muted ≥ AA large/UI text on the app background.
        assert!(contrast(Palette::dark().text_primary, Palette::dark().bg) >= 7.0);
        assert!(contrast(Palette::dark().text_muted, Palette::dark().bg) >= 4.0);
        assert!(contrast(Palette::light().text_primary, Palette::light().bg) >= 7.0);
        assert!(contrast(Palette::light().text_muted, Palette::light().bg) >= 4.0);
    }

    #[test]
    fn from_theme_selects_mode_by_background_luminance() {
        use config::themes::Theme;

        let dark_theme = Theme {
            name: "Dark".into(),
            colors: [("background".to_string(), "#181a1f".to_string())]
                .into_iter()
                .collect(),
        };
        let light_theme = Theme {
            name: "Light".into(),
            colors: [("background".to_string(), "#f6f7f9".to_string())]
                .into_iter()
                .collect(),
        };
        assert!(Palette::from_theme(&dark_theme).dark);
        assert!(!Palette::from_theme(&light_theme).dark);
    }

    #[test]
    fn from_theme_falls_back_to_dark_without_background() {
        use config::themes::Theme;

        let empty = Theme {
            name: "Empty".into(),
            colors: Default::default(),
        };
        assert!(Palette::from_theme(&empty).dark);
    }

    #[test]
    fn component_color_values_parse_as_rgba() {
        for p in [Palette::dark(), Palette::light()] {
            for (key, value) in component_colors(&p) {
                let value = value.as_str().expect("colors are strings");
                // Round-trip through gpui's hex parser (supports #rrggbb and #rrggbbaa).
                let Ok(parsed) = Rgba::try_from(value) else {
                    panic!("{key} = {value} is not a parseable hex color");
                };
                assert!(parsed.a > 0.0 && parsed.a <= 1.0);
            }
            // The focus ring is intentionally translucent; everything else
            // opaque unless explicitly tinted.
            assert!(component_colors(&p)["ring"]
                .as_str()
                .and_then(|v| Rgba::try_from(v).ok())
                .is_some_and(|c| c.a < 1.0));
        }
    }

    #[test]
    fn translucent_tokens_keep_full_rgb() {
        let dark = Palette::dark();
        assert_eq!(dark.accent_selected.a, 0x16 as f32 / 255.0);
        assert_eq!(dark.diff_add_bg.a, 0x14 as f32 / 255.0);
        assert_eq!(dark.focus_ring.a, 0x66 as f32 / 255.0);
        // The rgb channels of the tint are preserved (not blended).
        assert_eq!(dark.accent_selected.r, dark.accent.r);
    }
}
