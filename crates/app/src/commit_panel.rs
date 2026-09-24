//! Commit panel — summary input, description, commit button, AI generation,
//! and working-tree actions (stash / reset).
//!
//! Mirrors the Qt commit container (`src/mainwindow.cpp`):
//! - "AI Generate" runs the configured provider (cloud chat APIs via
//!   `ai_core::cloud`, or local GGUF via `ai_core::commit_message`) and fills
//!   the summary (with an optional "Casual Description" body).
//! - "Description" generates a body-only description.
//! - Stash All / Stash Pop / Unstage All are working-tree utilities.
//!
//! Generation is dispatched onto tokio's blocking pool so the UI stays
//! responsive for both network calls and local model inference.

use std::sync::atomic::AtomicBool;

use ai_core::cloud::{self, CloudProvider};
use gpui::prelude::*;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::checkbox::Checkbox;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::*;

use crate::git_service::GitService;
use lazydesktop_app::theme::Palette;

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

/// The AI request kind, mirroring `AiRequestKind` in mainwindow.cpp.
#[derive(Clone, Copy, PartialEq, Eq)]
enum AiKind {
    /// Subject (summary) generation; may include a "Casual Description" body.
    Message,
    /// Body-only description generation.
    Description,
}

#[derive(Clone, Debug)]
enum AiProviderSpec {
    Local {
        model_path: String,
        gpu_layers: i32,
    },
    Cloud {
        provider: CloudProvider,
        model: String,
        api_key: String,
    },
}

pub struct CommitPanel {
    git_service: Entity<GitService>,
    summary_input: Entity<InputState>,
    description: String,
    #[allow(dead_code)] // Planned: co-author trailers
    co_authors: Vec<String>,
    skip_hooks: bool,
    /// True while an AI request is in flight.
    ai_busy: bool,
    /// Latest AI error to display under the inputs.
    ai_error: Option<String>,
    /// Generated summary awaiting a render to flush into the input.
    pending_summary: Option<String>,
    /// Generated description body awaiting a render to flush.
    pending_description: Option<String>,
}

impl CommitPanel {
    pub fn new(
        window: &mut Window,
        git_service: Entity<GitService>,
        cx: &mut Context<Self>,
    ) -> Self {
        let summary_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Summary (required)"));
        cx.subscribe(&summary_input, |_, _, _: &InputEvent, cx| cx.notify())
            .detach();
        Self {
            git_service,
            summary_input,
            description: String::new(),
            co_authors: Vec::new(),
            skip_hooks: false,
            ai_busy: false,
            ai_error: None,
            pending_summary: None,
            pending_description: None,
        }
    }

    /// Apply a generated message or description into the inputs.
    fn apply_ai_response(&mut self, kind: AiKind, text: &str, cx: &mut Context<Self>) {
        self.ai_busy = false;
        self.ai_error = None;

        if kind == AiKind::Description {
            self.pending_description = Some(text.trim().to_string());
            cx.notify();
            return;
        }

        let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
        if let Some(first) = lines.first() {
            let summary = first
                .trim_start_matches(|c: char| c == '#' || c.is_whitespace())
                .trim();
            self.pending_summary = Some(summary.to_string());
        }

        // Extract a "Casual Description" body if present (mirrors Qt).
        let mut desc_lines = Vec::new();
        let mut collecting = false;
        for line in &lines {
            if line.to_lowercase().contains("casual description") {
                collecting = true;
                continue;
            }
            if collecting && !line.trim().is_empty() {
                desc_lines.push(line.trim());
            }
        }
        if !desc_lines.is_empty() {
            self.pending_description = Some(desc_lines.join("\n"));
        }
        cx.notify();
    }

