---
name: workflow-checks
description: Automates pre-commit hooks, linting, code quality, and CI verification for LazyDesktop. Trigger when the user asks to run linters, fix formatting, check code quality, or verify pre-commit compliance.
---

# Workflow Checks Skill

## Pre-commit Hooks

Configured via `.pre-commit-config.yaml`:

| Hook | Tool | Scope |
|------|------|-------|
| trailing-whitespace | pre-commit-hooks | All files |
| end-of-file-fixer | pre-commit-hooks | All files |
| check-yaml | pre-commit-hooks | YAML files |
| check-added-large-files | pre-commit-hooks | Git index |
| check-merge-conflict | pre-commit-hooks | All files |
| mixed-line-ending | pre-commit-hooks | All files (fix=LF) |

## Running Checks

```bash
# Run all pre-commit hooks on all files
pre-commit run --all-files

# Via just
just lint          # pre-commit hooks
just format        # cargo fmt --all
just clippy        # cargo clippy --workspace --all-targets -- -D warnings
just audit         # cargo audit
```

## Code Style

- Rust formatting via rustfmt (`cargo fmt --all`)
- Rust lints via clippy (`cargo clippy --workspace --all-targets -- -D warnings`)
- LF line endings
- No trailing whitespace
- YAML files must be valid

### Rust Naming Conventions (enforced by rustfmt/clippy)

| Category | Convention | Example |
|----------|-----------|---------|
| Types/Enums | `PascalCase` | `LazyDesktopApp` |
| Functions/Methods | `snake_case` | `refresh_all` |
| Constants | `SCREAMING_SNAKE_CASE` | `AI_PROVIDER_OPENROUTER` |
| Variables | `snake_case` | `app_state` |
| Modules | `snake_case` | `git_service` |

## CI Requirements (pre-merge)

1. All pre-commit hooks pass
2. `cargo fmt --all -- --check` produces no diffs
3. `cargo clippy --workspace --all-targets -- -D warnings` passes
4. Project builds cleanly (`cargo build --workspace`)
5. `cargo test --workspace` passes (all suites green)