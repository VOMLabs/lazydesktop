# Manifest Format

Every LazyAddons plugin must include a `manifest.yml` (or `manifest.json`)
at the root of the package.

## Example

```yaml
id: com.example.myplugin
name: My Plugin
version: 1.0.0
runtime: lua
entry: src/main.lua
api: 1
author: Jane Doe
description: A useful plugin for LazyDesktop
license: MIT
website: https://example.com
icon: assets/icon.png
min_version: 0.1.0
max_version: 1.0.0
categories:
  - productivity
tags:
  - git
  - automation
dependencies:
  - other.plugin.id
permissions:
  - network
  - notifications
```

## Fields

| Field | Required | Type | Description |
|-------|----------|------|-------------|
| `id` | **Yes** | string | Unique plugin identifier (reverse domain notation recommended) |
| `name` | **Yes** | string | Human-readable plugin name |
| `version` | **Yes** | string | Semantic version (e.g. `1.0.0`, `2.3.1`) |
| `runtime` | **Yes** | string | Either `lua` or `python` |
| `entry` | **Yes** | string | Path to the entry point relative to plugin root (e.g. `src/main.lua`) |
| `api` | **Yes** | integer | LazyAddons API version (currently `1`) |
| `author` | No | string | Plugin author name |
| `description` | No | string | Short description of the plugin |
| `license` | No | string | SPDX license identifier (e.g. `MIT`, `GPL-3.0`) |
| `website` | No | string | Project website or repository URL |
| `icon` | No | string | Path to plugin icon relative to plugin root |
| `min_version` | No | string | Minimum supported LazyDesktop version |
| `max_version` | No | string | Maximum supported LazyDesktop version |
| `categories` | No | list | Category tags for organization |
| `tags` | No | list | Arbitrary search tags |
| `dependencies` | No | list | Plugin IDs that must be installed first |
| `permissions` | No | list | Requested [permissions](permissions.md) |

## JSON Format

The same metadata can be expressed as JSON:

```json
{
  "id": "com.example.myplugin",
  "name": "My Plugin",
  "version": "1.0.0",
  "runtime": "lua",
  "entry": "src/main.lua",
  "api": 1,
  "author": "Jane Doe",
  "description": "A useful plugin for LazyDesktop",
  "license": "MIT"
}
```

## Validation Rules

- `id` must be non-empty and unique among installed plugins
- `runtime` must be `lua` or `python`
- `entry` must point to an existing file within the plugin
- `api` must be >= 1
- `version` should follow [Semantic Versioning](https://semver.org/)
- Python plugins must include a `pyproject.toml`
