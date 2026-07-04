# Tutorial: Your First LazyAddons Plugin

In this tutorial, you'll create a simple Lua plugin that logs a greeting
when LazyDesktop starts.

## Prerequisites

- LazyDesktop installed
- Basic familiarity with Lua

## Step 1: Create the Project Structure

```
hello-lazy/
├── manifest.yml
└── src/
    └── main.lua
```

## Step 2: Write the Manifest

**hello-lazy/manifest.yml:**

```yaml
id: com.example.hello-lazy
name: Hello Lazy
version: 1.0.0
runtime: lua
entry: src/main.lua
api: 1
author: Your Name
description: Says hello when LazyDesktop starts
license: MIT
```

## Step 3: Write the Plugin Code

**hello-lazy/src/main.lua:**

```lua
function onEnable()
    addon.log("Hello from Hello Lazy plugin!", "info")
end

function onAppStarted()
    addon.log("LazyDesktop is now running!", "info")
    addon.notify("Hello Lazy", "Thanks for using LazyDesktop!")
end

function onDisable()
    addon.log("Goodbye!", "info")
end
```

## Step 4: Package the Plugin

```bash
cd hello-lazy
zip -r ../hello-lazy.lza .
```

## Step 5: Install

1. Open LazyDesktop
2. Go to **Addons > Install Addon...**
3. Select `hello-lazy.lza`
4. The plugin will be installed and enabled automatically

## Step 6: Verify

- Check the plugin appears in **Addons > Addon Manager...**
- Restart LazyDesktop and look for the notification

## Python Version

The same plugin in Python:

**hello-lazy/src/main.py:**

```python
import sys
import json

def send(msg):
    sys.stdout.write(json.dumps(msg) + "\n")
    sys.stdout.flush()

def log(msg, level="info"):
    send({"method": "log", "params": {"message": msg, "level": level}})

for line in sys.stdin:
    try:
        msg = json.loads(line.strip())
        method = msg.get("method", "")

        if method == "onEnable":
            log("Hello from Hello Lazy plugin!")
        elif method == "onAppStarted":
            log("LazyDesktop is now running!")
        elif method == "onDisable":
            log("Goodbye!")

        send({"result": {}, "id": msg.get("id", 0)})
    except json.JSONDecodeError:
        pass
```

**hello-lazy/pyproject.toml:**

```toml
[project]
name = "hello-lazy"
version = "1.0.0"
dependencies = []
```

Update the manifest's `runtime` to `python` and `entry` to `src/main.py`.
