# `ai_core` — the Rust AI Engine

`ai_core` is a Rust crate that gives LazyDesktop AI capabilities: cloud
provider calls (OpenRouter, OpenAI, Anthropic, Google AI Studio), local GGUF
model inference, model downloads, and Conventional Commits message
generation. The GPUI app consumes it directly as a Rust library; the crate
also builds a `staticlib` (via `ffi.rs`) for external consumer projects.

## Why Rust?

- `llama-cpp-2` is a mature, safe Rust binding for llama.cpp.
- Memory safety for code that runs untrusted token data.
- Cloud calls use `reqwest`; local inference uses `llama-cpp-2`.

## Crate layout

| Module | Responsibility |
|--------|---------------|
| `lib.rs` | Crate root and module wiring |
| `cloud.rs` | Cloud providers (OpenRouter / OpenAI / Anthropic / Google) |
| `ffi.rs` | `extern "C"` exports (kept for external consumer projects) |
| `inference.rs` | GGUF loading (`llama_cpp_2`) + streaming inference on a worker thread |
| `download.rs` | HuggingFace downloads with progress + SHA-256 verification |
| `discovery.rs` | Model discovery (HuggingFace API + curated list), `ggml` backend detection |
| `commit_message.rs` | Prompt building + normalization to Conventional Commits |
| `types.rs` | Shared types, `CommitContext` JSON |
| `error.rs` | Error types |

## Cloud providers

`ai_core::cloud` provides `generate_cloud(provider, model, api_key,
system_prompt, user_prompt)` — a blocking function that returns the generated
text. It implements the exact API contracts used by the app:

- **OpenRouter / OpenAI** — `POST /chat/completions`, Bearer auth.
- **Anthropic** — `POST /v1/messages`, `x-api-key` auth, single user message
  combining system and user prompts.
- **Google AI Studio** — `POST /v1beta/models/{model}:generateContent?key=…`.

`extract_text` normalizes each provider's JSON response into plain text.

## `CommitContext` JSON

The commit-message API takes a JSON object:

```json
{
  "vcs":             "git" | "jujutsu",
  "repo_path":       "string",   // repository root
  "diff":            "string",   // diff text for the changes
  "files":           [ { "path": "string", "status": "M" } ],
  "staged":          [ "string", "..." ],
  "branch":          "string",   // branch name / working-copy change id
  "recent_messages": [ "string", "..." ],
  "mode":            "message" | "description",
  "system_prompt":   "string"    // optional; may contain "<diff>"
}
```

All fields are optional unless noted. The crate builds a prompt (honoring the
`<diff>` placeholder), runs inference, and normalizes the output to
`type(scope): subject`.

## GPUI-side usage

`crates/app/src/commit_panel.rs` calls:

- `ai_core::cloud::generate_cloud(...)` — blocking; invoked from
  `tokio::task::spawn_blocking`.
- `ai_core::commit_message::generate_commit_message(...)` — local GGUF
  inference (non-blocking in the app thanks to `spawn_blocking`).

## Background downloads

Model downloads in the UI do not use the FFI directly. The app launches a
detached worker mode (`lazydesktop --background-dl <url> <dest> <sha256>
<modelsDir> <configPath>`), which downloads the model via a `.part` file,
verifies the SHA-256 checksum, and writes JSON sidecar status files so the UI
can show progress and offer cancellation across restarts.

## Testing

Run the crate's test suite with:

```bash
cargo test --manifest-path crates/ai_core/Cargo.toml
```

or `just test`. Tests use `tempfile` for isolated model directories and mock
HTTP responses for the cloud providers.