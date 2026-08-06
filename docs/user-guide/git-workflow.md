# Git Workflow

This guide covers the everyday Git operations in LazyDesktop: checking
status, staging files, committing, and pushing.

## Opening a repository

Click the **Open Folder** button (top-left) and pick a directory that is a
Git repository, or use the **Add project** dropdown to clone or create one —
see [projects](projects.md).

## Status list

The **Changes** tab shows the working tree as a flat file list. Each file has
a colored status indicator:

| Color | Meaning |
|-------|---------|
| Yellow | Modified |
| Green | Added |
| Red | Deleted |
| Purple | Renamed |
| Gray | Untracked |

The list updates automatically: a file-system watcher on `.git/index` and
`.git/HEAD` triggers a 2-second debounced refresh whenever something changes.

## Staging files

- Check the box next to a file to stage it.
- Use the master **Select All** checkbox in the header bar to stage or unstage
  everything.
- Checked files are the ones that will be committed (and the ones whose diffs
  feed the AI message generator).

## Viewing diffs

Click any file to show its diff in the right-hand viewer:

- Text files render in a syntax-highlighted diff view with line numbers.
- Added lines are green, deleted lines are red, hunk headers are blue.
- Image files (png, jpg, webp, gif, …) render inline.
- Video files show a placeholder.

## Committing

1. Write a **summary** (required) and an optional **description**.
2. (Optional) Toggle the lightning-bolt button to skip pre-commit hooks —
   this adds `--no-verify`.
3. (Optional) Click the people icon to add **co-authors** — LazyDesktop scans
   the checked files' Git history for authors and appends
   `Co-authored-by:` trailers to the description.
4. Click **Commit**.

Internally this stages the checked files with `git add` and runs
`git commit -m <summary> [-m <description>] [--no-verify]`.

### AI-assisted commits

The AI button (lightning/robot icon) generates a summary and description from
the diffs of your checked files. Right-click it to switch providers. See the
[AI guides](../ai/overview.md) for setup.

## Push / Fetch / Pull

The button in the toolbar cycles through states automatically:

1. **Push** — sends your local commits to the remote.
2. **Fetch** — after a successful push with nothing to upload, or when the
   button cycles; fetches remote refs.
3. **Pull** — if you are behind the remote (or a push is rejected as
   non-fast-forward), the button becomes **Pull** to merge remote changes.

In short: keep clicking the same button and LazyDesktop does the right thing.

## Discarding changes

Right-click a modified file in the status list and choose **Discard Changes**
(`git restore`). Right-click an untracked file to **delete** it instead.

## Credentials

If a remote requires authentication and no credentials are cached, LazyDesktop
shows a credential dialog (host, username, token/password) and wires it into
Git through `GIT_ASKPASS` for that operation.
