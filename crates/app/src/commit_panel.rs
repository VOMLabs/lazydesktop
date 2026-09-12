//! Commit panel — summary input, description, commit button.

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
                // Header
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_lg().font_bold().child("Commit"))
                    .child(
                        div()
                            .text_sm()
                            .child(format!("{} files changed", file_count)),
                    ),
            )
            .child(
                // Summary input
                Input::new(&self.summary_input),
            )
            .child(
                // Description textarea
                div()
                    .flex_1()
                    .min_h(px(100.0))
                    .rounded_md()
                    .border_1()
                    .p_2()
                    .child(div().text_sm().child(if self.description.is_empty() {
                        "Description (optional)".to_string()
                    } else {
                        self.description.clone()
                    })),
            )
            .child(
                // Options row
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
                    ),
            )
            .child(
                // Action buttons
                div()
                    .flex()
                    .items_center()
                    .gap_2()
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
    }
}
