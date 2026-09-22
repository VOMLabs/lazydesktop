//! Sidebar — branch selector, commit history, recent projects.
//!
//! Mirrors the Qt sidebar (`src/mainwindow.cpp`):
//! - Branches: list with create (auto-switch), rename current, delete.
//! - History: commit list; clicking a commit shows its diff.
//! - Projects: recent-projects list plus load / init / clone controls.
//! - Settings row pinned at the bottom.

use std::path::PathBuf;

use gpui::prelude::*;
use gpui::*;
use gpui_component::button::Button;
use gpui_component::input::{Input, InputState};
use gpui_component::*;

use config::paths::projects_path;
use config::projects::RecentProjects;

use crate::git_service::GitService;
use lazydesktop_app::theme::Palette;

/// Events emitted by the sidebar.
#[derive(Clone, Debug)]
pub enum SidebarEvent {
    /// User clicked a commit in the history — show its diff.
    CommitSelected(String),
    /// User clicked the Settings row.
    SettingsRequested,
    /// User asked to open a repository at the given path.
    OpenRepoRequested(PathBuf),
}

impl EventEmitter<SidebarEvent> for Sidebar {}

pub struct Sidebar {
    git_service: Entity<GitService>,
    expanded_section: SidebarSection,
    // Branch management.
    new_branch_input: Entity<InputState>,
    branch_rename_input: Entity<InputState>,
    renaming_branch: bool,
    // Projects.
    recent_projects: Vec<String>,
    project_path_input: Entity<InputState>,
    clone_url_input: Entity<InputState>,
    clone_dest_input: Entity<InputState>,
    /// Last sidebar operation error, shown as a status line.
    op_error: Option<String>,
}

#[derive(Clone, Copy, PartialEq)]
enum SidebarSection {
    Branches,
    History,
    Projects,
}

impl Sidebar {
    pub fn new(
        window: &mut Window,
        git_service: Entity<GitService>,
        cx: &mut Context<Self>,
    ) -> Self {
        let new_branch_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("new branch name"));
        let branch_rename_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("branch name"));
        let project_path_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("/path/to/repo"));
        let clone_url_input = cx
            .new(|cx| InputState::new(window, cx).placeholder("https://github.com/user/repo.git"));
        let clone_dest_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("destination dir"));

        Self {
            git_service,
            expanded_section: SidebarSection::History,
            new_branch_input,
            branch_rename_input,
            renaming_branch: false,
            recent_projects: RecentProjects::load(&projects_path()).paths().to_vec(),
            project_path_input,
            clone_url_input,
            clone_dest_input,
            op_error: None,
        }
    }

    /// Reload the recent-projects list (called after a repo is opened).
    pub fn refresh_recent_projects(&mut self) {
        self.recent_projects = RecentProjects::load(&projects_path()).paths().to_vec();
    }

    /// Create a branch (auto-switches, matching Qt) and refresh.
    fn create_branch(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.new_branch_input.read(cx).value().trim().to_string();
        if name.is_empty() {
            return;
        }
        let result = self.git_service.update(cx, |service, _| {
            let r = service.create_branch(&name);
            service.refresh_all();
            r
        });
        self.op_error = result.err().map(|e| e.to_string());
        self.new_branch_input
            .update(cx, |state, cx| state.set_value("", window, cx));
        cx.notify();
    }

    /// Rename the current branch and refresh.
    fn rename_current_branch(&mut self, cx: &mut Context<Self>) {
        let name = self.branch_rename_input.read(cx).value().trim().to_string();
        if name.is_empty() {
            return;
        }
        let result = self.git_service.update(cx, |service, _| {
            let r = service.rename_branch(&name);
            service.refresh_all();
            r
        });
        self.op_error = result.err().map(|e| e.to_string());
        self.renaming_branch = false;
        cx.notify();
    }

    /// Delete a branch and refresh.
    fn delete_branch(&mut self, name: &str, cx: &mut Context<Self>) {
        let result = self.git_service.update(cx, |service, _| {
            let r = service.delete_branch(name);
            service.refresh_all();
            r
        });
        self.op_error = result.err().map(|e| e.to_string());
        cx.notify();
    }

    /// Open the repository at the given path.
    fn open_path(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if !path.exists() {
            self.op_error = Some(format!("Path does not exist: {}", path.display()));
            cx.notify();
            return;
        }
        cx.emit(SidebarEvent::OpenRepoRequested(path));
    }

    /// Load a repository from the path input.
    fn load_from_input(&mut self, cx: &mut Context<Self>) {
        let path = PathBuf::from(self.project_path_input.read(cx).value().trim());
        self.open_path(path, cx);
    }

    /// Initialize a git repository at the path input, then open it.
    fn init_from_input(&mut self, cx: &mut Context<Self>) {
        let path = PathBuf::from(self.project_path_input.read(cx).value().trim());
        if !path.exists() {
            self.op_error = Some(format!("Path does not exist: {}", path.display()));
            cx.notify();
            return;
        }
        let result = self
            .git_service
            .update(cx, |service, _| service.init_repo(&path));
        if let Err(e) = result {
            self.op_error = Some(e.to_string());
            cx.notify();
            return;
        }
        cx.emit(SidebarEvent::OpenRepoRequested(path));
    }

    /// Clone a repository from the URL input into the destination input.
    fn clone_from_input(&mut self, cx: &mut Context<Self>) {
        let url = self.clone_url_input.read(cx).value().trim().to_string();
        let dest = PathBuf::from(self.clone_dest_input.read(cx).value().trim());
        if url.is_empty() || dest.as_os_str().is_empty() {
            self.op_error = Some("Enter both a URL and a destination dir.".to_string());
            cx.notify();
            return;
        }
        let result = self
            .git_service
            .update(cx, |service, _| service.clone_repo(&url, &dest));
        if let Err(e) = result {
            self.op_error = Some(e.to_string());
            cx.notify();
            return;
        }
        cx.emit(SidebarEvent::OpenRepoRequested(dest));
    }

    /// Set the rename input to the current branch name (called before showing
    /// the rename row) — needs a window to set input state.
    fn start_rename(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.branch_rename_input
            .update(cx, |state, cx| state.set_value(name, window, cx));
        self.renaming_branch = true;
        cx.notify();
    }
}

