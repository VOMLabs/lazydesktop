# LazyDesktop — Implementation Details

Comprehensive documentation of what is implemented in LazyDesktop, how it
works internally, and what each component does. This is the technical
companion to the user-facing guides in [`docs/`](docs/README.md).

---

## Part 1: High-Level Overview

### What is LazyDesktop?

LazyDesktop is a native Git GUI client for KDE Plasma, built as a lightweight
alternative to GitHub Desktop. It uses the same technology stack as KDE
itself (Qt 6, C++23) so it integrates seamlessly with the desktop without
requiring additional runtimes like Electron.

### Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     MainWindow                          │
│  ┌──────────┐  ┌────────────────────┐  ┌─────────────┐ │
│  │  Toolbar  │  │   Sidebar Tabs     │  │   Viewer    │ │
│  │  - Branch │  │  ┌──────────────┐ │  │  Stack      │ │
│  │  - Push   │  │  │ Changes Tab  │ │  │  - Diff     │ │
│  │  - Project│  │  │  - File List │ │  │  - Image    │ │
│  │           │  │  │  - Commit    │ │  │  - Placeholder│ │
│  │           │  │  ├──────────────┤ │  │             │ │
│  │           │  │  │ History Tab  │ │  │             │ │
│  │           │  │  │  - Commits   │ │  │             │ │
│  │           │  │  │  - Files     │ │  │             │ │
│  │           │  │  └──────────────┘ │  │             │ │
│  └──────────┘  └────────────────────┘  └─────────────┘ │
├─────────────────────────────────────────────────────────┤
│  Git Process Layer (QProcess)     │  AI Layer           │
│  - status, diff, commit, push     │  - Cloud APIs       │
│  - branch, checkout, log          │  - Rust ai_core    │
│  - clone, init                    │    crate (FFI)     │
│  - background model downloads     │                     │
├─────────────────────────────────────────────────────────┤
│  Native Rust Layer (vcs_core)     │                     │
│  - SSH keys, connection tests     │                     │
│  - git remote read/write          │                     │
├─────────────────────────────────────────────────────────┤
│  Persistence Layer  │
│  - QSettings (INI)   │
│  - YAML (projects,   │
│    themes)           │
└──────────────────────┘
```

### Technology Stack

| Component | Technology | Purpose |
|-----------|-----------|---------|
| UI Framework | Qt 6 (Core, Gui, Widgets, Network) | All UI rendering, networking, process management |
| Language | C++23 | Application logic |
| Build System | XMake | Compilation and linking of the C++ app and the Rust crate |
| Config Storage | yaml-cpp | YAML parsing for projects and themes |
| Local AI | Rust `ai_core` crate (`llama-cpp-2`) | GGUF model inference and download, exposed over a C FFI |
| Native VCS/SSH | Rust `vcs_core` crate (`ssh-key`, `russh`, `gix-config`) | SSH key management, connection tests, git remote config reads/writes, exposed over a C FFI |
| Background Downloads | Detached worker process (`--background-dl`) | Model downloads that survive UI restarts |

### Data Storage

All persistent data lives under `~/.config/lazydesktop/`:

| File | Format | Purpose |
|------|--------|---------|
| `lazydesktop.conf` | QSettings INI | App settings (AI provider, API key, theme, model) |
| `projects.yaml` | YAML | Recent project paths |
| `themes/*.theme.yaml` | YAML | Custom theme definitions |
| `models/` | GGUF files | Downloaded local AI models |

---

## Part 2: Technical Deep-Dive

### Source Files

| File | Responsibility |
|------|---------------|
| `src/mainwindow.h` | MainWindow class declaration, all UI member variables, process pointers, enums |
| `src/mainwindow.cpp` | All application logic: UI setup, git operations, AI generation, settings, project management |
| `src/diffviewer.h` / `.cpp` | Custom `QPlainTextEdit` subclass with line numbers and diff syntax highlighting |
| `src/model_manager_bridge.h` / `.cpp` | C++ wrapper around the `ai_core` C FFI: loading, streaming inference, commit-message generation |
| `src/vcs_bridge.h` / `.cpp` | C++ wrapper around the `vcs_core` C FFI: SSH key listing/generation/fingerprinting, connection tests, remote add/edit/rename/remove |
| `src/background_download.h` / `.cpp` | Detached "model installer" mode for background model downloads (sidecar status files, cancellation) |
| `src/main.cpp` | Entry point, creates `QApplication` and `MainWindow` |
| `crates/ai_core/` | Rust crate: local GGUF inference (`inference.rs`), model download/discovery (`download.rs`, `discovery.rs`), Conventional Commits message generation (`commit_message.rs`), and the C ABI (`ffi.rs`, `ai_core.h`) |
| `crates/vcs_core/` | Rust crate: SSH key management (`ssh.rs`), SSH connection testing via `russh` (`connect.rs`), git remote read/write via `gix-config` (`remote.rs`), and the C ABI (`ffi.rs`, `vcs_core.h`) |

### Build System

LazyDesktop builds with **XMake** (`xmake.lua`). The build:

1. Runs `cargo build --lib` for `crates/ai_core` and `crates/vcs_core` in
   `before_build` hooks (release profile when building in release mode).
2. Links the resulting static libraries (`add_links("ai_core")`,
   `add_links("vcs_core")`) from `target/debug` or `target/release`.
3. Enables `cxx23`, the Qt Widgets rule, and platform-specific system
   libraries (`pthread`, `dl`, `rt`, `gomp` on Linux; OpenMP on macOS; `/EHsc`
   on Windows).
4. On Linux/macOS, passes `--allow-multiple-definition` to the linker because
   each Rust staticlib embeds its own copy of Rust's std; the first (identical)
   definition wins.

A [`justfile`](justfile) wraps common workflows: `just setup`, `just build`,
`just run`, `just test` (cargo tests), `just format`, `just tidy`,
`just lint`, plus `docker-*` and `release-*` recipes.

### Git Integration

All git operations are performed by shelling out to the `git` CLI via
`QProcess`. There is no libgit2 dependency. This keeps the binary small and
ensures behavioral parity with the command line.

**Process management:**

- `m_gitProcess` — General-purpose git query (status, unpushed files)
- `m_commitProcess` — Commit execution
- `m_pushProcess` — Push/Fetch/Pull
- `m_branchProcess` — Branch listing
- `m_checkoutProcess` — Branch checkout
- `m_createBranchProcess` — New branch creation
- `m_logProcess` — Commit history
- `m_commitDetailProcess` — Files changed in a commit
- `m_stageProcess` — Staging/unstaging files
- `m_installProcess` — Git installation
- `m_authProcess` — Auth checking

**Status query flow:**

1. `startGitStatusQuery()` runs `git status --porcelain`
2. `onGitProcessFinished()` parses the two-character XY status codes
3. Files are added to the tree widget with colored status icons and checkboxes
4. `startGitUnpushedQuery()` then runs `git diff --name-only @{u}..HEAD` to
   show unpushed files

**Commit flow:**

1. User writes summary + optional description
2. Checked files are collected via `checkedFiles()`
3. Files are staged with `git add -- <files>`
4. Commit is executed with `git commit [-m summary] [-m description] [--no-verify]`
5. On success, status and log are refreshed

**Push/Fetch/Pull state machine:**

- Starts in `Push` state
- On push success with "Everything up-to-date" → switches to `Fetch`
- On fetch, checks `git rev-list --count HEAD..@{u}` → if behind, switches to
  `Pull`
- On push rejection (non-fast-forward) → switches to `Pull`
- After pull → back to `Push`

### AI System

The AI system supports two categories of providers: cloud APIs and local
inference.

#### Cloud Providers

All cloud providers use `QNetworkAccessManager` for HTTP requests.

| Provider | Endpoint | Auth Method |
|----------|----------|-------------|
| OpenRouter | `openrouter.ai/api/v1/chat/completions` | Bearer token |
| OpenAI | `api.openai.com/v1/chat/completions` | Bearer token |
| Anthropic | `api.anthropic.com/v1/messages` | `x-api-key` header |
| Google AI Studio | `generativelanguage.googleapis.com/v1beta/models/...` | Query parameter |

Each provider has a different response format. `extractAiText()` normalizes
the response:

- OpenAI/OpenRouter: `choices[0].message.content`
- Anthropic: `content[0].text`
- Google AI Studio: `candidates[0].content.parts[0].text`

#### Local inference (`ai_core` Rust crate)

Local GGUF inference lives in the `crates/ai_core` Rust crate rather than in
C++. The crate is built as a `staticlib` and linked into the app; the C++ side
talks to it through a small C ABI declared in `crates/ai_core/ai_core.h`:

1. `ModelManagerBridge` (C++) owns the FFI handle; `model_manager_bridge.cpp`
   wraps the exported functions in Qt-friendly signals (`inferenceToken`,
   `inferenceFinished`, `inferenceFailed`, `inferenceCancelled`).
2. The crate loads a GGUF file via `llama_cpp_2` and runs inference on a
   dedicated thread, with an `Arc<AtomicBool>` cancel flag. Worker threads
   never touch the manager, so the handle can be safely torn down on exit.
3. Tokens stream back through an `on_token` callback for real-time display;
   the full text is accumulated and returned on `on_finish`.
4. A single `inference_running` flag serializes inference requests.

**Commit message generation** (`commit_message.rs`): the UI collects VCS
context — vcs kind, repo path, diff, changed files, staged files, current
branch, recent commit messages — into a JSON `CommitContext`. The crate builds
a prompt (supporting a `<diff>` placeholder), runs inference, and normalizes
the raw output into a clean Conventional Commits message
(`type(scope): subject`). This is the crate's only application-level feature;
everything else in `ai_core` is generic model management.

**FFI entry points:**

| Function | Purpose |
|----------|---------|
| `mm_init` / `mm_destroy` | Manager lifecycle |
| `mm_stream_inference` | Generic completion (`prompt` argument) |
| `mm_generate_commit_message` | Commit-message generation (`context_json` argument) |
| `mm_download_model` / `mm_cancel_download` | HuggingFace model downloads |
| `mm_list_local_models` / `mm_discover_models` | Model listing (JSON) |
| `mm_delete_model` | Delete a model file |
| `mm_free_string` | Free strings returned by the crate |

**Backend selection (GPU vs CPU):** `discovery.rs` scans for available
`ggml` backends; the C++ UI toggles offloading via the `ai/gpu_acceleration`
setting (mapped to `n_gpu_layers`, `99` when enabled).

**Available models (built-in catalog):**

| Model | Quantization | Size |
|-------|-------------|------|
| Qwen3-1.7B | Q8_0 | ~1.8 GB |
| Qwen2.5-0.5B | Q5_0 | ~500 MB |
| TinyLlama-1.1B-Chat | Q4_K_M | ~669 MB |
| Llama-3.2-1B-Instruct | Q4_K_M | ~808 MB |
| SmolLM2-1.7B-Instruct | Q4_K_M | ~1.06 GB |
| Gemma-2-2B-it | Q4_K_M | ~1.71 GB |
| Phi-3.5-mini-instruct | Q4_K_M | ~2.39 GB |

Models are downloaded from HuggingFace with progress tracking and can be
selected/deleted in Settings → AI.

#### Background model downloads

Large model downloads do not block the UI. When the user starts a download:

1. `spawnBackgroundDownload()` relaunches the app binary in a detached
   "model installer" mode: `lazydesktop --background-dl <url> <dest> <sha256>
   <modelsDir> <configPath>`.
2. The worker downloads `<url>` to `<dest>` via a `.part` file, verifies the
   SHA-256 checksum, and exits.
3. Progress/completion is tracked through JSON sidecar files written next to
   the model: a status file (`status` field, `received`/`total` bytes, `pid`),
   a cancel file (request cancellation), and the `.part` file itself.
4. The UI polls the sidecar to update progress, detect completion, and offer
   cancellation; downloads survive closing and reopening the app.

See `src/background_download.h` for the sidecar path helpers
(`bgStatusPath`, `bgCancelPath`, `bgPartialPath`) and `bgProcessAlive()`.

#### AI Prompt System

- The raw prompt from settings can contain a `<diff>` placeholder where the
  actual git diff goes; `buildAiPrompt()` substitutes the diff text for cloud
  providers, and `buildPrompt` in `commit_message.rs` does the same for local
  inference.
- Two separate prompts exist: one for commit message summary, one for
  description (`ai/system_prompt`, `ai/description_system_prompt`).
- For local inference the UI gathers VCS context (diff, files, staged set,
  branch, recent messages) into a `CommitContext` JSON object instead of a
  bare prompt.
- Fields are disabled and an overlay with "AI is thinking..." is shown during
  generation, with a "Show more" expandable view of the raw text.

#### AI Editor Skills

The repo ships skills that teach AI coding tools the project's commit and
branch conventions, installed for OpenCode in `.opencode/skills/`.

Each directory contains `commit` and `create-branch` skills. Both auto-detect
Git vs Jujutsu (preferring `.jj` in a colocated repo). `commit` produces a
Conventional Commits message (`type(scope): subject`) and commits only after
approval; `create-branch` names branches `type/scope?/short-description`
(e.g. `feat/vcs/jj-support`).

### UI Architecture

#### Main Window Layout

```
QMainWindow
└── centralWidget (QWidget)
    ├── topLayout (QHBoxLayout)
    │   ├── projectButton ("Open Folder" / repo name)
    │   ├── branchComboBox
    │   ├── deleteBranchButton
    │   └── pushButton ("Push" / "Fetch" / "Pull")
    ├── contentLayout (QHBoxLayout)
    │   ├── recentDrawer (overlay, 260px, hidden by default)
    │   │   ├── addProjectButton (+ Add dropdown)
    │   │   ├── scanButton
    │   │   ├── recentList (QListWidget)
    │   │   └── clearAllBtn
    │   └── splitter (QSplitter, Horizontal)
    │       ├── sidebarTabs (QTabWidget)
    │       │   ├── Tab "Changes"
    │       │   │   ├── gitStatusTree (QTreeWidget, checkboxes)
    │       │   │   └── commitContainer
    │       │   │       ├── summaryInput
    │       │   │       ├── descriptionInput
    │       │   │       ├── aiRow (AI, skip hooks, co-authors)
    │       │   │       └── commitButton
    │       │   └── Tab "History"
    │       │       ├── commitHistoryList (QListWidget + CommitDelegate)
    │       │       └── commitFilesContainer
    │       └── viewerStack (QStackedWidget)
    │           ├── [0] DiffViewer
    │           ├── [1] Placeholder ("No file selected")
    │           └── [2] BinaryPreview (QLabel for images)
```

#### CommitDelegate

A custom `QStyledItemDelegate` that renders commit history items with three
lines:

1. Hash — monospace, small, gray
2. Subject — bold
3. Author + date — smaller, gray

Each item takes `height * 3 + 4` pixels of vertical space.

#### DiffViewer

A `QPlainTextEdit` subclass with:

- `LineNumberArea` widget painted to the left margin
- `DiffHighlighter` (`QSyntaxHighlighter`) that colorizes:
  - Added lines: green background
  - Deleted lines: red background
  - Hunk headers (`@@`): blue background

Image files (png, jpg, jpeg, gif, bmp, webp, svg, ico, tiff) render inline via
a `QLabel`, scaled to a maximum of 800×600 while preserving aspect ratio.
Video files show a placeholder message.

### Settings System

Uses `QSettings` with INI format. All settings are stored in
`~/.config/lazydesktop/lazydesktop.conf`.

**Settings categories:**

| Key | Default | Purpose |
|-----|---------|---------|
| `appearance/theme` | `"system"` | Active theme name |
| `git/user.name` | — | Global git user name |
| `git/user.email` | — | Global git user email |
| `ai/enabled` | `false` | AI features toggle |
| `ai/provider` | `"OpenRouter"` | Selected AI provider |
| `ai/api_key` | — | API key for cloud providers |
| `ai/model` | `"gpt-4o-mini"` | Model name |
| `ai/system_prompt` | (built-in) | Commit message generation prompt |
| `ai/description_system_prompt` | (built-in) | Description generation prompt |
| `ai/local_model_path` | — | Path to active GGUF model |
| `ai/gpu_acceleration` | `true` | GPU offload toggle |
| `ai/local_models` | `[]` | JSON array of downloaded model metadata |
| `paths/projects` | `~/.config/lazydesktop/projects.yaml` | Project list path |
| `paths/settings` | `~/.config/lazydesktop/lazydesktop.conf` | Settings path |
| `paths/themes` | `~/.config/lazydesktop/themes` | Themes directory |

The Settings dialog is organized into **Appearance**, **Git**, and **AI**
pages. The AI page (rebuilt in the v0.2 era) covers provider selection, API
key, model, system prompts, GPU acceleration, and local model management
(download, select, delete).

### Theme System

Themes are YAML files in `~/.config/lazydesktop/themes/` with a `.theme.yaml`
extension:

```yaml
name: "Theme Name"
colors:
  background: "#hex"
  foreground: "#hex"
  widget_background: "#hex"
  input_background: "#hex"
  input_foreground: "#hex"
  button_background: "#hex"
  button_foreground: "#hex"
  tooltip_background: "#hex"
  tooltip_foreground: "#hex"
  selection: "#hex"
```

`generateStylesheet()` converts these into a Qt stylesheet string applied to
the `QApplication`. The built-in "Dark" theme uses hardcoded VS Code-style
colors. The theme picker in Settings → Appearance scans the themes directory
and lets the user switch instantly.

### Auto-Refresh

When a repository is opened:

1. `QFileSystemWatcher` monitors `.git/`, `.git/index`, and `.git/HEAD`
2. Any change triggers `onRepoDirChanged()` which starts a 2-second debounce
   timer
3. After the debounce, `startGitStatusQuery()` re-queries
   `git status --porcelain`
4. Refreshes are skipped while a commit is in progress

### Credential Handling

1. `checkGitAuth()` runs `git ls-remote --exit-code` to test authentication
2. On failure (exit code 128), `showAuthDialog()` presents a credential form
3. `setupAskPass()` creates a temporary shell script that responds to
   `GIT_ASKPASS` prompts
4. The script is set as the `GIT_ASKPASS` environment variable on
   push/fetch/pull processes
5. `cleanupAskPass()` removes the temporary script on shutdown

### Git Bootstrapping

On startup, `checkGitAvailable()` checks if `git` is on PATH. If not:

- **Linux**: Tries `apt-get`, `dnf`, or `pacman` via `pkexec`/`sudo`
- **macOS**: Runs `xcode-select --install`
- **Windows**: Runs `winget install Git.Git`

### Docker

The Docker image (`Dockerfile`) builds the C++ UI and the bundled Rust
`ai_core` crate, then ships only the runtime (no toolchain). The entrypoint
(`docker/entrypoint.sh`) picks a display backend:

- **Host X11** when an X socket is mounted and `DISPLAY` is set
- **Xvfb + VNC/noVNC** otherwise, or when `VNC_MODE=1` (browser at
  `http://localhost:6080`)

`PUID`/`PGID` re-map the app user so mounted config is not root-owned. The
compose file mounts the host X socket, a named volume for config
(`lazydesktop-config`), and `./repos` → `/workspace` for your repositories.

---

## Part 3: Feature Checklist

### Git Operations

- [x] Open folder as Git repository
- [x] Validate `.git` directory existence
- [x] `git status --porcelain` parsing with XY status codes
- [x] Colored status indicators (M=yellow, D=red, A=green, R=purple, ?=gray)
- [x] Per-file checkboxes for staging selection
- [x] Master "Select All" checkbox
- [x] Commit with summary + optional description (`git commit -m`)
- [x] Skip pre-commit hooks (`--no-verify`)
- [x] Co-author trailers from git log history
- [x] Push (`git push`)
- [x] Fetch (`git fetch`)
- [x] Pull (`git pull`)
- [x] Smart push/fetch/pull state cycling
- [x] Branch listing (`git branch -a`)
- [x] Branch switching (`git checkout`)
- [x] Branch creation (`git checkout -b`)
- [x] Branch deletion (`git branch -D`)
- [x] Uncommitted changes warning before branch switch
- [x] Commit history (`git log --pretty=format`)
- [x] Commit file drill-down (`git diff-tree`)
- [x] File diff from history (`git show`)
- [x] Discard changes (`git restore`)
- [x] Delete untracked files (`QFile::remove`)
- [x] Clone repository (`git clone`)
- [x] Create repository (`git init`)
- [x] Close repository (reset all state)
- [x] Auto-refresh via QFileSystemWatcher (2s debounce)
- [x] Refresh skipped during active commits

### Diff Viewer

- [x] Custom QPlainTextEdit-based viewer
- [x] Line numbers (LineNumberArea widget)
- [x] Diff syntax highlighting (additions=green, deletions=red, hunk=blue)
- [x] Image preview (png, jpg, jpeg, gif, bmp, webp, svg, ico, tiff)
- [x] Video file placeholder message
- [x] Image scaling (max 800x600, aspect ratio preserved)

### AI Commit Message Generation

- [x] OpenRouter provider (API key required)
- [x] OpenAI provider (API key required)
- [x] Anthropic provider (API key required)
- [x] Google AI Studio provider (API key required)
- [x] Local GGUF inference via the Rust `ai_core` crate (no key needed)
- [x] Separate summary and description generation
- [x] Configurable system prompts with `<diff>` placeholder
- [x] Right-click provider switching on AI button
- [x] AI thinking indicator overlay with "Show more" expandable text
- [x] Fields disabled during generation
- [x] Automatic summary/description parsing from AI response

### Local AI (`ai_core` Rust crate)

- [x] GGUF model loading via `llama-cpp-2` (Rust crate)
- [x] Dedicated inference thread (blocking core, non-blocking UI)
- [x] Token-by-token streaming (inferenceToken signal)
- [x] GPU acceleration toggle (n_gpu_layers offload)
- [x] CPU/GPU backend discovery
- [x] Built-in model catalog (7 models from HuggingFace)
- [x] One-click model download with progress bar
- [x] Background downloads (detached `--background-dl` worker + sidecar status)
- [x] Model selection (set as active)
- [x] Model deletion with disk space info
- [x] Dedicated commit-message FFI (`mm_generate_commit_message`) that
      normalizes output to Conventional Commits
- [x] Single in-flight inference guard (`inference_running`)

### Project Management

- [x] Recent projects list (persistent YAML storage, max 100)
- [x] Projects grouped by remote owner (extracted from origin URL)
- [x] Dirty repo indicator (yellow dot)
- [x] Scan folder for Git repos (bulk import)
- [x] Remove individual project (with option to delete directory)
- [x] Clear all projects (with confirmation)
- [x] Add project: Clone Repository
- [x] Add project: Create Repository
- [x] Add project: Load Existing
- [x] SSH to HTTPS URL normalization for owner extraction

### UI

- [x] Menu bar: Files menu (Open in Editor, File Manager, Terminal, GitHub,
      Settings)
- [x] Menu bar: View menu (Commit Panel, Commit Files toggles)
- [x] Commit panel with close button
- [x] Commit files panel in History tab with close button
- [x] Stacked widget viewer (diff / placeholder / image)
- [x] Overlay drawer for recent projects with drop shadow
- [x] Custom CommitDelegate for history items (3-line rendering)

### Settings

- [x] General: Data path configuration (projects, settings, themes)
- [x] Appearance: Theme selection (System Default, Dark, custom) with theme
      picker
- [x] Git: Global user.name and user.email (read/written via
      `git config --global`)
- [x] AI: Enable/disable toggle
- [x] AI: Provider selection dropdown
- [x] AI: API key input (password masked)
- [x] AI: Model selection (editable combo, auto-fetched from API)
- [x] AI: System prompt editor for commit messages
- [x] AI: System prompt editor for descriptions
- [x] AI: Local model management (download, select, delete)
- [x] AI: GPU acceleration checkbox

### Themes

- [x] System Default (no stylesheet)
- [x] Dark theme (VS Code-style colors)
- [x] Custom YAML themes from `~/.config/lazydesktop/themes/`
- [x] Theme appears in Settings after directory scan
- [x] Stylesheet generation from YAML color map

### Git Bootstrapping

- [x] Detect missing Git on startup
- [x] Linux: auto-install via apt-get/dnf/pacman with privilege escalation
- [x] macOS: xcode-select --install
- [x] Windows: winget install

### Credential Handling

- [x] GIT_ASKPASS integration
- [x] Credential dialog (host, username, token)
- [x] Temporary askpass script creation
- [x] Script cleanup on shutdown
- [x] Auth environment setup for push/fetch/pull

### SSH Keys (native `vcs_core` crate)

- [x] Generate Ed25519 / RSA-4096 keypairs (optionally passphrase-encrypted)
- [x] List public keys in `~/.ssh` (public material only, never private)
- [x] SHA256 fingerprint via `ssh-key` crate (no `ssh-keygen` subprocess)
- [x] Copy public key to clipboard
- [x] Delete keypair with confirmation
- [x] Connection test via `russh` (batch mode, 10s timeout, no prompts)

### Remotes (native `vcs_core` crate)

- [x] List remotes with URLs (`gix-config`, no `git remote` subprocess)
- [x] Add remote (name + URL, installs default fetch refspec)
- [x] Edit remote (rename with refspec re-keying, set URL)
- [x] Remove remote with confirmation
- [x] Copy URL to clipboard

### Bug Fixes (v0.2)

- [x] FileTreeDelegate staging checkboxes respond to clicks (Qt
      `editorEvent` handling)
- [x] "View on GitHub" opens SSH-style remotes via URL normalization

### Build, Packaging, and CI

- [x] XMake build system (links the Rust `ai_core` crate via its C FFI)
- [x] PKGBUILD for Arch Linux
- [x] Debian packaging (control, rules, changelog)
- [x] AppImage packaging (linuxdeploy + Qt plugin)
- [x] Windows MSI packaging (WiX toolset)
- [x] Docker packaging (X11 or VNC/noVNC)
- [x] GitHub Actions CI (Ubuntu / Windows / macOS matrix + Rust tests +
      pre-commit)
- [x] GitHub Actions release workflow (tag-driven, multi-artifact, XMake
      builds)
- [x] CodeQL static analysis workflow
- [x] Dependabot config (GitHub Actions + Cargo)
- [x] Desktop entry file

---

## Part 4: Addon System (`crates/addons`) — Implementation Status

> Session summary: Phase 2 of the addon system (ADDON_SPEC.md). This part
> documents the current state of the `lazydesktop-addons` Rust security core,
> the in-flight test fixes (root causes identified), and the performance
> roadmap.

### 4.1 Overview

The addon system adds scriptable, sandboxed extensions to LazyDesktop. Rust is
the trust boundary: archive parsing, directory loading, ignore rules, manifest
validation, path security, and resource access are enforced in Rust; C++/Qt
only consumes validated data through a C ABI.

**Deliverables so far:**

- `ADDON_SPEC.md` — full design specification (package format, `.lzdignore`
  system, security model, provider architecture, FFI ABI, error model, Lua
  execution model, testing strategy).
- `crates/addons/` — new workspace member `lazydesktop-addons`
  (`staticlib`, C ABI via `include/addons.h`, `#![deny(unsafe_code)]` with
  `unsafe` isolated to `ffi.rs`).
- Workspace wiring: `crates/addons` added to `Cargo.toml` members and
  `Cargo.lock`.

**Module layout:**

| Module | Responsibility |
|--------|---------------|
| `archive.rs` | `.zip`/`.lzd` streaming extraction, path-security, size/entry limits, symlink handling, duplicate detection, ignore rules, manifest peeking |
| `pathsec.rs` | path normalization, traversal rejection, lexical/canonical containment, symlink target policy |
| `ignore.rs` | `.lzdignore` parsing + gitignore-style matching (`*`, `?`, `[...]`, `**`, `!`, escapes), default ignores |
| `manifest.rs` | `config.toml` schema + ordered validation (semver, API version, ID rules) |
| `registry.rs` | provider aggregation, snapshot index (`by_id`), install/uninstall/open/read-asset |
| `provider/` | `AddonProvider` trait; `local.rs` (dirs/zips/`.lzd` scanning, shadowing/conflict resolution), `remote.rs` (design-ready stub returning `NotSupported`) |
| `error.rs` | structured `AddonError` taxonomy + sanitized JSON serialization (never leaks absolute paths) |
| `ffi.rs` + `include/addons.h` | C ABI (`lda_*` functions, `lda_result` codes, error JSON contract) |
| `json.rs`, `package.rs` | JSON helpers, descriptor/package model |

### 4.2 Test status (session start)

`cargo test -p lazydesktop-addons`: **59 passed, 14 failed.** No production
code changes had been made yet; the failures below had been fully diagnosed
with concrete fix plans.

### 4.3 Failing tests — root causes and fix plans

**`archive.rs` (4 tests) — all test-fixture bugs:**

1. `extract_rejects_duplicate_entries` — the `zip` crate's `ZipWriter`
   rejects duplicate raw filenames (`start_file("x.txt")` twice panics on
   `unwrap`). Fix: write `x.txt` + `./x.txt` (distinct raw names, same
   normalized path) so the crate's own normalized `seen`-set duplicate
   detection fires.
2. `content_size_counts_only_non_ignored` — fixture writes
   `root/.git/config` without creating the `.git` directory first
   (`fs::write` does not create parents) → `NotFound`. Fix: create `.git`
   before writing.
3./4. `symlink_within_root_is_extracted` and `symlink_escaping_root_is_rejected`
   — `zip::write::SimpleFileOptions::unix_permissions(0o120777)` masks the
   mode with `& 0o777` (zip crate `write.rs`), stripping the `S_IFLNK`
   (0o120000) type bit, so `entry.is_symlink()` returns false and the entry
   extracts as a regular file. Fix: build the fixture with hand-crafted raw
   zip bytes whose central-directory `external_attributes = 0o120777 << 16`
   and `version_made_by` high byte = 3 (Unix) — the read path maps
   `unix_mode()` to `external_attributes >> 16`.

**`error.rs` (1 test) — production bug:**

5. `sanitizer_keeps_urls` — `sanitize_message` destroys `https://...` URLs:
   the Windows drive-prefix branch fires on the `s` in `https:` (any letter
   followed by `:`), and the absolute-path branch redacts `//example.com/addon`.
   Fix: only treat a drive prefix or absolute path as such when it starts a
   token (`i == 0` or previous char is whitespace).

**`ignore.rs` (2 tests) — production + reference bugs:**

6. `double_star_crosses_directories` — a non-dir-only pattern (`a/**/b`)
   matching a path *prefix* sets `excluded_ancestor = true`, so `a/x/b/y` is
   reported ignored; the contract says only directory-only patterns
   (trailing `/`) may exclude a whole subtree. Fix: in `is_ignored`, only
   `dir_only` patterns may claim ancestor directories.
7. `proptest_tests::matches_reference_implementation` — the naive reference
   matcher treats a `Glob` pattern as matching only when `g == "*" || g == s`,
   so `*.txt` never matches `x.txt` in the reference. Fix: make the reference
   use the production `glob_segment_match` for glob segments and mirror the
   dir-only-prefix rule.

**`manifest.rs` (3 tests):**

8. `id_validator` — `"a"*65 + ".b"` accepted because only the *total* length
   (≤ 128) is enforced. Fix: add a per-segment maximum (64, DNS-label style).
9. `rejects_bad_ids` — all-numeric `"1.2"` passes the `^[a-z0-9]+(\.[a-z0-9]+)+$`
   regex. Fix: require at least one ASCII letter per segment (reverse-DNS
   convention; `a0.b1.c2` still valid).
10. `rejects_bad_semver` — `"1.2.3-beta+ok"` is *valid* semver 2.0.0 (the
    spec mandates the `semver` crate), so the test expectation is wrong.
    Fix: remove that entry from the bad list.

**`pathsec.rs` (1 test) — production bug:**

11. `symlink_target_validation` — `validate_symlink_target` rejects
    `RootDir`/`Prefix` components in the *joined* path. In real extraction
    `entry_dir` is absolute (derived from `dest`), so `entry_dir.join("../src/main.lua")`
    starts with `RootDir` and is wrongly rejected as "must be relative".
    Fix: allow `RootDir`/`Prefix` components (the *target string* itself is
    still required to be relative/backslash-free), reject pops past the root,
    and require the resolved path to be *strictly* inside `root` (rejecting a
    target that resolves exactly to the root, e.g. `..`).

**`registry.rs` (2 tests) — same production bug:**

12. `user_root_wins_over_system` — `AddonRegistry::load()` computes the full
    list (winners + shadowed + error descriptors) but stores only `by_id`
    (winners) in `RegistrySnapshot`, so `list()` drops shadowed/error entries
    (expected 2, got 1).
13. `invalid_manifest_is_surfaced_not_fatal` — same root cause: error
    descriptors never make it into the snapshot. Fix: snapshot stores the full
    `all` descriptor list alongside `by_id` (lookups still resolve winners).

**`provider/local.rs` (1 test) — production bug:**

14. `open_after_install_and_uninstall` — `LocalEntry.user` is hard-coded to
    `false` in `load_directory`/`load_archive`, and `scan()` never propagates
    the root's `user` flag into `entries_map`, so uninstalling a user-root
    addon wrongly returns `ReadOnly`. Fix: set `entry.user` from the root flag
    when building `entries_map` in `scan()`.

### 4.4 Performance roadmap (directive)

Audit and optimize the Rust backend concurrency/performance:

- **`tokio`** — offload blocking system calls, background subprocesses (git
  execution), and network/remote fetches to async tasks; keep the FFI ⇄
  C++/Qt event-loop hand-off non-blocking.
- **`rayon`** — convert CPU/memory-bound loops (multi-entry zip extraction,
  git status scanning, commit-graph metrics) to parallel iterators.
- **Ecosystem crates** — `flume` or `crossbeam-channel` for lock-free FFI
  message passing; `dashmap` for concurrent state/cache maps.
- **Verification** — `cargo clippy --all-targets -D warnings` and
  `cargo test` must stay clean.

### 4.5 Next steps

1. Apply the 14 diagnosed fixes (fixture + production), then get the suite
   green.
2. Implement the concurrency work above with clippy `-D warnings` clean.
3. Commit and push.

---

## Known Notes / Caveats

- **Packaging build system drift** — The Debian rules (`debian/rules` uses
  `--buildsystem=meson`) and the Arch `PKGBUILD` still reference the
  Meson/Ninja build from before the XMake migration, and no `meson.build` is
  present in the tree. The CI and release workflows build with XMake and do
  not use these scripts; migrating them to XMake before they are used to
  build source packages is tracked on the roadmap.
- **AI quality depends on the model/provider** — small local models (e.g.
  0.5B) produce usable but terse commit messages; larger models and cloud
  providers generally produce better summaries.
