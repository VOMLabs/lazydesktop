//! File tree — status list with checkboxes for staging.

use gpui::prelude::*;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::checkbox::Checkbox;
use gpui_component::*;

use crate::git_service::GitService;
use lazydesktop_app::theme::Palette;

/// Events emitted by the file tree.
#[derive(Clone, Debug)]
pub enum FileTreeEvent {
    /// User clicked a file row — the diff view should show this file.
    FileSelected(String),
}

impl EventEmitter<FileTreeEvent> for FileTree {}

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

    fn status_color(p: &Palette, status: char) -> gpui::Rgba {
        match status {
            'M' => p.status_modified,  // Modified
            'A' => p.status_added,     // Added
            'D' => p.status_deleted,   // Deleted
            'R' => p.status_renamed,   // Renamed
            '?' => p.status_untracked, // Untracked
            _ => p.text_primary,       // Default
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
        let p = Palette::current(cx);

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(p.panel)
            .child(
                // Header
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_4()
                    .py_2()
                    .border_b_1()
                    .border_color(p.separator)
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
                    .child(div().text_sm().font_medium().child("Changed Files"))
                    .child(div().flex_1())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .px_2()
                            .py_0p5()
                            .rounded_full()
                            .bg(p.elevated)
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(p.text_muted)
                                    .child(format!("{}", file_count)),
                            ),
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
                            .py_1_5()
                            .when(is_selected, |this| this.bg(p.selected))
                            .when(!is_selected, |this| this.hover(|this| this.bg(p.hover)))
                            .id(format!("file-row-{}", f.path))
                            .cursor_pointer()
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if this.selected_files.contains(&path) {
                                    this.selected_files.retain(|p| p != &path);
                                } else {
                                    this.selected_files.push(path.clone());
                                }
                                cx.emit(FileTreeEvent::FileSelected(path.clone()));
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
                                    .bg(Self::status_color(&p, status))
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
                    .px_4()
                    .py_2()
                    .border_t_1()
                    .border_color(p.separator)
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