impl Render for Sidebar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let branches = self.git_service.read(cx).branches.clone();
        let current = self.git_service.read(cx).current_branch.clone();
        let history = self.git_service.read(cx).commit_history.clone();
        let recent_projects = self.recent_projects.clone();
        let renaming_branch = self.renaming_branch;
        let op_error = self.op_error.clone();
        let p = Palette::current(cx);

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(p.panel)
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
            .child(self.render_branches(branches, current, renaming_branch, cx))
            .child(self.render_history(history, cx))
            .child(self.render_projects(recent_projects, cx))
            .when_some(op_error.clone(), |this, err| {
                this.child(div().text_xs().text_color(p.error).px_2().py_1().child(err))
            })
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
                    .hover(|this| this.bg(p.hover))
                    .on_click(cx.listener(|_this, _, _, cx| {
                        cx.emit(SidebarEvent::SettingsRequested);
                        cx.notify();
                    }))
                    .child(div().text_sm().font_bold().child("Settings")),
            )
    }
}

impl Sidebar {
    fn render_branches(
        &mut self,
        branches: Vec<git_cmd::types::BranchEntry>,
        current: String,
        renaming_branch: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let branches_len = branches.len();
        let new_branch_input = self.new_branch_input.clone();
        let p = Palette::current(cx);

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
                    .hover(|this| this.bg(p.hover))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.expanded_section = SidebarSection::Branches;
                        cx.notify();
                    }))
                    .child(div().text_sm().font_bold().child("Branches"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(p.text_muted)
                            .child(format!("{}", branches_len)),
                    ),
            )
            .when(self.expanded_section == SidebarSection::Branches, |this| {
                this.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_0p5()
                        .children(branches.into_iter().map(|b| {
                            let is_current = b.name == current;
                            let branch_name = b.name.clone();
                            let p = Palette::current(cx);
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .px_2()
                                .py_1_5()
                                .rounded_md()
                                .id(format!("branch-{}", b.name))
                                .when(is_current, |this| this.bg(p.accent_selected))
                                .cursor_pointer()
                                .on_click(cx.listener({
                                    let switch_name = branch_name.clone();
                                    move |this, _, _, cx| {
                                        if !is_current {
                                            this.git_service.update(cx, |service, _| {
                                                let _ = service.checkout(&switch_name);
                                                service.refresh_all();
                                            });
                                            cx.notify();
                                        }
                                    }
                                }))
                                .child(div().flex_1().text_sm().child(b.name.clone()))
                                .when(is_current, |this| {
                                    // Rename button (current branch).
                                    let name = branch_name.clone();
                                    this.child(
                                        div()
                                            .id(format!("rename-{branch_name}"))
                                            .cursor_pointer()
                                            .hover(|this| this.text_color(p.success))
                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                this.start_rename(&name, window, cx);
                                            }))
                                            .child(div().text_xs().child("✎")),
                                    )
                                })
                                .when(!is_current, |this| {
                                    // Delete button (non-current branches).
                                    let name = branch_name.clone();
                                    this.child(
                                        div()
                                            .id(format!("del-{branch_name}"))
                                            .cursor_pointer()
                                            .hover(|this| this.text_color(p.error))
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.delete_branch(&name, cx);
                                            }))
                                            .child(div().text_xs().child("✕")),
                                    )
                                })
                        })),
                )
                .when(renaming_branch, |this| {
                    this.child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .px_2()
                            .py_1()
                            .child(div().flex_1().child(Input::new(&self.branch_rename_input)))
                            .child(
                                Button::new("rename-confirm")
                                    .small()
                                    .label("Rename")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.rename_current_branch(cx);
                                    })),
                            ),
                    )
                })
                .child(div().h_1())
                .child(
                    // New branch row: input + create.
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .px_2()
                        .py_1()
                        .child(div().flex_1().child(Input::new(&new_branch_input)))
                        .child(
                            Button::new("create-branch")
                                .small()
                                .label("Create")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.create_branch(window, cx);
                                })),
                        ),
                )
            })
    }

    fn render_history(
        &mut self,
        history: Vec<git_cmd::types::CommitEntry>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let history_len = history.len();
        let p = Palette::current(cx);
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
                    .hover(|this| this.bg(p.hover))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.expanded_section = SidebarSection::History;
                        cx.notify();
                    }))
                    .child(div().text_sm().font_bold().child("History"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(p.text_muted)
                            .child(format!("{}", history_len)),
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
                                .hover(|this| this.bg(p.hover))
                                .on_click(cx.listener(move |_this, _, _, cx| {
                                    cx.emit(SidebarEvent::CommitSelected(hash.clone()));
                                    cx.notify();
                                }))
                                .child(
                                    // Hash — monospace, small
                                    div()
                                        .text_xs()
                                        .font_family("monospace")
                                        .text_color(p.text_muted)
                                        .child(c.hash.clone()),
                                )
                                .child(
                                    // Subject — normal
                                    div().text_sm().child(c.subject.clone()),
                                )
                        })),
                )
            })
    }

    fn render_projects(
        &mut self,
        recent_projects: Vec<String>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let recent_len = recent_projects.len();
        let project_path_input = self.project_path_input.clone();
        let clone_url_input = self.clone_url_input.clone();
        let clone_dest_input = self.clone_dest_input.clone();
        let p = Palette::current(cx);

        // Static "load / init / clone" control row appended after the recent list.
        let actions_row =
            div()
                .flex()
                .flex_col()
                .gap_2()
                .px_2()
                .py_1()
                .id("project-actions")
                .child(
                    // Load / Init path row.
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(div().flex_1().child(Input::new(&project_path_input)))
                        .child(Button::new("load-repo").small().label("Load").on_click(
                            cx.listener(|this, _, _, cx| {
                                this.load_from_input(cx);
                            }),
                        ))
                        .child(Button::new("init-repo").small().label("Init").on_click(
                            cx.listener(|this, _, _, cx| {
                                this.init_from_input(cx);
                            }),
                        )),
                )
                .child(
                    // Clone row: URL + destination + button.
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(div().flex_1().child(Input::new(&clone_url_input)))
                        .child(div().flex_1().child(Input::new(&clone_dest_input)))
                        .child(Button::new("clone-repo").small().label("Clone").on_click(
                            cx.listener(|this, _, _, cx| {
                                this.clone_from_input(cx);
                            }),
                        )),
                );

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
                    .id("projects-header")
                    .cursor_pointer()
                    .hover(|this| this.bg(p.hover))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.expanded_section = SidebarSection::Projects;
                        cx.notify();
                    }))
                    .child(div().text_sm().font_bold().child("Projects"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(p.text_muted)
                            .child(format!("{}", recent_len)),
                    ),
            )
            .when(self.expanded_section == SidebarSection::Projects, |this| {
                this.child(
                    div().flex().flex_col().gap_2().children(
                        recent_projects
                            .into_iter()
                            .map(|path| {
                                let path_for_click = path.clone();
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .px_2()
                                    .py_1_5()
                                    .rounded_md()
                                    .id(format!("project-{path}"))
                                    .cursor_pointer()
                                    .hover(|this| this.bg(p.hover))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.open_path(PathBuf::from(path_for_click.clone()), cx);
                                    }))
                                    .child(div().text_sm().child(path))
                            })
                            .chain(std::iter::once(actions_row)),
                    ),
                )
            })
    }
}
