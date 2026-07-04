# LazyAddons Plugin System

LazyAddons is a first-class plugin system for LazyDesktop that allows extending
the application with Lua or Python scripts. Plugins can register commands,
respond to application events, access the filesystem, make network requests,
and more.

## Quick Links

- [Getting Started](getting-started.md) — create your first plugin
- [Manifest Format](manifest.md) — how to write manifest.yml
- [Lua API Reference](lua-api.md) — Lua plugin development
- [Python API Reference](python-api.md) — Python plugin development
- [Permissions](permissions.md) — security model reference
- [Tutorials](tutorials/first-plugin.md) — step-by-step guides

## Supported Languages

| Language | Runtime | Recommended For |
|----------|---------|-----------------|
| **Lua**  | Embedded (sol2) | Lightweight extensions, quick scripts |
| **Python** | Process-based (uv) | Complex logic, rich dependencies |

Only one runtime is allowed per plugin.

## Package Format

Plugins are distributed as `.zip` or `.lza` archives:

```
plugin.lza
├── manifest.yml           # required: plugin metadata
├── conf.yml               # optional: default configuration
├── assets/
│   └── icon.png           # optional: bundled resources
├── src/
│   ├── main.lua           # Lua entry point
│   └── main.py            # Python entry point
└── pyproject.toml          # required for Python plugins only
```

## Key Concepts

- **Manifest** — YAML/JSON metadata describing the plugin
- **Runtime** — the execution environment (Lua or Python)
- **Lifecycle** — plugins receive load/enable/disable/unload events
- **Permissions** — security model controlling resource access
- **Configuration** — isolated per-plugin settings storage
