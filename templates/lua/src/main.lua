--[[
  My Lua Plugin
  A LazyAddons plugin template for Lua

  Demonstrates the full plugin API:
    - Lifecycle hooks
    - Config, assets, data storage
    - Commands & menu contributions
    - File CRUD within sandboxed data dir
    - HTTP requests & notifications
]]

function onLoad()
    addon.log("Plugin loading...", "debug")
end

function onEnable()
    local greeting = addon.readConfig("greeting", "Hello from Lua!")
    addon.log(greeting, "info")
    addon.notify("Plugin Enabled", "My Lua Plugin is now active")

    -- Register commands callable from the Command Palette (Ctrl+P)
    addon.registerCommand("lua:greet", function(args)
        local who = args and args[1] or "world"
        addon.log("Hello, " .. who .. "!", "info")
        return "Greeted " .. who
    end)

    addon.registerCommand("lua:write-note", function(args)
        local text = args and args[1] or "default note"
        addon.writeFile("notes.txt", text)
        addon.log("Note saved: " .. text, "info")
        return "Saved"
    end)

    addon.registerCommand("lua:list-files", function()
        local files = addon.listFiles()
        local msg = "Files: " .. table.concat(files, ", ")
        addon.log(msg, "info")
        return msg
    end)

    addon.registerCommand("lua:read-note", function()
        local content = addon.readFile("notes.txt")
        if content and #content > 0 then
            addon.notify("Last Note", content)
        else
            addon.log("No notes found", "warn")
        end
        return content or ""
    end)

    -- Contribute a menu item to the Addons menu
    addon.createMenu("Addons", "Greet from Lua", "lua:greet")
    addon.createMenu("Addons", "Read Note", "lua:read-note")
end

function onDisable()
    addon.log("Plugin disabled", "info")
end

function onUnload()
    addon.log("Plugin unloading", "debug")
end

function onAppStarted()
    addon.log("Application started", "info")
end

function onAppShuttingDown()
    addon.log("Application shutting down", "info")
end

function onProjectOpened(data)
    if data and data.project then
        addon.log("Project opened: " .. data.project, "info")
    end
end

function onProjectClosed()
    addon.log("Project closed", "info")
end

function onThemeChanged(data)
    if data and data.theme then
        addon.log("Theme changed to: " .. data.theme, "info")
    end
end

function onSettingsChanged()
    addon.log("Settings changed", "info")
end
