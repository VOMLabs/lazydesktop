# Themes

LazyDesktop uses your system palette by default and lets you switch to a dark
theme or fully custom themes.

## Built-in themes

| Theme | Description |
|-------|-------------|
| **System Default** | No stylesheet — the app uses your desktop palette |
| **Dark** | Hardcoded VS Code-style dark colors applied as a Qt stylesheet |

Switch themes in **Settings → Appearance**.

## Custom YAML themes

Drop a `.theme.yaml` file into `~/.config/lazydesktop/themes/` to add a new
theme option. The theme appears in Settings → Appearance after you restart
the app or reopen the settings dialog.

> **Note:** the bundled Rust `config` crate (used by the in-development GPUI
> frontend) reads themes from `*.theme.lua` files instead. The Qt UI still
> uses YAML.

### Format

```yaml
name: "Ocean Night"
colors:
  background: "#0d1117"
  foreground: "#c9d1d9"
  widget_background: "#161b22"
  input_background: "#21262d"
  input_foreground: "#c9d1d9"
  button_background: "#1f6feb"
  button_foreground: "#ffffff"
  tooltip_background: "#21262d"
  tooltip_foreground: "#c9d1d9"
  selection: "#1f6feb"
```

### Color reference

| Key | Used for |
|-----|----------|
| `background` | Main window / application background |
| `foreground` | Default text color |
| `widget_background` | Widget (panel, list) backgrounds |
| `input_background` | Text input fields |
| `input_foreground` | Text inside input fields |
| `button_background` | Push buttons |
| `button_foreground` | Text on push buttons |
| `tooltip_background` | Tooltip background |
| `tooltip_foreground` | Tooltip text |
| `selection` | Selected items / highlights |

All colors are hex values (`#rrggbb`). The `name` must be unique and is the
label shown in the settings dialog.

### How it works

At startup (and when the theme picker changes), `generateStylesheet()`
converts the YAML color map into a Qt stylesheet string applied to the
`QApplication`. System Default applies no stylesheet, leaving the palette to
your desktop theme.

## Example: a GitHub-dark theme

```yaml
name: "GitHub Dark"
colors:
  background: "#0d1117"
  foreground: "#e6edf3"
  widget_background: "#161b22"
  input_background: "#21262d"
  input_foreground: "#e6edf3"
  button_background: "#238636"
  button_foreground: "#ffffff"
  tooltip_background: "#1c2128"
  tooltip_foreground: "#e6edf3"
  selection: "#1f6feb"
```
