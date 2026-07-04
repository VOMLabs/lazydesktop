# Lua Plugin API

Lua plugins have access to a global `addon` table that provides the following
functions.

## Logging

```lua
addon.log(message, level)
```

Log a message. `level` can be `"info"`, `"warn"`, `"error"`, or `"debug"`
(defaults to `"info"`).

```lua
addon.log("Hello world!")
addon.log("Something went wrong", "error")
```

## Configuration

```lua
addon.readConfig(key, defaultValue)
```

Read a configuration value. Returns `defaultValue` if the key is not set.

```lua
local name = addon.readConfig("username", "default")
```

```lua
addon.writeConfig(key, value)
```

Write a configuration value. Persisted to `settings.json` in the plugin's
config directory.

```lua
addon.writeConfig("username", "jane")
```

## Assets

```lua
addon.readAsset(relativePath)
```

Read a bundled asset file as a string. Path is relative to the `assets/`
directory of the plugin.

```lua
local icon = addon.readAsset("icon.png")
```

```lua
addon.assetPath(relativePath)
```

Get the absolute filesystem path to a bundled asset.

```lua
local path = addon.assetPath("templates/header.html")
```

## Data Storage

```lua
addon.dataPath(subpath)
```

Get an absolute path for storing plugin data. Subdirectory is created
automatically.

```lua
local path = addon.dataPath("cache")
```

## Notifications

```lua
addon.notify(title, message)
```

Display a notification to the user.

```lua
addon.notify("Plugin Loaded", "My plugin is now active")
```

## Network

```lua
addon.httpGet(url)
```

Make an HTTP GET request. Returns the response body as a string.
Requires the `network` permission.

```lua
local response = addon.httpGet("https://api.example.com/data")
```

## Lifecycle Hooks

Define any of these functions in your main script to respond to lifecycle
events:

```lua
function onLoad()
    -- Called when the plugin script is first loaded
end

function onEnable()
    -- Called when the plugin is enabled
end

function onDisable()
    -- Called when the plugin is disabled
end

function onUnload()
    -- Called when the plugin is about to be unloaded
end

function onSettingsChanged()
    -- Called when plugin settings change
end
```

## Application Events

```lua
function onAppStarted()
    -- Application has finished starting
end

function onAppShuttingDown()
    -- Application is shutting down
end

function onProjectOpened(data)
    -- A project/repository was opened. data.project contains the path
end

function onProjectClosed()
    -- The current project was closed
end

function onThemeChanged(data)
    -- The application theme changed. data.theme contains the theme name
end

function onFileOpened(data)
    -- A file was opened. data.file contains the path
end

function onFileSaved(data)
    -- A file was saved. data.file contains the path
end
```

## Example

```lua
function onEnable()
    addon.log("Plugin enabled!", "info")
    local data = addon.httpGet("https://api.github.com/zen")
    if data then
        addon.log("Zen: " .. data)
    end
    addon.notify("Active", "My plugin is running")
end

function onDisable()
    addon.writeConfig("last_status", "disabled")
end
```
