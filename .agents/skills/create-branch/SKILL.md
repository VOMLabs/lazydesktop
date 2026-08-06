---
name: create-branch
description: Creates a new branch (git and jujutsu) using the Conventional Branch naming convention type/scope?/short-description. Trigger when the user asks to create a new branch, start a branch, or set up a branch for a feature or task.
---

# Conventional Branch Skill

Creates a new branch following the Conventional Branch naming convention
`type/scope?/short-description`.

## When to use

Use this skill when the user asks to create a new branch, "start a branch", or
set up a branch for a feature or task. Skip when the user asks to switch to an
existing branch, or when there is no VCS repository.

## Detect the VCS

1. Check for a `.jj` directory, then a `.git` directory. Prefer Jujutsu when
   both exist (colocated repository).
2. If neither is found in the current directory, walk up parent directories to
   the repository root. If there is no repository, stop and tell the user.

## Choose the name

Format: `<type>/<scope?>/<short-description>`

- Type: `feat`, `fix`, `refactor`, `chore`, `docs`, `test`, `perf`, `build`,
  `ci`, `style`, `revert`.
- Scope (optional): a short module name, for example `vcs`, `ai-core`,
  `settings`. Omit when it adds no value.
- Description: lowercase, kebab-case, concise (aim for 30 characters or less),
  no trailing slash.

Valid examples: `feat/vcs/jj-support`, `fix/ai-core/ffi-linking`,
`docs/readme`, `feat/theme-toggle`.

## Check before creating

- Confirm the name does not already exist:
  - Git: `git branch --list <name>`
  - Jujutsu: `jj branch list <name>`
  Pick another name if a match is found.
- If the working tree has uncommitted changes, ask the user whether to carry
  them onto the new branch or branch from a clean state.

## Create the branch (only after approval)

Show the branch name and ask for confirmation before running any command.

Git:
- `git checkout -b <name>` or `git switch -c <name>` (branch from current commit)
- Optional publish: `git push -u origin <name>`

Jujutsu:
- `jj branch create <name>` creates a branch pointing at the working copy
  commit.
- Optional publish: `jj git push --branch <name>`

Verify the created branch name matches the convention before finishing.

## Examples

- `feat/vcs/jj-support` — feature adding Jujutsu support to the VCS module
- `fix/ai-core/ffi-linking` — bugfix for FFI linking in the AI core crate
- `docs/readme` — documentation update, no scope
