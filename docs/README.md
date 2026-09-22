# LazyDesktop Documentation

Welcome to the LazyDesktop documentation. LazyDesktop is a fast, native Git
GUI for the KDE Plasma desktop. The UI is built with **Rust and GPUI** in
`crates/app` and calls the backend Rust crates directly.

## Getting started

New here? Start with the guides below.

- [Installation](getting-started/installation.md) — install from packages,
  AppImage, or source
- [Build from source](getting-started/build-from-source.md) — requirements,
  cargo + moon, and the `justfile` recipes
- [Docker](getting-started/docker.md) — run LazyDesktop in a container
  (X11 or VNC/noVNC)
- [Configuration](getting-started/configuration.md) — where data lives and
  what every setting does

## User guide

Day-to-day usage of the application.

- [Git workflow](user-guide/git-workflow.md) — status, staging, commit,
  push/fetch/pull
- [Branches & history](user-guide/branches-and-history.md) — branch
  management and commit history drill-down
- [Projects](user-guide/projects.md) — recent projects, clone/create/load,
  scanning folders
- [Themes](user-guide/themes.md) — system, dark, and custom YAML themes

## AI

LazyDesktop can write commit messages for you — either in the cloud or fully
local.

- [AI overview](ai/overview.md) — how AI commit messages work
- [Cloud providers](ai/cloud-providers.md) — OpenRouter, OpenAI, Anthropic,
  Google AI Studio
- [Local models](ai/local-models.md) — GGUF models, downloads, GPU
  acceleration
- [Editor skills](ai/editor-skills.md) — `commit` and `create-branch` skills
  for AI coding tools

## Development

For contributors and people who want to understand the internals.

- [Architecture](development/architecture.md) — layers, data flow, key
  decisions
- [Source layout](development/source-layout.md) — where everything lives
- [`ai_core` crate](development/ai-core.md) — the Rust inference engine and
  its C FFI
- [Build & test](development/build-and-test.md) — cargo + moon, cargo
  test, pre-commit, CI
- [Packaging](development/packaging.md) — .deb, .AppImage, .pkg.tar.zst,
  .msi, Docker

## Project meta

- [Contributing](contributing.md)
- [FAQ](faq.md)
- [Release process](release-process.md)
- [Roadmap](../ROADMAP.md)
- [Implementation details](../IMPLEMENTATION.md)
