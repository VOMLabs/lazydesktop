# AI Editor Skills

The repository ships skills that teach AI coding tools (OpenCode) the
project's **commit** and **branch** conventions, so any agent working in the
repo produces consistent output.

The two skills live in `.opencode/skills/`:

| Tool | Location | Skills |
|------|----------|--------|
| OpenCode | `.opencode/skills/` | `commit`, `create-branch` |

## `commit`

The commit skill:

1. Detects whether the repository uses **Git** or **Jujutsu** (prefers `.jj`
   in a colocated repository).
2. Gathers the diff and recent history.
3. Produces a **Conventional Commits** message: `type(scope): subject`
   (e.g. `feat(vcs): add jujutsu branch creation`).
4. Commits **only after explicit approval**.

Supported types: `feat`, `fix`, `refactor`, `chore`, `docs`, `test`, `perf`,
`build`, `ci`, `style`, `revert`.

## `create-branch`

The branch-creation skill names branches using the
`type/scope?/short-description` convention, for both Git and Jujutsu:

```
feat/vcs/jj-support
fix/ai-core/link-ffi
refactor/settings/prompts
```

## Why it matters

- **Consistency** — every AI-generated commit message and branch follows the
  same conventions as the project's own history.
- **Reviewable** — agents present a Conventional Commit message before
  committing, so you can approve exactly what gets created.
- **Tool-agnostic** — the same behavior available in OpenCode, so the whole
  team produces matching output regardless of editor.

## Related

- [Roadmap](../../ROADMAP.md) — where skills fit in the project history
- [Implementation details](../../IMPLEMENTATION.md) — technical notes on the
  skill implementations
