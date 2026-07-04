-- Hello World Example (Lua)
-- Demonstrates core plugin API and extensibility

function onEnable()
    addon.log("Hello World! Lua plugin is running.", "info")
    addon.notify("Hello World", "Your Lua plugin is now active!")

    -- Register a command accessible via Ctrl+P Command Palette
    addon.registerCommand("hello:greet", function(args)
        local name = args and args[1] or "Plugin User"
        addon.notify("Hello", "Greetings, " .. name .. "!")
        return "Hello, " .. name
    end)

    -- Write initial data
    addon.createFile("hello.txt", "Hello from Lua plugin!\n")

    -- Contribute to the Addons menu
    addon.createMenu("Addons", "Say Hello", "hello:greet")
end

function onDisable()
    addon.log("Hello World plugin disabled.", "info")
end

function onAppStarted()
    addon.log("LazyDesktop has started! Hello World plugin ready.", "info")
end
