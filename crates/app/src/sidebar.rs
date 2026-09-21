//! Sidebar — branch selector, commit history, recent projects.

use gpui::prelude::*;
use gpui::*;
use gpui_component::*;

use crate::git_service::GitService;

/// Events emitted by the sidebar.
#[derive(Clone, Debug)]
pub enum SidebarEvent {
    /// User clicked a commit in the history — show its diff.
    CommitSelected(String),
    /// User clicked the Settings row.
    SettingsRequested,
}

impl EventEmitter<SidebarEvent> for Sidebar {}

pub struct Sidebar {
    git_service: Entity<GitService>,
    expanded_section: SidebarSection,
}

#[derive(Clone, Copy, PartialEq)]
enum SidebarSection {
    Branches,
    History,
    #[allow(dead_code)] // Planned: recent projects list
    Projects,
}

impl Sidebar {
    pub fn new(git_service: Entity<GitService>, _cx: &mut Context<Self>) -> Self {
        Self {
            git_service,
            expanded_section: SidebarSection::History,
        }
    }
}

impl Render for Sidebar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let branches = self.git_service.read(cx).branches.clone();
        let current = self.git_service.read(cx).current_branch.clone();
        let history = self.git_service.read(cx).commit_history.clone();

        div()
            .flex()
            .flex_col()
            .size_full()
            .p_3()
            .gap_4()
            .overflow_hidden()
            .child(
                // App title
                div()
                    .px_1()
                    .py_1()
                    .text_lg()
                    .font_bold()
                    .child("LazyDesktop"),
            )
            .child(
                // Branches section
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .id("branches-header")
                            .cursor_pointer()
                            .hover(|this| this.bg(gpui::rgb(0x8c959f)))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.expanded_section = SidebarSection::Branches;
                                cx.notify();
                            }))
                            .child(div().text_sm().font_bold().child("Branches"))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(gpui::rgb(0x8c959f))
                                    .child(format!("{}", branches.len())),
                            ),
                    )
                    .when(self.expanded_section == SidebarSection::Branches, |this| {
                        this.child(div().flex().flex_col().gap_0p5().children(
                            branches.into_iter().map(|b| {
                                let is_current = b.name == current;
                                let branch_name = b.name.clone();
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .px_2()
                                    .py_1_5()
                                    .rounded_md()
                                    .id(format!("branch-{}", b.name))
                                    .when(is_current, |this| this.bg(gpui::rgb(0x0969da)))
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.git_service.update(cx, |service, _| {
                                            let _ = service.checkout(&branch_name);
                                        });
                                    }))
                                    .child(div().text_sm().child(b.name.clone()))
                            }),
                        ))
                    }),
            )
            .child(
                // History section
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .id("history-header")
                            .cursor_pointer()
                            .hover(|this| this.bg(gpui::rgb(0x8c959f)))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.expanded_section = SidebarSection::History;
                                cx.notify();
                            }))
                            .child(div().text_sm().font_bold().child("History"))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(gpui::rgb(0x8c959f))
                                    .child(format!("{}", history.len())),
                            ),
                    )
                    .when(self.expanded_section == SidebarSection::History, |this| {
                        this.child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_0p5()
                                .id("history-list")
                                .overflow_y_scroll()
                                .children(history.into_iter().map(|c| {
                                    let hash = c.hash.clone();
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap_0p5()
                                        .px_2()
                                        .py_1_5()
                                        .rounded_md()
                                        .id(format!("commit-{hash}"))
                                        .cursor_pointer()
                                        .hover(|this| this.bg(gpui::rgb(0x8c959f)))
                                        .on_click(cx.listener(move |_this, _, _, cx| {
                                            cx.emit(SidebarEvent::CommitSelected(hash.clone()));
                                            cx.notify();
                                        }))
                                        .child(
                                            // Hash — monospace, small
                                            div()
                                                .text_xs()
                                                .font_family("monospace")
                                                .text_color(gpui::rgb(0x8c959f))
                                                .child(c.hash.clone()),
                                        )
                                        .child(
                                            // Subject — normal
                                            div().text_sm().child(c.subject.clone()),
                                        )
                                })),
                        )
                    }),
            )
            .child(
                // Settings (pinned to the bottom of the sidebar)
                div()
                    .mt_auto()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .py_1_5()
                    .rounded_md()
                    .id("settings-row")
                    .cursor_pointer()
                    .hover(|this| this.bg(gpui::rgb(0x8c959f)))
                    .on_click(cx.listener(|_this, _, _, cx| {
                        cx.emit(SidebarEvent::SettingsRequested);
                        cx.notify();
                    }))
                    .child(div().text_sm().font_bold().child("Settings")),
            )
    }
}
