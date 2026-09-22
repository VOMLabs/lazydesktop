//! Main application view — toolbar + sidebar + content split layout.

use gpui::prelude::*;
use gpui::*;
use gpui_component::button::Button;
use gpui_component::*;

use crate::commit_panel::CommitPanel;
use crate::diff_view::DiffView;
use crate::file_tree::{FileTree, FileTreeEvent};
use crate::git_service::GitService;
use crate::settings_view::{SettingsEvent, SettingsView};
use crate::sidebar::{Sidebar, SidebarEvent};
use lazydesktop_app::theme::Palette;

/// Which main content area is shown in the right pane.
#[derive(Clone, Copy, PartialEq)]
enum MainView {
    /// Working tree: file tree + diff viewer + commit panel.
    WorkingTree,
    /// Application settings.
    Settings,
}

pub struct LazyDesktopApp {
    sidebar: Entity<Sidebar>,
    file_tree: Entity<FileTree>,
    diff_view: Entity<DiffView>,
    commit_panel: Entity<CommitPanel>,
    settings_view: Entity<SettingsView>,
    git_service: Entity<GitService>,
    view: MainView,
    /// Latest toolbar operation result: (message, is_error).
    op_feedback: Option<(String, bool)>,
}

impl LazyDesktopApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let git_service = cx.new(|_| GitService::new(std::path::PathBuf::new()));
        let sidebar = cx.new(|cx| Sidebar::new(window, git_service.clone(), cx));
        let file_tree = cx.new(|cx| FileTree::new(git_service.clone(), cx));
        let diff_view = cx.new(|cx| DiffView::new(git_service.clone(), cx));
        let commit_panel = cx.new(|cx| CommitPanel::new(window, git_service.clone(), cx));
        let settings_view = cx.new(|cx| SettingsView::new(git_service.clone(), window, cx));

        // Selecting a file in the tree shows its diff.
        cx.subscribe(&file_tree, |this, _emitter, event: &FileTreeEvent, cx| {
            let FileTreeEvent::FileSelected(path) = event;
            this.diff_view
                .update(cx, |view, cx| view.set_file(path.clone(), cx));
        })
        .detach();

        // Sidebar: history click → commit diff; settings row → settings view;
        // repo open request → swap GitService.
        cx.subscribe(&sidebar, |this, _emitter, event: &SidebarEvent, cx| {
            match event {
                SidebarEvent::CommitSelected(hash) => {
                    this.view = MainView::WorkingTree;
                    this.diff_view
                        .update(cx, |view, cx| view.set_commit(hash.clone(), cx));
                }
                SidebarEvent::SettingsRequested => {
                    this.view = MainView::Settings;
                }
                SidebarEvent::OpenRepoRequested(path) => {
                    this.open_repo(path.clone(), cx);
                    this.sidebar.update(cx, |sidebar, _| {
                        sidebar.refresh_recent_projects();
                    });
                }
            }
            cx.notify();
        })
        .detach();

        // Settings: apply the selected theme across the app and the
        // component kit.
        cx.subscribe(
            &settings_view,
            |_this, _emitter, event: &SettingsEvent, cx| {
                let SettingsEvent::ThemeChanged(palette) = event;
                let palette = palette.unwrap_or_else(|| match cx.window_appearance() {
                    gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark => {
                        Palette::dark()
                    }
                    _ => Palette::light(),
                });
                Palette::install(palette, cx);
                cx.notify();
            },
        )
        .detach();

        Self {
            sidebar,
            file_tree,
            diff_view,
            commit_panel,
            settings_view,
            git_service,
            view: MainView::WorkingTree,
            op_feedback: None,
        }
    }

    /// Open a repository. Recorded in the recent-projects list.
    pub fn open_repo(&mut self, path: std::path::PathBuf, cx: &mut Context<Self>) {
        if path.exists() {
            let mut recent =
                config::projects::RecentProjects::load(&config::paths::projects_path());
            recent.add(&path.display().to_string());
            let _ = recent.save(&config::paths::projects_path());
        }
        self.git_service.update(cx, |service, _| {
            *service = GitService::new(path);
            service.refresh_all();
        });
        self.view = MainView::WorkingTree;
        self.op_feedback = None;
        cx.notify();
    }

    /// Run a toolbar git operation (push / pull / fetch), refresh state, and
    /// surface the result as feedback text.
    fn run_git_op(
        &mut self,
        label: &str,
        op: fn(&GitService) -> Result<git_cmd::types::CommandResult, git_cmd::types::VcsError>,
        cx: &mut Context<Self>,
    ) {
        let result = self.git_service.update(cx, |service, _| {
            let r = op(service);
            service.refresh_all();
            r
        });
        self.op_feedback = Some(match &result {
            Ok(r) if r.success() => (format!("{label}: OK"), false),
            Ok(r) => (format!("{label} failed: {}", r.stderr.trim()), true),
            Err(e) => (format!("{label} error: {e}"), true),
        });
        cx.notify();
    }
}

impl Render for LazyDesktopApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = Palette::current(cx);
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(palette.bg)
            .text_color(palette.text_primary)
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
                            .border_color(palette.separator)
                            .child(self.sidebar.clone()),
                    )
                    .child(match self.view {
                        MainView::WorkingTree => {
                            let content: AnyElement = div()
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
                                                .border_color(palette.separator)
                                                .child(self.file_tree.clone()),
                                        )
                                        .child(
                                            // Diff view (center)
                                            div().flex_1().h_full().child(self.diff_view.clone()),
                                        ),
                                )
                                .child(
                                    // Commit panel (bottom)
                                    div()
                                        .border_t_1()
                                        .border_color(palette.separator)
                                        .child(self.commit_panel.clone()),
                                )
                                .into_any();
                            content
                        }
                        MainView::Settings => div()
                            .flex_1()
                            .h_full()
                            .child(self.settings_view.clone())
                            .into_any(),
                    }),
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
        let op_feedback = self.op_feedback.clone();
        let palette = Palette::current(cx);

        div()
            .flex()
            .flex_col()
            .bg(palette.panel)
            .border_b_1()
            .border_color(palette.separator)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .py_2()
                    .child(
                        // Left: repo actions
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                // Branch badge
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .px_3()
                                    .py_1()
                                    .rounded_md()
                                    .bg(palette.elevated)
                                    .text_color(palette.text_primary)
                                    .child(div().text_sm().font_bold().child(branch.clone()))
                                    .when(is_dirty, |this| {
                                        this.child(
                                            div().w_2().h_2().rounded_full().bg(palette.warning),
                                        )
                                    }),
                            )
                            .when_some(op_feedback.clone(), |this, (msg, is_err)| {
                                this.child(
                                    div()
                                        .text_sm()
                                        .text_color(if is_err {
                                            palette.error
                                        } else {
                                            palette.success
                                        })
                                        .child(msg),
                                )
                            }),
                    )
                    .child(
                        // Right: git actions
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(Button::new("fetch").label("Fetch").on_click(cx.listener(
                                |this, _, _, cx| {
                                    this.run_git_op("Fetch", GitService::fetch, cx);
                                },
                            )))
                            .child(Button::new("pull").label("Pull").on_click(cx.listener(
                                |this, _, _, cx| {
                                    this.run_git_op("Pull", GitService::pull, cx);
                                },
                            )))
                            .child(Button::new("push").label("Push").on_click(cx.listener(
                                |this, _, _, cx| {
                                    this.run_git_op("Push", GitService::push, cx);
                                },
                            ))),
                    ),
            )
    }
}
