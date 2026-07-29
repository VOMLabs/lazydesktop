# LazyDesktop — Implementation Details

Comprehensive documentation of what is implemented in LazyDesktop, how it works internally, and what each component does.

---

## Part 1: High-Level Overview

### What is LazyDesktop?

LazyDesktop is a native Git GUI client for KDE Plasma, built as a lightweight alternative to GitHub Desktop. It uses the same technology stack as KDE itself (Qt 6, C++23) so it integrates seamlessly with the desktop without requiring additional runtimes like Electron.

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
│  - branch, checkout, log          │  - Local llama.cpp   │
│  - clone, init                    │  - LlamaWorker thread│
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
| Build System | Meson + Ninja | Compilation and linking |
| Config Storage | yaml-cpp | YAML parsing for projects and themes |
| Local AI | llama.cpp (subproject) | GGUF model inference for commit message generation |

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

| File | Lines | Responsibility |
|------|-------|----------------|
| `src/mainwindow.h` | 241 | MainWindow class declaration, all UI member variables, process pointers, enums |
| `src/mainwindow.cpp` | ~4131 | All application logic: UI setup, git operations, AI generation, settings, project management |
| `src/diffviewer.h` / `.cpp` | ~200 | Custom `QPlainTextEdit` subclass with line numbers and diff syntax highlighting |
| `src/llamaai.h` / `.cpp` | ~460 | llama.cpp integration: model loading, inference on a worker thread, GPU/CPU backend selection |
| `src/main.cpp` | minimal | Entry point, creates `QApplication` and `MainWindow` |


### Git Integration

All git operations are performed by shelling out to the `git` CLI via `QProcess`. There is no libgit2 dependency. This keeps the binary small and ensures behavioral parity with the command line.

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
4. `startGitUnpushedQuery()` then runs `git diff --name-only @{u}..HEAD` to show unpushed files

**Commit flow:**
1. User writes summary + optional description
2. Checked files are collected via `checkedFiles()`
3. Files are staged with `git add -- <files>`
4. Commit is executed with `git commit [-m summary] [-m description] [--no-verify]`
5. On success, status and log are refreshed

**Push/Fetch/Pull state machine:**
- Starts in `Push` state
- On push success with "Everything up-to-date" → switches to `Fetch`
- On fetch, checks `git rev-list --count HEAD..@{u}` → if behind, switches to `Pull`
- On push rejection (non-fast-forward) → switches to `Pull`
- After pull → back to `Push`

### AI System

The AI system supports two categories of providers: cloud APIs and local inference.

#### Cloud Providers

All cloud providers use `QNetworkAccessManager` for HTTP requests.

| Provider | Endpoint | Auth Method |
|----------|----------|-------------|
| OpenRouter | `openrouter.ai/api/v1/chat/completions` | Bearer token |
| OpenAI | `api.openai.com/v1/chat/completions` | Bearer token |
| Anthropic | `api.anthropic.com/v1/messages` | `x-api-key` header |
| Google AI Studio | `generativelanguage.googleapis.com/v1beta/models/...` | Query parameter |

Each provider has a different response format. `extractAiText()` normalizes the response:
- OpenAI/OpenRouter: `choices[0].message.content`
- Anthropic: `content[0].text`
- Google AI Studio: `candidates[0].content.parts[0].text`

#### Local llama.cpp

The local AI path uses the bundled llama.cpp library:

1. `LlamaAI` manages a `LlamaWorker` running on a separate `QThread`
2. `LlamaWorker::loadModel()` loads a GGUF model file via `llama_model_load_from_file()`
3. Inference runs with `llama_decode()`, sampling tokens one at a time
4. Each token is emitted via `thinking()` signal for real-time display
5. The result is emitted via `finished()` signal

**Backend selection (GPU vs CPU):**
- When GPU acceleration is disabled, `loadBestCpuBackend()` scans for `libggml-cpu-*.so` files and loads only the best-scoring CPU backend
- When enabled, `ggml_backend_load_all()` loads all available backends including CUDA/Vulkan
- The backend registry is a process-wide singleton initialized once via `std::call_once`

**Available models (built-in catalog):**
- Qwen3-1.7B (Q8_0) — ~1.8 GB
- Qwen2.5-0.5B (Q5_0) — ~500 MB
- TinyLlama-1.1B-Chat (Q4_K_M) — ~669 MB
- Llama-3.2-1B-Instruct (Q4_K_M) — ~808 MB
- SmolLM2-1.7B-Instruct (Q4_K_M) — ~1.06 GB
- Gemma-2-2B-it (Q4_K_M) — ~1.71 GB
- Phi-3.5-mini-instruct (Q4_K_M) — ~2.39 GB

Models are downloaded from HuggingFace with progress tracking and can be selected/deleted in Settings → AI.

#### AI Prompt System

The prompt system uses a `<diff>` placeholder:
- The raw prompt from settings contains `<diff>` where the actual git diff should go
- `buildAiPrompt()` substitutes the diff text into the prompt
- Two separate prompts exist: one for commit message summary, one for description
- Fields are disabled and an overlay with "AI is thinking..." is shown during generation

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

