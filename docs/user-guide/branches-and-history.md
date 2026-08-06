# Branches & History

## Branch management

The branch dropdown sits in the toolbar next to the project button.

- **Switch** — pick a branch from the dropdown. If you have uncommitted
  changes, LazyDesktop warns you first.
- **Create** — choose "New branch…" (or type a name) to run
  `git checkout -b <name>`.
- **Delete** — select a branch and click the delete button (uses
  `git branch -D`). You cannot delete the branch you are on.

## Commit history

The **History** tab lists commits in reverse chronological order. Each entry
shows, from top to bottom:

1. The commit hash (monospace, gray)
2. The subject line (bold)
3. Author and date (smaller, gray)

### Drilling into a commit

1. Click a commit in the list — the panel below fills with the files that
   commit changed (`git diff-tree`).
2. Click a file — its content at that commit is shown in the diff viewer
   (`git show`).

This makes it easy to review what a commit actually did, file by file.

## Viewing unpushed work

The status query also computes files that differ from your upstream branch
(`git diff --name-only @{u}..HEAD`), so you can see at a glance what is
committed locally but not yet pushed.

## Related

- [Git workflow](git-workflow.md) — status, staging, and committing
- [Projects](projects.md) — switching between repositories
