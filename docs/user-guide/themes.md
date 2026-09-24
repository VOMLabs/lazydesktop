# Themes

LazyDesktop uses a shared token-based palette. The app follows your system
appearance by default and lets you switch to built-in **Dark** or **Light**
palettes, or fully custom themes stored as Lua files.

## Built-in themes

| Theme | Description |
|-------|-------------|
| **System Default** | Follows the OS appearance (`window_appearance()`); used when no palette is persisted |
| **Dark** | The tuned dark token set |
| **Light** | The tuned light token set |

Switch themes in **Settings → Appearance**. Your choice is persisted under
`appearance/theme` in `lazydesktop.conf` and applied at startup, before the
first frame renders.

## How the palette works

All colors come from a single shared token set — `Palette` in
`crates/app/src/theme.rs` (37 tokens for surfaces, interactive states, text,
status colors, and diff colors). Two tuned variants exist: `Palette::dark()`
and `Palette::light()`.

When a palette is applied, it is also projected into the `gpui-component`
theme (`Theme::apply_config` + `Theme::change`), so built-in components —
buttons, inputs, checkboxes, list rows, popovers — pick up the same shades
automatically. The app surfaces (sidebar, file tree, diff view, commit
panel, settings) read tokens directly from `Palette::current(cx)`.

## Custom Lua themes

Drop a `*.theme.lua` file into `~/.config/lazydesktop/themes/` to add a new
theme option. The file name must end in `.theme.lua` (for example
`ocean-night.theme.lua`). The theme appears in **Settings → Appearance**
after you restart the app or reopen the settings dialog.

### Format

```lua
return {
  name = "Ocean Night",
  colors = {
    background = "#0d1117",
    foreground = "#c9d1d9",
  }
}
```

### Color reference

| Key | Used for |
|-----|----------|
| `background` | Determines the mode: perceived luminance picks the **Dark** or **Light** token set |
| `foreground` | Optional; informational (the tuned token set supplies text colors) |

Only `background` is required. A dark background (relative luminance < 0.5)
resolves to the tuned dark token set; a light background resolves to the
light set. All colors are hex strings (`#rrggbb`). The `name` must be unique
and is the label shown in the settings dialog.

> Custom themes tune the *mode*, not every pixel: every shade (surfaces,
> hover/selected states, text tiers, status colors) comes from the matching
> token set, so contrast and interactive states stay consistent.

## Switching/resolving behavior

- **Settings → Appearance** options: System Default, Dark, Light, plus any
  custom themes on disk.
- Selecting an option persists `appearance/theme` and re-installs the palette
  (and the component theme) immediately.
- Custom themes are resolved by `background` luminance at selection time and
  again at startup.

## Related

- [Configuration](../getting-started/configuration.md) — where themes live and
  how settings are stored
- [Architecture](../development/architecture.md) — the `Palette` token system