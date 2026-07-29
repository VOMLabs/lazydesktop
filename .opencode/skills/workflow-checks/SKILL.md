---
name: workflow-checks
description: Automates pre-commit hooks, linting, code quality, and CI verification for LazyDesktop. Trigger when the user asks to run linters, fix formatting, check code quality, or verify pre-commit compliance.
---

# Workflow Checks Skill

## Pre-commit Hooks

Configured via `.pre-commit-config.yaml`:

| Hook | Tool | Scope |
|------|------|-------|
| clang-format | pre-commit-mirrors (v19.1.7) | C++, C, CUDA |
| cmake-format | cmake-format-precommit (v0.6.13) | CMake/Meson files |
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

# Run specific hook
pre-commit run clang-format --all-files

# Via just
just lint    # pre-commit hooks
just format  # clang-format all source files
just tidy    # clang-tidy static analysis
```

## Code Style

- C++23 standard
- `.clang-format` for formatting (LLVM-based, 4-space indent, Allman braces, 120 column limit)
- `.clang-tidy` for static analysis (bugprone, clang-analyzer, concurrency, modernize, performance, readability)
- LF line endings
- No trailing whitespace
- YAML files must be valid

### Naming Conventions (enforced by clang-tidy)

| Category | Convention | Example |
|----------|-----------|---------|
| Classes/Structs | `CamelCase` | `MainWindow` |
| Enums | `CamelCase` | `AiRequestKind` |
| Functions/Methods | `camelBack` | `startGitStatusQuery` |
| Member variables | `m_` prefix + `camelBack` | `m_gitProcess` |
| Constants | `k` prefix + `UPPER_CASE` | `kDefaultTimeout` |
| Parameters | `camelBack` | `const QString &path` |

## CI Requirements (pre-merge)

1. All pre-commit hooks pass
2. `just format` produces no diffs (code is clang-formatted)
3. `just tidy` passes (no clang-tidy warnings treated as errors)
4. Project builds cleanly (`meson setup build && ninja -C build`)
5. No new compiler warnings (warning_level=3)