A custom `QStyledItemDelegate` that renders commit history items with three lines:
1. Hash — monospace, small, gray
2. Subject — bold
3. Author + date — smaller, gray

Each item takes `height * 3 + 4` pixels of vertical space.

#### DiffViewer

A `QPlainTextEdit` subclass with:
- `LineNumberArea` widget painted to the left margin
- `DiffHighlighter` (QSyntaxHighlighter) that colorizes:
  - Added lines: green background
  - Deleted lines: red background
  - Hunk headers (`@@`): blue background

### Settings System

Uses `QSettings` with INI format. All settings are stored in `~/.config/lazydesktop/lazydesktop.conf`.

**Settings categories:**

| Key | Default | Purpose |
|-----|---------|---------|
| `appearance/theme` | `"system"` | Active theme name |
| `git/user.name` | — | Global git user name |
| `git/user.email` | — | Global git user email |
| `ai/enabled` | `false` | AI features toggle |
| `ai/provider` | `"OpenRouter"` | Selected AI provider |
| `ai/api_key` | — | API key for cloud providers |
| `ai/model` | `"deepseek/deepseek-v4-flash"` | Model name |
| `ai/system_prompt` | (built-in) | Commit message generation prompt |
| `ai/description_system_prompt` | (built-in) | Description generation prompt |
| `ai/local_model_path` | — | Path to active GGUF model |
| `ai/gpu_acceleration` | `true` | GPU offload toggle |
| `ai/local_models` | `[]` | JSON array of downloaded model metadata |
| `paths/projects` | `~/.config/lazydesktop/projects.yaml` | Project list path |
| `paths/settings` | `~/.config/lazydesktop/lazydesktop.conf` | Settings path |
| `paths/themes` | `~/.config/lazydesktop/themes` | Themes directory |

### Theme System

Themes are YAML files in `~/.config/lazydesktop/themes/` with `.theme.yaml` extension:

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
```

`generateStylesheet()` converts these into a Qt stylesheet string applied to the `QApplication`. The built-in "Dark" theme uses hardcoded VS Code-style colors.

### Auto-Refresh

When a repository is opened:
1. `QFileSystemWatcher` monitors `.git/`, `.git/index`, and `.git/HEAD`
2. Any change triggers `onRepoDirChanged()` which starts a 2-second debounce timer
3. After the debounce, `startGitStatusQuery()` re-queries `git status --porcelain`
4. Refreshes are skipped while a commit is in progress

### Credential Handling

1. `checkGitAuth()` runs `git ls-remote --exit-code` to test authentication
2. On failure (exit code 128), `showAuthDialog()` presents a credential form
3. `setupAskPass()` creates a temporary shell script that responds to GIT_ASKPASS prompts
4. The script is set as `GIT_ASKPASS` environment variable on push/fetch/pull processes
5. `cleanupAskPass()` removes the temporary script on shutdown

### Git Bootstrapping

On startup, `checkGitAvailable()` checks if `git` is on PATH. If not:
- **Linux**: Tries `apt-get`, `dnf`, or `pacman` via `pkexec`/`sudo`
- **macOS**: Runs `xcode-select --install`
- **Windows**: Runs `winget install Git.Git`

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
- [x] Local llama.cpp provider (no key needed)
- [x] Separate summary and description generation
- [x] Configurable system prompts with `<diff>` placeholder
- [x] Right-click provider switching on AI button
- [x] AI thinking indicator overlay with "Show more" expandable text
- [x] Fields disabled during generation
- [x] Automatic summary/description parsing from AI response

### Local AI (llama.cpp)

- [x] GGUF model loading via llama.cpp C API
- [x] Worker thread for non-blocking inference
- [x] Token-by-token streaming (thinking signal)
- [x] GPU acceleration toggle (CUDA/Vulkan backend loading)
- [x] CPU-only backend selection (avoids loading GPU backends)
- [x] Built-in model catalog (7 models from HuggingFace)
- [x] One-click model download with progress bar
- [x] Model selection (set as active)
- [x] Model deletion with disk space info
- [x] Process-wide backend singleton initialization

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

- [x] Menu bar: Files menu (Open in Editor, File Manager, Terminal, GitHub, Settings)
- [x] Menu bar: View menu (Commit Panel, Commit Files toggles)
- [x] Commit panel with close button
- [x] Commit files panel in History tab with close button
- [x] Stacked widget viewer (diff / placeholder / image)
- [x] Overlay drawer for recent projects with drop shadow
- [x] Custom CommitDelegate for history items (3-line rendering)

### Settings

- [x] General: Data path configuration (projects, settings, themes)
- [x] Appearance: Theme selection (System Default, Dark, custom)
- [x] Git: Global user.name and user.email (read/written via `git config --global`)
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

### Packaging

- [x] Meson build system
- [x] PKGBUILD for Arch Linux
- [x] Debian packaging (control, rules, changelog)
- [x] Desktop entry file
