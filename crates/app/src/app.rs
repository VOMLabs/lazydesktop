//! Main application view — toolbar + sidebar + content split layout.

use gpui::prelude::*;
use gpui::*;
use gpui_component::button::Button;
use gpui_component::*;

use crate::commit_panel::CommitPanel;
use crate::diff_view::DiffView;
use crate::file_tree::{FileTree, FileTreeEvent};
use crate::git_service::GitService;
use crate::sidebar::Sidebar;

pub struct LazyDesktopApp {
    sidebar: Entity<Sidebar>,
    file_tree: Entity<FileTree>,
    diff_view: Entity<DiffView>,
    commit_panel: Entity<CommitPanel>,
    git_service: Entity<GitService>,
}

impl LazyDesktopApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let git_service = cx.new(|_| GitService::new(std::path::PathBuf::new()));
        let sidebar = cx.new(|cx| Sidebar::new(git_service.clone(), cx));
        let file_tree = cx.new(|cx| FileTree::new(git_service.clone(), cx));
        let diff_view = cx.new(|cx| DiffView::new(git_service.clone(), cx));
        let commit_panel = cx.new(|cx| CommitPanel::new(window, git_service.clone(), cx));

        // Selecting a file in the tree shows its diff.
        cx.subscribe(&file_tree, |this, _emitter, event: &FileTreeEvent, cx| {
            let FileTreeEvent::FileSelected(path) = event;
            this.diff_view
                .update(cx, |view, cx| view.set_file(path.clone(), cx));
        })
        .detach();

        Self {
            sidebar,
            file_tree,
            diff_view,
            commit_panel,
            git_service,
        }
    }

    /// Open a repository. Wired up once the file dialog is implemented.
    #[allow(dead_code)]
    pub fn open_repo(&mut self, path: std::path::PathBuf, cx: &mut Context<Self>) {
        self.git_service.update(cx, |service, _| {
            *service = GitService::new(path);
            service.refresh_all();
        });
        cx.notify();
    }
}

impl Render for LazyDesktopApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .child(self.render_toolbar(cx))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .child(
                        // Sidebar
                        div()
                            .w(px(260.0))
                            .h_full()
                            .border_r_1()
                            .child(self.sidebar.clone()),
                    )
                    .child(
                        // Main content: file tree + diff view on top,
                        // commit panel at the bottom
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .flex()
                                    .flex_1()
                                    .child(
                                        // File tree (left)
                                        div()
                                            .w(px(320.0))
                                            .h_full()
                                            .border_r_1()
                                            .child(self.file_tree.clone()),
                                    )
                                    .child(
                                        // Diff view (center)
                                        div().flex_1().h_full().child(self.diff_view.clone()),
                                    ),
                            )
                            .child(
                                // Commit panel (bottom)
                                div().border_t_1().child(self.commit_panel.clone()),
                            ),
                    ),
            )
            .children(Root::render_dialog_layer(_window, cx))
            .children(Root::render_sheet_layer(_window, cx))
            .children(Root::render_notification_layer(_window, cx))
    }
}

impl LazyDesktopApp {
    fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let branch = self.git_service.read(cx).current_branch.clone();
        let is_dirty = self.git_service.read(cx).is_dirty;

        div()
            .flex()
            .items_center()
            .justify_between()
            .px_4()
            .py_2()
            .border_b_1()
            .child(
                // Left: repo actions
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        Button::new("open-folder")
                            .label("Open Folder")
                            .on_click(cx.listener(|_this, _, _window, _cx| {
                                // TODO: file dialog
                            })),
                    )
                    .child(
                        // Branch badge
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .bg(gpui::rgb(0x2d333b))
                            .child(div().text_sm().font_bold().child(branch.clone()))
                            .when(is_dirty, |this| {
                                this.child(div().w_2().h_2().rounded_full().bg(gpui::rgb(0xcf222e)))
                            }),
                    ),
            )
            .child(
                // Right: git actions
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        Button::new("push")
                            .label("Push")
                            .disabled(!is_dirty)
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.git_service.update(cx, |service, _| {
                                    let _ = service.push();
                                });
                            })),
                    )
                    .child(Button::new("pull").label("Pull").on_click(cx.listener(
                        |this, _, _window, cx| {
                            this.git_service.update(cx, |service, _| {
                                let _ = service.pull();
                            });
                        },
                    ))),
            )
    }
}
