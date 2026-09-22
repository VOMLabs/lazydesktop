//! Settings view — appearance, git identity, AI providers, SSH keys,
//! remotes, data locations.
//!
//! Mirrors the Qt settings dialog (`src/mainwindow.cpp`):
//! - Appearance: theme selection, persisted under `appearance/theme` in the
//!   INI settings file (QSettings-compatible).
//! - Git Identity: `user.name` / `user.email` read from and saved to the
//!   global git config.
//! - AI: enable toggle, provider, model, API key, local GGUF model path, GPU
//!   acceleration — persisted under `ai/*`.
//! - SSH Keys: list/generate SSH keypairs via `vcs_core::ssh`.
//! - Remotes: list/add/remove git remotes via `vcs_core::remote`.
//! - Data Locations: read-only display of config paths.
//!
//! Theme changes emit [`SettingsEvent::ThemeChanged`] so the app root can
//! apply the chosen colors (background/foreground).

use std::path::PathBuf;

use gpui::prelude::*;
use gpui::*;
use gpui_component::button::Button;
use gpui_component::checkbox::Checkbox;
use gpui_component::input::{Input, InputState};
use gpui_component::*;

use config::paths::{settings_path, themes_dir};
use config::settings::Settings;
use config::themes::scan_themes;
use git_cmd::git;
use lazydesktop_app::theme::Palette;
use vcs_core::remote::{add_remote, list_remotes, remove_remote, Remote};
use vcs_core::ssh::{fingerprint_public_key, generate_key, list_public_keys, ssh_keys_dir};

use crate::git_service::GitService;

// Mirrors `kDefaultSystemPrompt` in mainwindow.cpp.
const DEFAULT_SYSTEM_PROMPT: &str = "You are a CLI tool that outputs exactly ONE single conventional commit message summarizing the entire diff.\n\
Do not write a separate commit for each file. Find the highest-level feature or fix and summarize it in one \
line. Do not explain.\n\
\n\
Diff:\n\
--- a/package.json\n\
+++ b/package.json\n\
@@ -10 +10,2 @@\n\
+ \"cors\": \"^2.8.5\"\n\
--- a/src/server.js\n\
+++ b/src/server.js\n\
@@ -2 +2,3 @@\n\
+ const cors = require('cors');\n\
+ app.use(cors());\n\
\n\
Commit: feat(api): add cors support to server\n\
\n\
Diff:\n\
<diff>\n\
\n\
Commit:";

// Mirrors `kDefaultDescriptionSystemPrompt` in mainwindow.cpp.
const DEFAULT_DESCRIPTION_PROMPT: &str =
    "Write a casual, plain-language description of the changes below. \
Explain what changed and why in a few sentences. Do not include a summary title line.\n\
\n\
Diff:\n\
<diff>\n\
\n\
Description:";

/// AI providers offered by the picker, in display order. The first entry is
/// the Qt default.
const AI_PROVIDERS: &[&str] = &[
    "OpenRouter",
    "OpenAI",
    "Anthropic",
    "Google AI Studio",
    "Local (internal llama.cpp)",
];

/// Events emitted by the settings view.
#[derive(Clone, Debug)]
pub enum SettingsEvent {
    /// The theme changed. `None` means "System Default" (no override); `Some`
    /// carries the resolved palette to apply app-wide.
    ThemeChanged(Option<Palette>),
}

impl EventEmitter<SettingsEvent> for SettingsView {}

pub struct SettingsView {
    git_service: Entity<GitService>,
    // Appearance
    selected_theme: String,
    theme_options: Vec<(String, String)>,
    // Git identity
    user_name: Entity<InputState>,
    user_email: Entity<InputState>,
    identity_status: Option<String>,
    // AI
    ai_enabled: bool,
    ai_provider: String,
    ai_model: Entity<InputState>,
    ai_api_key: Entity<InputState>,
    ai_local_model_path: Entity<InputState>,
    ai_gpu_accel: bool,
    ai_status: Option<String>,
    // SSH keys
    ssh_keys: Vec<(PathBuf, String)>,
    ssh_comment: Entity<InputState>,
    ssh_status: Option<String>,
    // Remotes
    remotes: Vec<Remote>,
    remote_name: Entity<InputState>,
    remote_url: Entity<InputState>,
    remote_status: Option<String>,
}

