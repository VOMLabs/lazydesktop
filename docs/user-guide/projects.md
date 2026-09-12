# Projects

LazyDesktop tracks the repositories you work on in a recent-projects list,
stored in `~/.config/lazydesktop/projects.yaml`.

> **Note:** the bundled Rust `config` crate (used by the in-development GPUI
> frontend) persists the list as `projects.lua` instead. The Qt UI still uses
> YAML.

## The Add Project dropdown

The drawer button in the top-left opens the recent-projects drawer with an
**+ Add** dropdown offering three options:

| Option | What it does |
|--------|--------------|
| **Clone Repository** | Prompts for a URL and destination, then runs `git clone` |
| **Create Repository** | Picks a directory, then runs `git init` |
| **Load Existing** | Opens a folder picker to open an existing repository |

## Recent projects drawer

The drawer (hidden by default) lists your recent repositories:

- **Grouped by remote owner** — projects are grouped under headers derived
  from the GitHub owner/org in the remote `origin` URL (SSH URLs are
  normalized to HTTPS for owner extraction).
- **Dirty indicator** — repositories with uncommitted changes show a yellow
  dot.
- Click an entry to open that repository.

### Managing the list

- **Scan Folder for Projects** — bulk-import every Git repository found in a
  folder's subdirectories.
- **Remove** (right-click / context action) — remove one project; you may
  optionally delete the directory on disk.
- **Remove All** — clear the entire list after a confirmation dialog.

## Closing a repository

Use **Close Repository** from the menu to reset all UI state (status list,
history, diff viewer, branch selection) without leaving the app.

## Related

- [Git workflow](git-workflow.md) — working inside an opened repository
- [Configuration](../getting-started/configuration.md) — where `projects.yaml`
  lives and how to change the data path
