# Getting Started with LazyAddons

## Creating a Lua Plugin

Create a directory for your plugin:

```
my-plugin/
├── manifest.yml
└── src/
    └── main.lua
```

**manifest.yml:**

```yaml
id: my.first.plugin
name: My First Plugin
version: 1.0.0
runtime: lua
entry: src/main.lua
api: 1
author: Your Name
description: A demonstration plugin
license: MIT
```

**src/main.lua:**

```lua
function onEnable()
    addon.log("Hello from Lua plugin!")
end

function onDisable()
    addon.log("Goodbye from Lua plugin!")
end

function onAppStarted()
    addon.log("Application has started!")
end
```

Package it as a ZIP file and install via the Addons menu.

## Creating a Python Plugin

Create a directory for your plugin:

```
my-python-plugin/
├── manifest.yml
├── src/
│   └── main.py
└── pyproject.toml
```

**manifest.yml:**

```yaml
id: my.python.plugin
name: My Python Plugin
version: 1.0.0
runtime: python
entry: src/main.py
api: 1
author: Your Name
description: A Python demonstration plugin
license: MIT
```

**pyproject.toml:**

```toml
[project]
name = "my-python-plugin"
version = "1.0.0"
dependencies = []
```

**src/main.py:**

```python
import sys
import json

def on_enable():
    log("Hello from Python plugin!")

def on_disable():
    log("Goodbye from Python plugin!")

# --- Runtime communication ---
def log(msg, level="info"):
    send({"method": "log", "params": {"message": msg, "level": level}})

def send(msg):
    sys.stdout.write(json.dumps(msg) + "\n")
    sys.stdout.flush()

# Main loop: receive and dispatch events
for line in sys.stdin:
    try:
        msg = json.loads(line.strip())
        method = msg.get("method", "")
        if method == "onEnable":
            on_enable()
        elif method == "onDisable":
            on_disable()
    except json.JSONDecodeError:
        pass
```

## Installing a Plugin

1. Open LazyDesktop
2. Go to **Addons > Install Addon...**
3. Select the `.zip` or `.lza` file
4. Review the permissions dialog (if any dangerous permissions are requested)
5. Confirm installation

## The Addon Manager

Open **Addons > Addon Manager...** to:

- View all installed plugins
- Enable/disable plugins
- Reload plugins (without restarting the app)
- Uninstall plugins
- View plugin metadata and permissions
- Open the plugin directory

## Plugin Lifecycle

1. **Install** — archive is extracted to the addons directory
2. **Load** — runtime loads and initializes the entry script
3. **Enable** — `onEnable()` lifecycle hook is called
4. **Active** — plugin receives events and can be interacted with
5. **Disable** — `onDisable()` is called, plugin is paused
6. **Unload** — runtime releases resources
7. **Uninstall** — plugin directory is removed
