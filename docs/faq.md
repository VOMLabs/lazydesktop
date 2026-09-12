# FAQ

## General

### What is LazyDesktop?

A fast, native Git GUI client for KDE Plasma, built with Qt 6 and C++23 — a
lightweight alternative to GitHub Desktop without Electron. A pure-Rust
frontend built on GPUI is in development. See the [README](../README.md).

### Is it only for KDE?

It is built with the same stack KDE uses and integrates best with KDE Plasma,
but it is a standard Qt application and works on any Linux desktop that has
Qt 6 available.

### Does it support Windows or macOS?

There is best-effort CI coverage for both (the app builds on all three
platforms), and a Windows MSI installer is produced per release. Linux/KDE is
the primary target and the most tested platform.

## Git

### Which Git operations are supported?

Status, staging, commit (with skip-hooks and co-authors), push/fetch/pull,
branch management, commit history with file drill-down, diff viewing,
clone/init, discard changes, and credential handling. See the
[git workflow](user-guide/git-workflow.md).

### Does it use libgit2?

No. All Git operations run through the `git` CLI via `QProcess`. This keeps
the binary small and guarantees parity with the command line.

### Where is my data stored?

Under `~/.config/lazydesktop/` — see [configuration](getting-started/configuration.md).

## AI

### Do I need an API key?

Only for cloud providers. If you use a [local GGUF model](ai/local-models.md),
no key or internet connection is needed at inference time.

### Are my diffs sent anywhere?

With cloud providers, the diff is sent to the provider as part of the prompt.
With local models, everything stays on your machine.

### Can I change the AI prompt?

Yes — both the summary and description prompts are editable in
Settings → AI. The `<diff>` placeholder is replaced with the actual diff.

### How do AI editor skills relate to the app?

The skills (`.opencode/skills/`) teach AI coding tools the repository's commit
and branch conventions. They are for agents working in this codebase, not for
the GUI app's AI feature. See [editor skills](ai/editor-skills.md).

## Build & install

### Which compiler do I need?

A C++23 compiler — GCC 14+ or Clang 18+.

### meson picked Qt 5 instead of Qt 6!

Point `PATH` at your Qt 6 bin directory before configuring, e.g.
`PATH=/usr/lib/qt6/bin:$PATH meson setup build --reconfigure`.

### The Rust build fails with "cargo not found"

Install the Rust stable toolchain and ensure it is on `PATH`. The build
invokes `cargo build --workspace` automatically.

### Can I run it without installing?

Yes — the binary runs directly from the build directory. See
[build from source](getting-started/build-from-source.md).

## Troubleshooting

### The status list does not refresh

The auto-refresh watches `.git/index` and `.git/HEAD` with a 2-second
debounce. If it is stale, trigger a manual action (e.g. switch branches) or
reopen the repository.

### A remote push asks for credentials every time

Configure a credential helper (`git config --global credential.helper store`
or your OS keyring), or use an SSH remote.

### Where do I report bugs?

Open an issue on the GitHub repository. Include the LazyDesktop version, your
OS/Qt versions, and steps to reproduce.