impl SettingsView {
    pub fn new(
        git_service: Entity<GitService>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let settings = Settings::load(&settings_path()).unwrap_or_default();
        let selected_theme = settings.get_or("appearance/theme", "system").to_string();

        // Offer system default, both builtin themes, then custom themes on disk.
        let mut theme_options = vec![
            ("system".to_string(), "System".to_string()),
            ("Dark".to_string(), "Dark".to_string()),
            ("Light".to_string(), "Light".to_string()),
        ];
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

        let ai_enabled = settings.get_bool("ai/enabled");
        let ai_provider = settings.get_or("ai/provider", "OpenRouter").to_string();
        let ai_model = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("gpt-4o-mini")
                .default_value(settings.get_or("ai/model", "gpt-4o-mini"))
        });
        let ai_api_key = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("sk-…")
                .default_value(settings.get_or("ai/api_key", ""))
        });
        let ai_local_model_path = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("/path/to/model.gguf")
                .default_value(settings.get_or("ai/local_model_path", ""))
        });
        let ai_gpu_accel = settings.get_bool("ai/gpu_acceleration");

        let ssh_comment = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("you@example.com")
                .default_value(git::config_get_global("user.email").unwrap_or_default())
        });
        let remote_name = cx.new(|cx| InputState::new(window, cx).placeholder("origin"));
        let remote_url =
            cx.new(|cx| InputState::new(window, cx).placeholder("git@github.com:user/repo.git"));

        let mut view = Self {
            git_service,
            selected_theme,
            theme_options,
            user_name,
            user_email,
            identity_status: None,
            ai_enabled,
            ai_provider,
            ai_model,
            ai_api_key,
            ai_local_model_path,
            ai_gpu_accel,
            ai_status: None,
            ssh_keys: Vec::new(),
            ssh_comment,
            ssh_status: None,
            remotes: Vec::new(),
            remote_name,
            remote_url,
            remote_status: None,
        };
        view.refresh_ssh(cx);
        view.refresh_remotes(cx);
        view
    }

    /// Persist the chosen theme and emit the resolved palette.
    fn select_theme(&mut self, value: &str, cx: &mut Context<Self>) {
        self.selected_theme = value.to_string();

        let mut settings = Settings::load(&settings_path()).unwrap_or_default();
        settings.set("appearance/theme", value);
        let _ = settings.save(); // non-fatal on failure

        cx.emit(SettingsEvent::ThemeChanged(self.theme_palette(value)));
        cx.notify();
    }

    /// Select and persist the AI provider.
    fn select_provider(&mut self, value: &str, cx: &mut Context<Self>) {
        self.ai_provider = value.to_string();
        let mut settings = Settings::load(&settings_path()).unwrap_or_default();
        settings.set("ai/provider", value);
        let _ = settings.save();
        cx.notify();
    }

    /// Resolve a theme value to a palette, or `None` for the system default.
    fn theme_palette(&self, value: &str) -> Option<Palette> {
        match value {
            "system" => None,
            "Dark" => Some(Palette::dark()),
            "Light" => Some(Palette::light()),
            other => scan_themes(&themes_dir())
                .iter()
                .find(|e| e.name == other)
                .and_then(|e| config::themes::Theme::load(&e.path).ok())
                .map(|theme| Palette::from_theme(&theme)),
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

    /// Save the AI settings block (provider, model, key, local model, GPU).
    fn save_ai(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let mut settings = Settings::load(&settings_path()).unwrap_or_default();

        settings.set_bool("ai/enabled", self.ai_enabled);
        settings.set("ai/provider", &self.ai_provider);

        let api_key = self.ai_api_key.read(cx).value().to_string();
        settings.set("ai/api_key", api_key.trim());

        let mut model = self.ai_model.read(cx).value().to_string();
        if model.trim().is_empty() && self.ai_provider == "OpenRouter" {
            model = "deepseek/deepseek-v4-flash".to_string();
        }
        settings.set("ai/model", model.trim());

        let local_path = self.ai_local_model_path.read(cx).value().to_string();
        settings.set("ai/local_model_path", local_path.trim());

        settings.set_bool("ai/gpu_acceleration", self.ai_gpu_accel);

        // Seed default prompts on first save so the commit panel can rely on
        // stored values even before the (not-yet-implemented) prompt editor.
        if settings.get("ai/system_prompt").is_none_or(str::is_empty) {
            settings.set("ai/system_prompt", DEFAULT_SYSTEM_PROMPT);
        }
        if settings
            .get("ai/description_system_prompt")
            .is_none_or(str::is_empty)
        {
            settings.set("ai/description_system_prompt", DEFAULT_DESCRIPTION_PROMPT);
        }

        let ok = settings.save().is_ok();
        self.ai_status = Some(if ok {
            "Saved ✓".to_string()
        } else {
            "Failed to save settings".to_string()
        });
        cx.notify();
    }

    /// List SSH public keys and their fingerprints.
    fn refresh_ssh(&mut self, cx: &App) {
        self.ssh_keys = match list_public_keys() {
            Ok(paths) => paths
                .into_iter()
                .filter_map(|p| fingerprint_public_key(&p).ok().map(|fp| (p, fp)))
                .collect(),
            Err(e) => {
                self.ssh_status = Some(format!("Failed to list keys: {e}"));
                Vec::new()
            }
        };
        let _ = cx;
    }

    /// Generate an ed25519 SSH keypair in the default SSH directory.
    fn generate_ssh_key(&mut self, cx: &mut Context<Self>) {
        let comment = self.ssh_comment.read(cx).value().to_string();
        let path = ssh_keys_dir().join("id_ed25519");
        if path.exists() {
            self.ssh_status = Some("~/.ssh/id_ed25519 already exists".to_string());
            cx.notify();
            return;
        }
        match generate_key("ed25519", &path, &comment, None) {
            Ok(()) => self.ssh_status = Some("Generated ✓".to_string()),
            Err(e) => self.ssh_status = Some(format!("Failed to generate key: {e}")),
        }
        self.refresh_ssh(cx);
        cx.notify();
    }

    /// List remotes for the open repository.
    fn refresh_remotes(&mut self, cx: &App) {
        let path = self.git_service.read(cx).repo_path.clone();
        if path.as_os_str().is_empty() {
            self.remotes.clear();
            return;
        }
        self.remotes = list_remotes(&path).unwrap_or_default();
    }

    /// Add a remote to the open repository.
    fn add_remote(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.remote_name.read(cx).value().to_string();
        let url = self.remote_url.read(cx).value().to_string();
        let path = self.git_service.read(cx).repo_path.clone();
        if name.trim().is_empty() || url.trim().is_empty() || path.as_os_str().is_empty() {
            self.remote_status = Some("Enter a name and URL (repo must be open)".to_string());
            cx.notify();
            return;
        }
        match add_remote(&path, name.trim(), url.trim()) {
            Ok(()) => {
                self.remote_name
                    .update(cx, |state, cx| state.set_value("", window, cx));
                self.remote_url
                    .update(cx, |state, cx| state.set_value("", window, cx));
                self.remote_status = Some("Added ✓".to_string());
            }
            Err(e) => self.remote_status = Some(format!("Failed to add remote: {e}")),
        }
        self.refresh_remotes(cx);
        cx.notify();
    }

    /// Remove a remote from the open repository.
    fn remove_remote(&mut self, name: &str, cx: &mut Context<Self>) {
        let path = self.git_service.read(cx).repo_path.clone();
        match remove_remote(&path, name) {
            Ok(()) => self.remote_status = Some("Removed ✓".to_string()),
            Err(e) => self.remote_status = Some(format!("Failed to remove remote: {e}")),
        }
        self.refresh_remotes(cx);
        cx.notify();
    }
}

impl Render for SettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selected_theme = self.selected_theme.clone();
        let theme_options = self.theme_options.clone();
        let identity_status = self.identity_status.clone();
        let ai_provider = self.ai_provider.clone();
        let ai_enabled = self.ai_enabled;
        let ai_status = self.ai_status.clone();
        let ssh_keys = self.ssh_keys.clone();
        let ssh_status = self.ssh_status.clone();
        let remotes = self.remotes.clone();
        let remote_status = self.remote_status.clone();
        let has_repo = !self.git_service.read(cx).repo_path.as_os_str().is_empty();
        let p = Palette::current(cx);

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
                div()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .children(theme_options.into_iter().map(|(value, display)| {
                        let is_selected = value == selected_theme;
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .when(is_selected, |this| this.bg(p.accent_selected))
                            .id(format!("theme-{value}"))
                            .cursor_pointer()
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.select_theme(&value, cx);
                            }))
                            .child(div().text_sm().child(display))
                            .child(div().flex_1())
                            .child(div().text_sm().child(if is_selected { "●" } else { "" }))
                    })),
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
                            .child(status_text(
                                &identity_status,
                                "Stored in global git config",
                                &p,
                            )),
                    ),
            )
            .child(div().h_6())
            // ── AI ──────────────────────────────────────────────────────
            .child(section_header("AI"))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .max_w(px(420.0))
                    .child(
                        Checkbox::new("ai-enabled")
                            .label("Enable AI Features")
                            .checked(ai_enabled)
                            .on_change(cx.listener(|this, value, _, cx| {
                                this.ai_enabled = *value;
                                cx.notify();
                            })),
                    )
                    .child(div().text_sm().text_color(p.text_muted).child("Provider:"))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .children(AI_PROVIDERS.iter().map(|&provider| {
                                let is_selected = provider == ai_provider;
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_3()
                                    .px_3()
                                    .py_1_5()
                                    .rounded_md()
                                    .when(is_selected, |this| this.bg(p.accent_selected))
                                    .id(format!("ai-provider-{provider}"))
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.select_provider(provider, cx);
                                    }))
                                    .child(div().text_sm().child(provider))
                                    .child(div().flex_1())
                                    .child(div().text_sm().child(if is_selected {
                                        "●"
                                    } else {
                                        ""
                                    }))
                            })),
                    )
                    .child(labelled_input("Model:", self.ai_model.clone()))
                    .child(labelled_input("API Key:", self.ai_api_key.clone()))
                    .child(labelled_input(
                        "Local GGUF Path:",
                        self.ai_local_model_path.clone(),
                    ))
                    .child(
                        Checkbox::new("ai-gpu")
                            .label("GPU acceleration (local inference)")
                            .checked(self.ai_gpu_accel)
                            .on_change(cx.listener(|this, value, _, cx| {
                                this.ai_gpu_accel = *value;
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(Button::new("save-ai").label("Save").on_click(cx.listener(
                                |this, _, window, cx| {
                                    this.save_ai(window, cx);
                                },
                            )))
                            .child(status_text(&ai_status, "Cloud + local GGUF", &p)),
                    ),
            )
            .child(div().h_6())
            // ── SSH keys ────────────────────────────────────────────────
            .child(section_header("SSH Keys"))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .max_w(px(420.0))
                    .child(labelled_input("Key comment:", self.ssh_comment.clone()))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                Button::new("gen-ssh")
                                    .label("Generate ed25519 key")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.generate_ssh_key(cx);
                                    })),
                            )
                            .child(status_text(&ssh_status, "Keys live in ~/.ssh", &p)),
                    )
                    .child(div().h_1())
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .children(ssh_keys.into_iter().map(|(path, fp)| {
                                let name = path
                                    .file_name()
                                    .map(|n| n.to_string_lossy().into_owned())
                                    .unwrap_or_default();
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_3()
                                    .px_2()
                                    .py_1()
                                    .rounded_md()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_family("monospace")
                                            .child(name.clone()),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(p.text_muted)
                                            .font_family("monospace")
                                            .child(fp),
                                    )
                            })),
                    ),
            )
            .child(div().h_6())
            // ── Remotes ─────────────────────────────────────────────────
            .child(section_header("Remotes"))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .max_w(px(560.0))
                    .when(!has_repo, |this| {
                        this.child(
                            div()
                                .text_sm()
                                .text_color(p.text_muted)
                                .child("Open a repository to manage its remotes."),
                        )
                    })
                    .child(labelled_input("Name:", self.remote_name.clone()))
                    .child(labelled_input("URL:", self.remote_url.clone()))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(Button::new("add-remote").label("Add Remote").on_click(
                                cx.listener(|this, _, window, cx| {
                                    this.add_remote(window, cx);
                                }),
                            ))
                            .child(status_text(&remote_status, "", &p)),
                    )
                    .child(div().h_1())
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .children(remotes.into_iter().map(|r| {
                                let name = r.name.clone();
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_3()
                                    .px_2()
                                    .py_1()
                                    .rounded_md()
                                    .child(div().text_sm().font_bold().child(r.name.clone()))
                                    .child(
                                        div()
                                            .flex_1()
                                            .text_sm()
                                            .text_color(p.text_muted)
                                            .font_family("monospace")
                                            .child(r.url.clone()),
                                    )
                                    .child(
                                        div()
                                            .id(format!("remote-del-{name}"))
                                            .cursor_pointer()
                                            .hover(|this| this.text_color(p.error))
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.remove_remote(&name, cx);
                                            }))
                                            .child(div().text_xs().child("✕")),
                                    )
                            })),
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
                        &p,
                    ))
                    .child(path_row("Themes:", &themes_dir().display().to_string(), &p))
                    .child(path_row(
                        "Projects:",
                        &config::paths::projects_path().display().to_string(),
                        &p,
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

fn status_text(status: &Option<String>, default: &str, p: &Palette) -> Div {
    div()
        .text_sm()
        .text_color(match status {
            Some(ref s) if s.starts_with("Saved") || s.contains('✓') => p.success,
            Some(_) => p.error,
            None => p.text_muted,
        })
        .child(status.clone().unwrap_or_else(|| default.to_string()))
}

fn path_row(label: impl Into<gpui::SharedString>, value: &str, p: &Palette) -> Div {
    div()
        .flex()
        .items_center()
        .gap_3()
        .child(
            div()
                .text_sm()
                .text_color(p.text_muted)
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
