# Permissions

LazyAddons uses a permission-based security model to control what plugins
can do. Permissions are declared in the plugin manifest and displayed to
the user before installation.

## Permission List

| Permission | Flag | Description |
|-----------|------|-------------|
| `filesystem` | 1 | Read and write files on your system |
| `network` | 2 | Make network requests to remote servers |
| `execute` | 4 | Execute external programs on your system |
| `clipboard` | 8 | Read and write to the system clipboard |
| `notifications` | 16 | Display system notifications |
| `project_access` | 32 | Read project files and repository data |
| `ui_access` | 64 | Modify the application user interface |

## Dangerous Permissions

The following permissions are considered **dangerous** and trigger a warning
dialog during installation:

- **`execute`** — can run arbitrary programs on your system
- **`network`** — can send/receive data from remote servers
- **`clipboard`** — can read sensitive data from the clipboard

## Requesting Permissions

In your `manifest.yml`:

```yaml
permissions:
  - network
  - notifications
```

Or in JSON:

```json
{
  "permissions": ["network", "notifications"]
}
```

## Security Best Practices

### For Plugin Developers

- Request only the permissions your plugin actually needs
- Never request `execute` unless absolutely necessary
- Treat API keys and user data with care
- Use `network` permission responsibly; respect rate limits

### For Users

- Review permissions before installing a plugin
- Be cautious with plugins requesting dangerous permissions
- Disable plugins you no longer use
- Uninstall plugins from sources you don't trust

## Permission Checking

The permission system is currently **declarative** — permissions are displayed
but not enforced at runtime. Future versions will implement mandatory access
control based on declared permissions.
