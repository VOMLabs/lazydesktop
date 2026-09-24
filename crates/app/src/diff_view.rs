//! Diff view — parsed, colorized unified diff for the selected file.
//!
//! Ported from the Qt `DiffViewer` / `DiffHighlighter`
//! (`src/diffviewer.cpp`): unified-diff headers are filtered out, `+`/`-`/`@@`
//! lines are colorized, and a line-number gutter is rendered in a monospace,
//! no-wrap scroll area.

use gpui::prelude::*;
use gpui::*;
use gpui_component::*;

use crate::git_service::GitService;
use lazydesktop_app::diff::{parse_diff, DiffLine, DiffLineKind};
use lazydesktop_app::theme::Palette;

fn kind_fg(kind: DiffLineKind, p: &Palette) -> Rgba {
    match kind {
        DiffLineKind::Addition => p.diff_add_fg,
        DiffLineKind::Deletion => p.diff_del_fg,
        DiffLineKind::Hunk => p.hunk,
        DiffLineKind::Context => p.diff_ctx_fg,
    }
}

fn kind_bg(kind: DiffLineKind, p: &Palette) -> Option<Rgba> {
    match kind {
        DiffLineKind::Addition => Some(p.diff_add_bg),
        DiffLineKind::Deletion => Some(p.diff_del_bg),
        _ => None,
    }
}

/// What the diff view is currently showing.
#[derive(Clone, Debug, PartialEq)]
enum DiffSource {
    /// Working-tree diff of a single file.
    File(String),
    /// Diff introduced by a commit (`git show <hash>`).
    Commit(String),
}

pub struct DiffView {
    git_service: Entity<GitService>,
    source: Option<DiffSource>,
    lines: Vec<DiffLine>,
    error: Option<String>,
}

impl DiffView {
    pub fn new(git_service: Entity<GitService>, _cx: &mut Context<Self>) -> Self {
        Self {
            git_service,
            source: None,
            lines: Vec::new(),
            error: None,
        }
    }

    /// Show the working-tree diff for a file. Reloads on every call so
    /// staging changes are reflected immediately.
    pub fn set_file(&mut self, path: String, cx: &mut Context<Self>) {
        self.source = Some(DiffSource::File(path));
        self.reload(cx);
    }

    /// Show the diff introduced by a commit.
    pub fn set_commit(&mut self, hash: String, cx: &mut Context<Self>) {
        self.source = Some(DiffSource::Commit(hash));
        self.reload(cx);
    }

    /// Reload the diff for the current source.
    fn reload(&mut self, cx: &mut Context<Self>) {
        self.error = None;
        let Some(source) = self.source.clone() else {
            self.lines.clear();
            return;
        };

        let result = match &source {
            DiffSource::File(path) => self.git_service.read(cx).diff_file(path),
            DiffSource::Commit(hash) => self.git_service.read(cx).show_commit(hash),
        };

        match result {
            Ok(raw) => self.lines = parse_diff(&raw),
            Err(err) => {
                self.lines.clear();
                self.error = Some(err.to_string());
            }
        }
        cx.notify();
    }

    /// Clear the current diff (mirrors Qt `DiffViewer::clear`).
    #[allow(dead_code)] // Planned: file deselection
    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.source = None;
        self.lines.clear();
        self.error = None;
        cx.notify();
    }
}

fn source_title(source: &DiffSource) -> String {
    match source {
        DiffSource::File(path) => path.clone(),
        DiffSource::Commit(hash) => format!("commit {hash}"),
    }
}

impl Render for DiffView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let source = self.source.clone();
        let error = self.error.clone();
        let lines = self.lines.clone();
        let p = Palette::current(cx);

        let title = source.as_ref().map(source_title);
        let body = match (&source, error, lines.is_empty()) {
            (None, _, _) => placeholder("Select a file or commit to view changes", &p),
            (_, Some(err), _) => placeholder(&format!("Could not load diff: {err}"), &p),
            (_, None, true) => placeholder("No changes to display", &p),
            (_, None, false) => diff_scroll(&lines, &p),
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(p.bg)
            .child(
                // Header bar
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(p.separator)
                    .child(
                        div()
                            .text_sm()
                            .font_medium()
                            .child(title.unwrap_or_else(|| "Diff".to_string())),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .text_xs()
                            .text_color(p.text_muted)
                            .child(format!("{} lines", lines.len())),
                    ),
            )
            .child(body)
    }
}

fn placeholder(text: &str, p: &Palette) -> AnyElement {
    div()
        .flex()
        .flex_1()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_3()
        .child(
            // Dark-tinted icon block (diff emblem).
            div()
                .flex()
                .items_center()
                .justify_center()
                .w_10()
                .h_10()
                .rounded_md()
                .bg(p.elevated)
                .border_1()
                .border_color(p.separator)
                .child(div().text_lg().text_color(p.text_muted).child("±")),
        )
        .child(
            div()
                .text_sm()
                .text_color(p.text_muted)
                .child(text.to_string()),
        )
        .into_any()
}

fn diff_scroll(lines: &[DiffLine], p: &Palette) -> AnyElement {
    div()
        .flex_1()
        .id("diff-scroll")
        .overflow_y_scroll()
        .children(
            lines
                .iter()
                .enumerate()
                .map(|(index, line)| diff_row(index, line, p)),
        )
        .into_any()
}

fn diff_row(index: usize, line: &DiffLine, p: &Palette) -> Div {
    let mut row = div()
        .flex()
        .flex_row()
        .whitespace_nowrap()
        .font_family("monospace");
    if let Some(bg) = kind_bg(line.kind, p) {
        row = row.bg(bg);
    }
    row.child(
        // Line-number gutter (mirrors the Qt LineNumberArea)
        div()
            .w_8()
            .flex_shrink_0()
            .text_right()
            .pr_2()
            .text_color(p.diff_gutter)
            .child((index + 1).to_string()),
    )
    .child(
        div()
            .text_color(kind_fg(line.kind, p))
            .child(line.text.clone()),
    )
}
