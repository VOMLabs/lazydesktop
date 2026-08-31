//! Commit-message generation: builds a Conventional Commits prompt from host
//! context, runs inference, and normalizes the result.

use std::sync::atomic::AtomicBool;

use serde::Deserialize;

use crate::inference::run_inference_blocking;

const DIFF_PLACEHOLDER: &str = "<diff>";

/// A single changed file (path + short status letter, e.g. "M", "A", "D").
#[derive(Debug, Clone, Deserialize)]
pub struct FileChange {
    pub path: String,
    #[serde(default)]
    pub status: String,
}

/// Host-supplied context for commit-message generation. Deserialized from the
/// JSON blob passed to `mm_generate_commit_message`.
#[derive(Debug, Clone, Deserialize)]
pub struct CommitContext {
    /// "git" or "jujutsu".
    #[serde(default)]
    pub vcs: String,
    #[serde(default)]
    pub repo_path: String,
    /// Raw diff text for the changes to be committed.
    #[serde(default)]
    pub diff: String,
    #[serde(default)]
    pub files: Vec<FileChange>,
    #[serde(default)]
    pub staged: Vec<String>,
    /// Current branch name (git) or working-copy change id (jujutsu).
    #[serde(default)]
    pub branch: String,
    /// Recent commit subjects, used as style reference.
    #[serde(default)]
    pub recent_messages: Vec<String>,
    /// "message" (subject + optional body) or "description" (body only).
    #[serde(default)]
    pub mode: String,
    /// Optional host-provided system prompt. If omitted, a built-in prompt
    /// following Conventional Commits is used.
    #[serde(default)]
    pub system_prompt: Option<String>,
}

/// Built-in system prompt for Conventional Commits message generation.
pub fn default_system_prompt(mode: &str) -> String {
    if mode == "description" {
        "You are an expert at writing commit message descriptions (bodies) for the \
         Conventional Commits specification.\n\
         Rules:\n\
         - Write a concise body that explains what changed and why.\n\
         - Use bullet points, each on its own line, no more than 8 bullets.\n\
         - Mention breaking changes with a 'BREAKING CHANGE:' footer when relevant.\n\
         - Do not repeat the commit subject.\n\
         - Do not wrap the output in markdown fences or quotes."
            .to_string()
    } else {
        "You are an expert at writing Git commit messages following the Conventional \
         Commits specification.\n\
         Rules:\n\
         - Subject format: type(optional scope): short imperative description.\n\
         - Valid types: feat, fix, refactor, chore, docs, test, perf, build, ci, style, revert.\n\
         - Subject: imperative present tense, no trailing period, at most 72 characters.\n\
         - If the change is larger, add a short body after a blank line and a \
           'BREAKING CHANGE:' footer only when the change is breaking.\n\
         - Output ONLY the commit message. Do not wrap it in markdown fences, quotes, \
           or add commentary."
            .to_string()
    }
}

/// Builds the (system, user) prompt pair for a given context.
///
/// If the system prompt contains the `<diff>` placeholder it is substituted
/// with the full context block; otherwise the context block is appended as the
/// user message (matching prompts written before the placeholder existed).
pub fn build_prompt(ctx: &CommitContext) -> (String, String) {
    let system = match &ctx.system_prompt {
        Some(p) if !p.trim().is_empty() => p.trim().to_string(),
        _ => default_system_prompt(&ctx.mode),
    };

    if system.contains(DIFF_PLACEHOLDER) {
        let filled = system.replace(DIFF_PLACEHOLDER, &context_block(ctx));
        (filled, "Generate the response now.".to_string())
    } else {
        (system, format!("Changes:\n{}", context_block(ctx)))
    }
}

