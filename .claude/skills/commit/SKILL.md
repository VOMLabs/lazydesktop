---
name: commit
description: Writes Conventional Commits messages for the current changes (git and jujutsu) and commits them after user approval. Trigger when the user asks to write a commit message, generate a commit message, make a commit, or commit the current changes.
---

# Conventional Commit Skill

Writes a Conventional Commits message for the current changes and commits them
only after explicit user approval.

## When to use

Use this skill when the user asks to write a commit message, generate a commit
message, "make a commit", or commit the current changes. Skip when there is no
VCS repository or the user only wants a diff review.

## Detect the VCS

1. Check for a `.jj` directory, then a `.git` directory. Prefer Jujutsu when
   both exist (colocated repository).
2. If neither is found in the current directory, walk up parent directories to
   the repository root. If there is no repository, stop and tell the user.

## Gather context

Jujutsu (jj):
- `jj status` — working copy changes and parent
- `jj diff` — working copy diff
- `jj --config ui.pagination=never log --no-graph --limit 10` — recent history

Git:
- `git status --short` — changed files
- `git diff --cached` — staged diff (prefer this when the user commits staged changes)
- `git diff HEAD` — all working tree changes
- `git log --oneline -10` — recent history
- `git branch --show-current` — current branch

For Git, base the message on the staged diff when the user is committing
staged changes, otherwise on the working tree diff. For Jujutsu, the working
copy commit is the change being described.

## Write the message

Subject: `type(scope): imperative subject`
- Type: `feat`, `fix`, `refactor`, `chore`, `docs`, `test`, `perf`, `build`,
  `ci`, `style`, `revert`.
- Scope: the module or component (for example `vcs`, `ai-core`, `settings`).
  Omit when it adds no value.
- Imperative present tense: "add", "fix", "remove" — never "added", "fixes".
- Lowercase after the colon, no trailing period, subject under 72 characters.

Body (only when it adds value): blank line, then a short explanation of what
changed and why, wrapped at 72 characters.

Footer (only for breaking changes): `BREAKING CHANGE:` followed by a short
description.

## Validate before committing

- Subject matches `^(feat|fix|refactor|chore|docs|test|perf|build|ci|style|revert)(\([a-z0-9-]+\))?: [a-z]`
- Subject is imperative, at most 72 characters, no trailing period.
- No filler text, no markdown code fences around the message.

## Apply the commit (only after approval)

Show the message and ask for confirmation before running any command.

Jujutsu:
- `jj describe -m <subject> -m <body>` sets the working copy change message.
- `jj commit` starts a new change on top when the user wants to continue.

Git:
- `git commit` commits the staged changes; use `-m` flags for the subject and
  body, or create the message through the editor. Never use `-a` without
  confirming the user wants unstaged changes included.

Never commit without explicit approval.

## Examples

- `feat(vcs): add jujutsu branch creation`
- `fix(ai-core): link commit-message ffi correctly`
- `refactor(settings): move prompts into ai_core`
- Breaking change: subject `feat(api): drop v2 endpoints` with footer
  `BREAKING CHANGE: the /v2 endpoints were removed; migrate to /v3.`
