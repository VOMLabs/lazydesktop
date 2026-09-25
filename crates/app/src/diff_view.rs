//! Diff view — parsed, colorized unified diff for the selected file.
//!
//! Ported from the Qt `DiffViewer` / `DiffHighlighter`
//! (`src/diffviewer.cpp`): unified-diff headers are filtered out, `+`/`-`/`@@`
//! lines are colorized, and a line-number gutter is rendered in a monospace,
//! no-wrap scroll area.
//!
//! Two extras on top of the Qt behavior:
//! - paired `-`/`+` lines get word-level highlighting (changed words tinted
//!   within the changed line, via [`lazydesktop_app::diff::word_diff`]);
//! - image files render inline in a bordered card above the diff text
//!   (worktree files directly, commit-side files via `git show <hash>:<path>`
//!   materialized to a temp file, decoded by gpui's asset loader).

use std::collections::HashMap;

use gpui::prelude::*;
use gpui::*;
use gpui_component::*;

use crate::git_service::GitService;
use lazydesktop_app::diff::{
    image_paths_from_raw, is_image_path, pair_changes, parse_diff, word_diff, ChangePair, DiffLine,
    DiffLineKind, WordSpan, WordSpanKind,
};
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
    /// Raw diff text, kept for image-path extraction on commit diffs.
    raw: String,
    /// Absolute paths of images to render inline above the diff text.
    images: Vec<String>,
    error: Option<String>,
}