    /// Kick off an AI generation request for the given kind.
    fn request_ai_generation(&mut self, kind: AiKind, cx: &mut Context<Self>) {
        if self.ai_busy {
            return;
        }

        let settings =
            config::settings::Settings::load(&config::paths::settings_path()).unwrap_or_default();
        if !settings.get_bool("ai/enabled") {
            self.ai_error = Some("Enable AI in Settings → AI first.".to_string());
            cx.notify();
            return;
        }

        let provider_str = settings.get_or("ai/provider", "OpenRouter").to_string();
        let model = settings.get_or("ai/model", "gpt-4o-mini").to_string();
        let raw_prompt = if kind == AiKind::Description {
            settings
                .get_or("ai/description_system_prompt", DEFAULT_DESCRIPTION_PROMPT)
                .to_string()
        } else {
            settings
                .get_or("ai/system_prompt", DEFAULT_SYSTEM_PROMPT)
                .to_string()
        };

        let git_service = self.git_service.clone();
        let file_statuses = git_service.read(cx).file_statuses.clone();
        if file_statuses.is_empty() {
            self.ai_error = Some("No changes to generate a commit message for.".to_string());
            cx.notify();
            return;
        }

        let diff_text = git_service.read(cx).ai_diff_text();
        let (system_prompt, user_content) = build_ai_prompt(&raw_prompt, &diff_text);

        let provider_spec = if provider_str == "Local (internal llama.cpp)" {
            let model_path = settings.get_or("ai/local_model_path", "").to_string();
            if model_path.is_empty() {
                self.ai_error = Some(
                    "No local model selected. Go to Settings → AI to download and select a GGUF model."
                        .to_string(),
                );
                cx.notify();
                return;
            }
            let gpu = settings.get_bool("ai/gpu_acceleration");
            AiProviderSpec::Local {
                model_path,
                gpu_layers: if gpu { 99 } else { 0 },
            }
        } else {
            let api_key = settings.get_or("ai/api_key", "").to_string();
            if api_key.is_empty() {
                self.ai_error =
                    Some("No API key configured. Go to Settings → AI to add one.".to_string());
                cx.notify();
                return;
            }
            match CloudProvider::parse(&provider_str) {
                Some(p) => AiProviderSpec::Cloud {
                    provider: p,
                    model,
                    api_key,
                },
                None => {
                    self.ai_error = Some(format!("Unknown AI provider: {provider_str}"));
                    cx.notify();
                    return;
                }
            }
        };

        // Build local inference context (mirrors Qt's JSON context blob).
        let context = ai_core::commit_message::CommitContext {
            vcs: git_service.read(cx).vcs_kind().to_string(),
            repo_path: git_service.read(cx).repo_path.display().to_string(),
            diff: diff_text,
            files: file_statuses
                .iter()
                .map(|f| ai_core::commit_message::FileChange {
                    path: f.path.clone(),
                    status: f.status.to_string(),
                })
                .collect(),
            staged: file_statuses.iter().map(|f| f.path.clone()).collect(),
            branch: git_service.read(cx).current_branch.clone(),
            recent_messages: git_service.read(cx).recent_subjects(10),
            mode: if kind == AiKind::Description {
                "description".to_string()
            } else {
                "message".to_string()
            },
            system_prompt: Some(raw_prompt),
        };

        self.ai_busy = true;
        self.ai_error = None;
        cx.notify();

        cx.spawn(async move |this, cx| {
            let result = tokio::task::spawn_blocking(move || match provider_spec {
                AiProviderSpec::Local {
                    model_path,
                    gpu_layers,
                } => ai_core::commit_message::generate_commit_message(
                    &context,
                    &model_path,
                    gpu_layers,
                    &AtomicBool::new(false),
                    &|_| {},
                ),
                AiProviderSpec::Cloud {
                    provider,
                    model,
                    api_key,
                } => {
                    cloud::generate_cloud(provider, &model, &api_key, &system_prompt, &user_content)
                }
            })
            .await;

            let result = match result {
                Ok(r) => r,
                Err(e) => Err(format!("background task failed: {e}")),
            };
            let _ = this.update(cx, |this, cx| match result {
                Ok(text) => this.apply_ai_response(kind, &text, cx),
                Err(e) => {
                    this.ai_busy = false;
                    this.ai_error = Some(e);
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// Stash all working-tree changes, then refresh.
    fn stash_all(&mut self, cx: &mut Context<Self>) {
        self.git_service.update(cx, |service, _| {
            let _ = service.stash_push();
            service.refresh_all();
        });
        cx.notify();
    }

    /// Pop the most recent stash, then refresh.
    fn stash_pop(&mut self, cx: &mut Context<Self>) {
        self.git_service.update(cx, |service, _| {
            let _ = service.stash_pop();
            service.refresh_all();
        });
        cx.notify();
    }

    /// Reset the index (unstage everything), keeping working-tree changes.
    fn unstage_all(&mut self, cx: &mut Context<Self>) {
        self.git_service.update(cx, |service, _| {
            let _ = service.reset_mixed();
            service.refresh_all();
        });
        cx.notify();
    }
}

impl Render for CommitPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Flush AI results into the inputs (needs a window to set input state).
        if let Some(summary) = self.pending_summary.take() {
            self.summary_input
                .update(cx, |state, cx| state.set_value(&summary, window, cx));
        }
        if let Some(desc) = self.pending_description.take() {
            self.description = desc;
        }

        let file_count = self.git_service.read(cx).file_statuses.len();
        let is_dirty = self.git_service.read(cx).is_dirty;
        let summary = self.summary_input.read(cx).value();
        let ai_busy = self.ai_busy;
        let ai_error = self.ai_error.clone();
        let stash_count = self.git_service.read(cx).stash_count;
        let p = Palette::current(cx);

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(p.panel)
            .p_4()
            .gap_3()
            .child(
                // Row 1: summary + description inputs
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        // Summary input
                        div().flex_1().child(Input::new(&self.summary_input)),
                    )
                    .child(
                        // Description input (inset block, mirrors the input
                        // surface until a real Textarea is wired up)
                        div()
                            .flex_1()
                            .h(px(32.0))
                            .rounded_md()
                            .border_1()
                            .border_color(p.border)
                            .bg(p.input)
                            .px_2()
                            .flex()
                            .items_center()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(if self.description.is_empty() {
                                        p.text_muted
                                    } else {
                                        p.text_primary
                                    })
                                    .child(if self.description.is_empty() {
                                        "Description (optional)".to_string()
                                    } else {
                                        self.description.clone()
                                    }),
                            ),
                    ),
            )
            .child(
                // Row 2: options + working-tree actions + file count
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        Checkbox::new("skip-hooks")
                            .label("Skip hooks")
                            .checked(self.skip_hooks)
                            .on_change(cx.listener(|this, value, _, cx| {
                                this.skip_hooks = *value;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("ai-generate")
                            .ghost()
                            .label(if ai_busy {
                                "Generating…"
                            } else {
                                "AI Generate"
                            })
                            .disabled(ai_busy || !is_dirty)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.request_ai_generation(AiKind::Message, cx);
                            })),
                    )
                    .child(
                        Button::new("ai-description")
                            .ghost()
                            .label("Write Description")
                            .disabled(ai_busy || !is_dirty)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.request_ai_generation(AiKind::Description, cx);
                            })),
                    )
                    .child(
                        Button::new("stash-pop")
                            .ghost()
                            .label(format!("Stash Pop ({stash_count})"))
                            .disabled(stash_count == 0)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.stash_pop(cx);
                            })),
                    )
                    .child(
                        Button::new("stash-all")
                            .ghost()
                            .label("Stash All")
                            .disabled(!is_dirty)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.stash_all(cx);
                            })),
                    )
                    .child(
                        Button::new("unstage-all")
                            .ghost()
                            .label("Unstage All")
                            .disabled(!is_dirty)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.unstage_all(cx);
                            })),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .text_xs()
                            .text_color(p.text_muted)
                            .child(format!("{} files changed", file_count)),
                    )
                    .child(
                        // Primary commit action, anchored bottom-right.
                        Button::new("commit-btn")
                            .primary()
                            .label("Commit")
                            .disabled(summary.is_empty() || !is_dirty)
                            .on_click(cx.listener(|this, _, window, cx| {
                                let summary = this.summary_input.read(cx).value().to_string();
                                let description = this.description.clone();
                                let message = if description.is_empty() {
                                    summary.clone()
                                } else {
                                    format!("{}\n\n{}", summary, description)
                                };
                                this.git_service.update(cx, |service, _| {
                                    // Skip hooks mirrors the Qt checkbox on the
                                    // commit panel (git commit --no-verify).
                                    let _ = service.commit_with_hooks(&message, !this.skip_hooks);
                                });
                                this.summary_input
                                    .update(cx, |state, cx| state.set_value("", window, cx));
                                this.description.clear();
                                this.git_service.update(cx, |service, _| {
                                    service.refresh_all();
                                });
                                cx.notify();
                            })),
                    ),
            )
            .when_some(ai_error.clone(), |this, err| {
                this.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .text_sm()
                        .text_color(p.error)
                        .child(div().flex_1().child(err)),
                )
            })
    }
}

/// Build the system/user prompt pair from a raw prompt and diff text.
///
/// If the raw prompt contains the `<diff>` placeholder it is substituted
/// there and the user message is minimal; otherwise the diff is appended as
/// the user message (mirrors `buildAiPrompt` in mainwindow.cpp).
fn build_ai_prompt(raw_prompt: &str, diff_text: &str) -> (String, String) {
    if raw_prompt.contains("<diff>") {
        (
            raw_prompt.replace("<diff>", diff_text),
            "Generate the response now.".to_string(),
        )
    } else {
        (raw_prompt.to_string(), format!("Changes:\n{diff_text}"))
    }
}
