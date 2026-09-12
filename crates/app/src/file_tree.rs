//! File tree — status list with checkboxes for staging.

use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::checkbox::Checkbox;
use gpui_component::*;

use crate::git_service::GitService;

pub struct FileTree {
    git_service: Entity<GitService>,
    selected_files: Vec<String>,
    select_all: bool,
}

impl FileTree {
    pub fn new(git_service: Entity<GitService>, _cx: &mut Context<Self>) -> Self {
        Self {
            git_service,
            selected_files: Vec::new(),
            select_all: false,
        }
    }

    fn status_color(status: char) -> gpui::Rgba {
        match status {
            'M' => gpui::rgb(0xf0c83c), // Modified
            'A' => gpui::rgb(0x2da44e), // Added
            'D' => gpui::rgb(0xcf222e), // Deleted
            'R' => gpui::rgb(0x0969da), // Renamed
            '?' => gpui::rgb(0x8c959f), // Untracked
            _ => gpui::rgb(0xf8f9fa),   // Default
        }
    }

    fn status_label(status: char) -> &'static str {
        match status {
            'M' => "M",
            'A' => "A",
            'D' => "D",
            'R' => "R",
            '?' => "?",
            _ => " ",
        }
    }
}

impl Render for FileTree {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let files = self.git_service.read(cx).file_statuses.clone();
        let file_count = files.len();
        let all_paths: Vec<String> = files.iter().map(|f| f.path.clone()).collect();

        div()
            .flex()
            .flex_col()
            .size_full()
            .child(
                // Header
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .child(
                        Checkbox::new("select-all")
                            .checked(self.select_all)
                            .on_change(cx.listener(move |this, value, _, cx| {
                                this.select_all = *value;
                                if *value {
                                    this.selected_files = all_paths.clone();
                                } else {
                                    this.selected_files.clear();
                                }
                                cx.notify();
                            })),
                    )
                    .child(div().text_sm().font_bold().child("Changed Files"))
                    .child(div().flex_1())
                    .child(
                        div()
                            .text_sm()
                            .font_bold()
                            .child(format!("{} files", file_count)),
                    ),
            )
            .child(
                // File list
                div()
                    .flex_1()
                    .id("file-list")
                    .overflow_y_scroll()
                    .children(files.into_iter().map(|f| {
                        let is_selected = self.selected_files.contains(&f.path);
                        let path = f.path.clone();
                        let path2 = path.clone();
                        let status = f.status;

                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .px_3()
                            .py_1()
                            .id(format!("file-row-{}", f.path))
                            .cursor_pointer()
                            .hover(|this| this.bg(gpui::rgb(0x8c959f)))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if this.selected_files.contains(&path) {
                                    this.selected_files.retain(|p| p != &path);
                                } else {
                                    this.selected_files.push(path.clone());
                                }
                                cx.notify();
                            }))
                            .child(
                                // Status indicator
                                div()
                                    .w_5()
                                    .h_5()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded_sm()
                                    .bg(Self::status_color(status))
                                    .text_xs()
                                    .child(Self::status_label(status)),
                            )
                            .child(
                                // Checkbox
                                Checkbox::new(format!("file-{}", f.path))
                                    .checked(is_selected)
                                    .on_change(cx.listener(move |this, value, _, cx| {
                                        if *value {
                                            if !this.selected_files.contains(&path2) {
                                                this.selected_files.push(path2.clone());
                                            }
                                        } else {
                                            this.selected_files.retain(|p| p != &path2);
                                        }
                                        cx.notify();
                                    })),
                            )
                            .child(
                                // File path
                                div().text_sm().child(f.path.clone()),
                            )
                    })),
            )
            .child(
                // Action bar
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_2()
                    .border_t_1()
                    .child(
                        Button::new("stage-all")
                            .ghost()
                            .label("Stage All")
                            .on_click(cx.listener(|this, _, _, cx| {
                                let files: Vec<String> = this
                                    .git_service
                                    .read(cx)
                                    .file_statuses
                                    .iter()
                                    .map(|f| f.path.clone())
                                    .collect();
                                this.git_service.update(cx, |service, _| {
                                    let file_refs: Vec<&str> =
                                        files.iter().map(|s| s.as_str()).collect();
                                    let _ = service.stage_files(&file_refs);
                                });
                                this.git_service.update(cx, |service, _| {
                                    service.refresh_all();
                                });
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("unstage-all")
                            .ghost()
                            .label("Unstage All")
                            .disabled(self.selected_files.is_empty())
                            .on_click(cx.listener(|this, _, _, cx| {
                                let files: Vec<String> = this.selected_files.clone();
                                this.git_service.update(cx, |service, _| {
                                    let file_refs: Vec<&str> =
                                        files.iter().map(|s| s.as_str()).collect();
                                    let _ = service.unstage_files(&file_refs);
                                });
                                this.selected_files.clear();
                                this.git_service.update(cx, |service, _| {
                                    service.refresh_all();
                                });
                                cx.notify();
                            })),
                    ),
            )
    }
}