impl DiffView {
    pub fn new(git_service: Entity<GitService>, _cx: &mut Context<Self>) -> Self {
        Self {
            git_service,
            source: None,
            lines: Vec::new(),
            raw: String::new(),
            images: Vec::new(),
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
        self.images.clear();
        let Some(source) = self.source.clone() else {
            self.lines.clear();
            self.raw.clear();
            return;
        };

        let result = match &source {
            DiffSource::File(path) => self.git_service.read(cx).diff_file(path),
            DiffSource::Commit(hash) => self.git_service.read(cx).show_commit(hash),
        };

        match result {
            Ok(raw) => {
                self.raw = raw.clone();
                self.lines = parse_diff(&raw);
                self.images = self.collect_images(&source, &raw, cx);
            }
            Err(err) => {
                self.lines.clear();
                self.raw.clear();
                self.images.clear();
                self.error = Some(err.to_string());
            }
        }
        cx.notify();
    }

    /// Resolve the images to render inline for the current source.
    fn collect_images(
        &self,
        source: &DiffSource,
        raw: &str,
        cx: &mut Context<Self>,
    ) -> Vec<String> {
        match source {
            DiffSource::File(path) if is_image_path(path) => {
                let abs = self.git_service.read(cx).repo_path.join(path);
                if abs.is_file() {
                    vec![abs.display().to_string()]
                } else {
                    Vec::new()
                }
            }
            DiffSource::File(_) => Vec::new(),
            DiffSource::Commit(hash) => image_paths_from_raw(raw)
                .iter()
                .filter_map(|path| self.write_commit_image(hash, path, cx))
                .collect(),
        }
    }

    /// Fetch an image at a revision and materialize it to a temp file so the
    /// gpui `img` element can render it (raw bytes avoid the lossy-string
    /// decoding of `run_git`, which would corrupt binary content).
    fn write_commit_image(&self, hash: &str, path: &str, cx: &mut Context<Self>) -> Option<String> {
        let bytes = self.git_service.read(cx).file_bytes_at(hash, path).ok()?;
        let dir = std::env::temp_dir()
            .join("opencode")
            .join("lazydesktop-images");
        std::fs::create_dir_all(&dir).ok()?;
        let stem: String = path
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || matches!(c, '.' | '-' | '_') {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let ext = path.rsplit('.').next().unwrap_or("img");
        let file = dir.join(format!("{hash}-{stem}.{ext}"));
        std::fs::write(&file, bytes).ok()?;
        Some(file.display().to_string())
    }

    /// Clear the current diff (mirrors Qt `DiffViewer::clear`).
    #[allow(dead_code)] // Planned: file deselection
    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.source = None;
        self.lines.clear();
        self.raw.clear();
        self.images.clear();
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
        let images = self.images.clone();
        let body = match (&source, error, lines.is_empty()) {
            (None, _, _) => placeholder("Select a file or commit to view changes", &p),
            (_, Some(err), _) => placeholder(&format!("Could not load diff: {err}"), &p),
            (_, None, true) if images.is_empty() => placeholder("No changes to display", &p),
            (_, None, true) => image_body(&images, &p, placeholder("No changes to display", &p)),
            (_, None, false) if images.is_empty() => diff_scroll(&lines, &p),
            (_, None, false) => image_body(&images, &p, diff_scroll(&lines, &p)),
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
    // Index every paired changed line so both sides render word-highlighted.
    let mut paired_with: HashMap<usize, ChangePair> = HashMap::new();
    for pair in pair_changes(lines) {
        if let (Some(deleted), Some(added)) = (pair.deleted, pair.added) {
            paired_with.insert(deleted, pair);
            paired_with.insert(added, pair);
        }
    }

    div()
        .flex_1()
        .id("diff-scroll")
        .overflow_y_scroll()
        .children(
            lines
                .iter()
                .enumerate()
                .map(|(index, line)| match paired_with.get(&index) {
                    Some(pair) => diff_row_worded(index, line, pair, lines, p),
                    None => diff_row(index, line, p),
                }),
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

/// A paired changed line rendered with word-level highlighting: the marker
/// plus spans whose changed words are tinted against the line background.
fn diff_row_worded(
    index: usize,
    line: &DiffLine,
    pair: &ChangePair,
    all: &[DiffLine],
    p: &Palette,
) -> Div {
    let removed = pair
        .deleted
        .map(|i| strip_marker(&all[i].text))
        .unwrap_or_default();
    let added = pair
        .added
        .map(|i| strip_marker(&all[i].text))
        .unwrap_or_default();
    let diff = word_diff(&removed, &added);
    let spans = if line.kind == DiffLineKind::Addition {
        diff.added
    } else {
        diff.removed
    };
    diff_row_spans(index, line, &spans, p)
}

/// Render a changed line from precomputed word spans.
fn diff_row_spans(index: usize, line: &DiffLine, spans: &[WordSpan], p: &Palette) -> Div {
    let marker = if line.kind == DiffLineKind::Addition {
        "+"
    } else {
        "-"
    };
    let mut row = div()
        .flex()
        .flex_row()
        .whitespace_nowrap()
        .font_family("monospace");
    if let Some(bg) = kind_bg(line.kind, p) {
        row = row.bg(bg);
    }
    row.child(
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
            .flex()
            .flex_row()
            .child(div().text_color(kind_fg(line.kind, p)).child(marker))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .children(spans.iter().map(|span| span_element(span, line.kind, p))),
            ),
    )
}

/// A single word span: unchanged text in the line color, changed words
/// (Insert/Delete) tinted with a translucent copy of their foreground.
fn span_element(span: &WordSpan, kind: DiffLineKind, p: &Palette) -> Div {
    let fg = match span.kind {
        WordSpanKind::Insert => p.diff_add_fg,
        WordSpanKind::Delete => p.diff_del_fg,
        WordSpanKind::Same => kind_fg(kind, p),
    };
    let mut el = div()
        .whitespace_nowrap()
        .text_color(fg)
        .child(span.text.clone());
    if let Some(bg) = span_bg(span.kind, p) {
        el = el.bg(bg);
    }
    el
}

/// Background tint for changed-word spans; unchanged spans get none.
fn span_bg(kind: WordSpanKind, p: &Palette) -> Option<Rgba> {
    let base = match kind {
        WordSpanKind::Insert => p.diff_add_fg,
        WordSpanKind::Delete => p.diff_del_fg,
        WordSpanKind::Same => return None,
    };
    let mut tinted = base;
    tinted.a *= 0.35;
    Some(tinted)
}

/// Strip the leading diff marker (`+`, `-` or context ` `) from a parsed line.
fn strip_marker(text: &str) -> String {
    match text.as_bytes().first() {
        Some(b'+') | Some(b'-') | Some(b' ') => text[1..].to_string(),
        _ => text.to_string(),
    }
}

/// Compose inline image cards above the diff body (scroll or placeholder).
fn image_body(images: &[String], p: &Palette, below: AnyElement) -> AnyElement {
    div()
        .flex()
        .flex_1()
        .flex_col()
        .children(images.iter().map(|path| image_card(path, p)))
        .child(below)
        .into_any()
}

/// A bordered card rendering one image at a capped width. gpui `img` derives
/// the height from its decoded aspect ratio, so a single dimension keeps
/// proportions (small images are upscaled to the cap — the common screenshot
/// case is downscaled to fit).
fn image_card(path: &str, p: &Palette) -> Div {
    let name = path.rsplit('/').next().unwrap_or(path).to_string();
    div()
        .flex()
        .flex_col()
        .gap_2()
        .mx_auto()
        .m_3()
        .p_4()
        .rounded_md()
        .border_1()
        .border_color(p.separator)
        .bg(p.elevated)
        .child(div().text_xs().text_color(p.text_muted).child(name))
        .child(
            div()
                .flex()
                .justify_center()
                .child(img(path.to_string()).w(px(560.0))),
        )
}
