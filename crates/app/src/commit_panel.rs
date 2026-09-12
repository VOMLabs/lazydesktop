//! Commit panel — summary input, description, commit button.
//!
//! Rendered as a compact bottom strip below the file list and diff view.

use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::checkbox::Checkbox;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::*;

use crate::git_service::GitService;

pub struct CommitPanel {
    git_service: Entity<GitService>,
    summary_input: Entity<InputState>,
    description: String,
    #[allow(dead_code)] // Planned: co-author trailers
    co_authors: Vec<String>,
    skip_hooks: bool,
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
        }
    }
}

impl Render for CommitPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let file_count = self.git_service.read(cx).file_statuses.len();
        let is_dirty = self.git_service.read(cx).is_dirty;
        let summary = self.summary_input.read(cx).value();

        div()
            .flex()
            .flex_col()
            .size_full()
            .p_4()
            .gap_3()
            .child(
                // Row 1: summary + description + commit
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        // Summary input
                        div().flex_1().child(Input::new(&self.summary_input)),
                    )
                    .child(
                        // Description input (placeholder until Textarea is wired up)
                        div()
                            .flex_1()
                            .h(px(32.0))
                            .rounded_md()
                            .border_1()
                            .px_2()
                            .flex()
                            .items_center()
                            .child(div().text_sm().text_color(gpui::rgb(0x8c959f)).child(
                                if self.description.is_empty() {
                                    "Description (optional)".to_string()
                                } else {
                                    self.description.clone()
                                },
                            )),
                    )
                    .child(
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
                                    let _ = service.commit(&message);
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
            .child(
                // Row 2: options + file count
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
                        Button::new("co-authors")
                            .ghost()
                            .label("Co-authors")
                            .disabled(true), // TODO
                    )
                    .child(
                        Button::new("ai-generate")
                            .ghost()
                            .label("AI Generate")
                            .disabled(!is_dirty)
                            .on_click(cx.listener(|_this, _, _, _cx| {
                                // TODO: AI generation
                            })),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .text_sm()
                            .text_color(gpui::rgb(0x8c959f))
                            .child(format!("{} files changed", file_count)),
                    ),
            )
    }
}
