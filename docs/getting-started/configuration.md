# Configuration

All persistent data lives under `~/.config/lazydesktop/`.

## Data paths

| Path | Format | Purpose |
|------|--------|---------|
| `~/.config/lazydesktop/lazydesktop.conf` | INI | Application settings |
| `~/.config/lazydesktop/projects.lua` | Lua | Recent project paths |
| `~/.config/lazydesktop/themes/*.theme.lua` | Lua | Custom theme definitions |
| `~/.config/lazydesktop/models/` | GGUF | Downloaded local AI models |

The data paths themselves can be changed in **Settings → General**.

## Settings reference

Settings are stored in `lazydesktop.conf` as key/value pairs grouped by
category (`[category] key = value`).

### Appearance

| Key | Default | Purpose |
|-----|---------|---------|
| `appearance/theme` | `"system"` | Active theme: `system` (follow the OS appearance), `Dark`, `Light`, or a custom theme name |

### Git

| Key | Default | Purpose |
|-----|---------|---------|
| `git/user.name` | — | Global Git user name (read/written via `git config --global`) |
| `git/user.email` | — | Global Git user email |

### AI

| Key | Default | Purpose |
|-----|---------|---------|
| `ai/enabled` | `false` | Master toggle for AI features |
| `ai/provider` | `"OpenRouter"` | Cloud provider (`OpenRouter`, `OpenAI`, `Anthropic`, `Google AI Studio`) |
| `ai/api_key` | — | API key for cloud providers |
| `ai/model` | `"gpt-4o-mini"` | Model name for the selected provider |
| `ai/system_prompt` | built-in | Prompt template for commit message summaries; may contain `<diff>` |
| `ai/description_system_prompt` | built-in | Prompt template for commit descriptions; may contain `<diff>` |
| `ai/local_model_path` | — | Path to the active GGUF model |
| `ai/gpu_acceleration` | `true` | Offload layers to the GPU (`n_gpu_layers = 99`) |
| `ai/local_models` | `[]` | JSON array of downloaded model metadata |

### Paths

| Key | Default | Purpose |
|-----|---------|---------|
| `paths/projects` | `~/.config/lazydesktop/projects.lua` | Project list path |
| `paths/settings` | `~/.config/lazydesktop/lazydesktop.conf` | Settings path |
| `paths/themes` | `~/.config/lazydesktop/themes` | Themes directory |

> **Security note:** the API key is stored in plain text in `lazydesktop.conf`.
> Protect this file the same way you protect other local credentials.

## Custom Lua themes

Drop a `.theme.lua` file into `~/.config/lazydesktop/themes/` and it appears
as a theme option in **Settings → Appearance** after a restart (or after
reopening the settings dialog). See [themes](../user-guide/themes.md) for the
full guide, the file format, and the color reference.