/// Human-readable context block describing the repository state and diff.
fn context_block(ctx: &CommitContext) -> String {
    let mut block = String::new();

    if !ctx.repo_path.is_empty() {
        block.push_str(&format!("Repository: {}\n", ctx.repo_path));
    }
    if ctx.vcs == "jujutsu" {
        block.push_str("VCS: Jujutsu (jj)\n");
    }
    if !ctx.branch.is_empty() {
        block.push_str(&format!("Current branch/change: {}\n", ctx.branch));
    }
    if !ctx.files.is_empty() {
        block.push_str("Changed files:\n");
        for f in &ctx.files {
            let status = if f.status.is_empty() { "M" } else { &f.status };
            block.push_str(&format!("  {status} {}\n", f.path));
        }
    }
    if !ctx.staged.is_empty() {
        block.push_str("Staged files:\n");
        for s in &ctx.staged {
            block.push_str(&format!("  {s}\n"));
        }
    }
    if !ctx.recent_messages.is_empty() {
        block.push_str("Recent commit messages (match their style):\n");
        for m in &ctx.recent_messages {
            block.push_str(&format!("  {m}\n"));
        }
    }

    block.push('\n');
    block.push_str("Diff:\n");
    if ctx.diff.trim().is_empty() {
        block.push_str("<no diff provided>\n");
    } else {
        block.push_str(&ctx.diff);
        if !ctx.diff.ends_with('\n') {
            block.push('\n');
        }
    }
    block
}

/// Cleans up model output into a usable Conventional Commits message:
/// strips markdown fences, surrounding quotes and leading filler lines, and
/// removes trailing punctuation from the subject line.
pub fn normalize_message(text: &str) -> String {
    let mut t = text.trim();

    if let Some(rest) = t.strip_prefix("```") {
        t = rest.trim_start();
        if let Some(stripped) = t.strip_suffix("```") {
            t = stripped.trim_end();
        }
    }
    if (t.starts_with('"') && t.ends_with('"')) || (t.starts_with('\'') && t.ends_with('\'')) {
        t = &t[1..t.len() - 1];
        t = t.trim();
    }

    let lines: Vec<&str> = t.lines().collect();
    if lines.is_empty() {
        return String::new();
    }

    let subject_idx = lines
        .iter()
        .position(|l| is_conventional_header(l))
        .unwrap_or_else(|| lines.iter().position(|l| !l.trim().is_empty()).unwrap_or(0));

    let mut out: Vec<String> = Vec::new();
    for line in lines.iter().skip(subject_idx) {
        let l = line.trim_end();
        if !l.is_empty() || !out.is_empty() {
            out.push(l.to_string());
        }
    }
    let mut joined = out.join("\n").trim().to_string();

    if let Some(header_end) = joined.find('\n') {
        let subject = &joined[..header_end];
        let rest = &joined[header_end..];
        joined = format!("{}{}", clean_subject(subject), rest);
    } else {
        joined = clean_subject(&joined);
    }
    joined.trim().to_string()
}

/// True if the line looks like a conventional commit header, with or without
/// a leading bullet (`- feat: ...`).
fn is_conventional_header(line: &str) -> bool {
    let l = line
        .trim()
        .trim_start_matches("- ")
        .trim_start_matches("* ");
    if !l.contains(':') {
        return false;
    }
    let header = l.split_once(':').map(|(h, _)| h).unwrap_or("");
    let core = header.split('(').next().unwrap_or(header).trim();
    matches!(
        core,
        "feat"
            | "fix"
            | "refactor"
            | "chore"
            | "docs"
            | "test"
            | "perf"
            | "build"
            | "ci"
            | "style"
            | "revert"
            | "improvement"
    )
}

fn clean_subject(s: &str) -> String {
    let mut s = s
        .trim()
        .trim_start_matches("- ")
        .trim_start_matches("* ")
        .to_string();
    if s.ends_with('.') && !s.ends_with("...") {
        s.pop();
    }
    s.trim().to_string()
}

/// Generates a normalized commit message for the given context by running
/// inference with the constructed prompt.
pub fn generate_commit_message(
    ctx: &CommitContext,
    model_path: &str,
    n_gpu_layers: i32,
    cancel: &AtomicBool,
    on_token: &dyn Fn(&str),
) -> Result<String, String> {
    let (system, user) = build_prompt(ctx);
    let prompt = format!("{system}\n\n{user}");
    let raw = run_inference_blocking(model_path, &prompt, n_gpu_layers, cancel, on_token)?;
    Ok(normalize_message(&raw))
}
