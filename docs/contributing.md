# Contributing

Thanks for your interest in LazyDesktop! This project is small and friendly —
here is how to get involved.

## Getting started

1. **Fork** the repository and clone your fork.
2. Set up the toolchain — see [build from source](getting-started/build-from-source.md)
   (`mise install` and `just setup` are your friends).
3. Build and run: `just run`.
4. Find something to work on in the [roadmap](../ROADMAP.md) or an open
   issue.

## Commit conventions

This repository follows **Conventional Commits**. The AI coding skills
shipped in `.opencode/skills/` can generate conforming messages for you.

```
type(scope): subject
```

- **Type**: `feat`, `fix`, `refactor`, `chore`, `docs`, `test`, `perf`,
  `build`, `ci`, `style`, `revert`
- **Scope**: the module or component (e.g. `settings`, `ai-core`, `vcs`)
- **Subject**: imperative present tense, lowercase, ≤ 72 characters

Examples:

```
feat(settings): add theme picker
fix(ai-core): link commit-message ffi correctly
docs: document the packaging workflow
```

## Branch naming

Branches follow the `type/scope?/short-description` convention:

```
feat/settings/theme-picker
fix/ai-core/link-ffi
docs/packaging
```

## Code style

- Rust is formatted with **rustfmt** (`cargo fmt --all`): `just format`.
- Lints with **clippy**: `just clippy`.
- Run all pre-commit hooks with `just lint` (see
  [build & test](development/build-and-test.md)).

## Testing

- The workspace has a unit test suite: `just test`
  (`cargo test --workspace`).
- Add regression tests where practical and describe manual verification in
  the PR.

## Pull request workflow

1. Create a branch off `main` (`type/scope/short-description`).
2. Make your change; keep it focused.
3. Run `just format` and `just lint`.
4. Push and open a pull request against `main`.
5. CI (`.github/workflows/ci.yml`) builds on Ubuntu, Windows, and macOS —
   make sure it is green.

## Documentation

User-facing changes should update the relevant page in [`docs/`](README.md)
if it exists. New features should be documented there too.
