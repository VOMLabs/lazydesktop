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

fn kind_fg(kind: DiffLineKind) -> Rgba {
    match kind {
        DiffLineKind::Addition => rgb(0x3fb950), // green
        DiffLineKind::Deletion => rgb(0xf85149), // red
        DiffLineKind::Hunk => rgb(0x79c0ff),     // blue
        DiffLineKind::Context => rgb(0xc9d1d9),  // soft gray
    }
}

fn kind_bg(kind: DiffLineKind) -> Option<Rgba> {
    match kind {
        DiffLineKind::Addition => Some(rgba(0x2ea04320)), // translucent green
        DiffLineKind::Deletion => Some(rgba(0xf8514920)), // translucent red
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
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let source = self.source.clone();
        let error = self.error.clone();
        let lines = self.lines.clone();

        let title = source.as_ref().map(source_title);
        let body = match (&source, error, lines.is_empty()) {
            (None, _, _) => placeholder("Select a file or commit to view changes"),
            (_, Some(err), _) => placeholder(&format!("Could not load diff: {err}")),
            (_, None, true) => placeholder("No changes to display"),
            (_, None, false) => diff_scroll(&lines),
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .child(
                // Header bar
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .child(
                        div()
                            .text_sm()
                            .font_bold()
                            .child(title.unwrap_or_else(|| "Diff".to_string())),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x8c959f))
                            .child(format!("{} lines", lines.len())),
                    ),
            )
            .child(body)
    }
}

fn placeholder(text: &str) -> AnyElement {
    div()
        .flex()
        .flex_1()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_2()
        .child(
            div()
                .text_sm()
                .text_color(rgb(0x8c959f))
                .child(text.to_string()),
        )
        .into_any()
}

fn diff_scroll(lines: &[DiffLine]) -> AnyElement {
    div()
        .flex_1()
        .id("diff-scroll")
        .overflow_y_scroll()
        .children(
            lines
                .iter()
                .enumerate()
                .map(|(index, line)| diff_row(index, line)),
        )
        .into_any()
}

fn diff_row(index: usize, line: &DiffLine) -> Div {
    let mut row = div()
        .flex()
        .flex_row()
        .whitespace_nowrap()
        .font_family("monospace");
    if let Some(bg) = kind_bg(line.kind) {
        row = row.bg(bg);
    }
    row.child(
        // Line-number gutter (mirrors the Qt LineNumberArea)
        div()
            .w_8()
            .flex_shrink_0()
            .text_right()
            .pr_2()
            .text_color(rgb(0x6e7681))
            .child((index + 1).to_string()),
    )
    .child(
        div()
            .text_color(kind_fg(line.kind))
            .child(line.text.clone()),
    )
}
