# `ai_core` — the Rust AI Engine

`ai_core` is a Rust crate that gives LazyDesktop local AI capabilities: GGUF
model inference, model downloads, and Conventional Commits message
generation. It is compiled as a `staticlib` and linked into the C++
application, which talks to it through a small C ABI.

## Why Rust?

- `llama-cpp-2` is a mature, safe Rust binding for llama.cpp.
- Memory safety for code that runs untrusted token data.
- The crate is self-contained; the C++ side only sees the C ABI.

## Crate layout

| Module | Responsibility |
|--------|---------------|
| `lib.rs` | Crate root and module wiring |
| `ffi.rs` | `extern "C"` exports |
| `inference.rs` | GGUF loading (`llama_cpp_2`) + streaming inference on a worker thread |
| `download.rs` | HuggingFace downloads with progress + SHA-256 verification |
| `discovery.rs` | Model discovery (HuggingFace API + curated list), `ggml` backend detection |
| `commit_message.rs` | Prompt building + normalization to Conventional Commits |
| `types.rs` | Shared types, `CommitContext` JSON |
| `error.rs` | Error types |

## C FFI reference

Declared in `crates/ai_core/ai_core.h`.

### Lifecycle

| Function | Description |
|----------|-------------|
| `ModelManager *mm_init(models_dir, config_path)` | Create the manager. `models_dir` is where GGUF files live; `config_path` persists model metadata. |
| `void mm_destroy(ModelManager *)` | Free all resources. Safe while no inference thread is running. |

### Downloads

| Function | Description |
|----------|-------------|
| `int32_t mm_download_model(mm, url, dest_path, expected_sha256, progress_cb, finished_cb, user_data)` | Start a download; returns a download ID or `-1`. Progress and completion callbacks are optional. |
| `void mm_cancel_download(mm, download_id)` | Cancel an active download. |

### Model listing / deletion

| Function | Description |
|----------|-------------|
| `char *mm_list_local_models(mm)` | JSON array of known models (name, url, path, size, sha256, downloaded). Free with `mm_free_string`. |
| `char *mm_discover_models(mm)` | JSON array of discoverable models from HuggingFace + curated list. |
| `bool mm_delete_model(mm, path)` | Delete a model file from disk. |
| `void mm_free_string(char *)` | Free strings returned by the crate. |

### Inference

| Function | Description |
|----------|-------------|
| `bool mm_stream_inference(mm, model_path, prompt, n_gpu_layers, on_token, on_error, on_cancelled, on_finish, user_data)` | Generic streaming completion. Returns immediately; callbacks run on the inference thread. `on_cancelled` is polled — return `true` to cancel. |
| `bool mm_generate_commit_message(mm, model_path, context_json, n_gpu_layers, on_token, on_error, on_cancelled, on_finish, user_data)` | Generate a Conventional Commits message from a `CommitContext` JSON object. Raw tokens stream via `on_token`; the normalized message arrives via `on_finish`. |

Only one inference may run at a time; a second request returns `false`.

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

## C++ side

`ModelManagerBridge` (`src/model_manager_bridge.h/.cpp`) wraps the FFI in
Qt-friendly signals:

- `inferenceToken` — streamed tokens
- `inferenceFinished` — final normalized text
- `inferenceFailed` — error message
- `inferenceCancelled` — user-requested cancellation

The bridge owns the `ModelManager` handle; worker threads never touch the
manager directly, so the handle can be torn down safely on exit.

## Background downloads

Model downloads in the UI do not use the FFI directly. The main process
relaunches itself in a detached worker mode
(`lazydesktop --background-dl <url> <dest> <sha256> <modelsDir>
<configPath>`), which downloads the model via a `.part` file, verifies the
SHA-256 checksum, and writes JSON sidecar status files so the UI can show
progress and offer cancellation across restarts. See
`src/background_download.h`.

## Testing

Run the crate's test suite with:

```bash
cargo test --manifest-path crates/ai_core/Cargo.toml
```

or `just test`. Tests use `tempfile` for isolated model directories.
