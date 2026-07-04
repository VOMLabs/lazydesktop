# Python Plugin API

Python plugins communicate with LazyDesktop via JSON messages over stdin/stdout.
The plugin runs as a child process managed by `uv` inside an isolated virtual
environment.

## Communication Protocol

LazyDesktop sends JSON messages to the plugin's stdin, one per line:

```json
{"method": "onEnable", "id": 1}
```

The plugin responds via stdout:

```json
{"result": {}, "id": 1}
```

Plugins can also send requests to the host:

```json
{"method": "log", "params": {"message": "Hello", "level": "info"}}
```

## Base Template

```python
import sys
import json

def send(msg):
    sys.stdout.write(json.dumps(msg) + "\n")
    sys.stdout.flush()

def log(message, level="info"):
    send({"method": "log", "params": {"message": message, "level": level}})

# --- Lifecycle Hooks ---

def on_load():
    log("Plugin loaded")

def on_enable():
    log("Plugin enabled")

def on_disable():
    log("Plugin disabled")

def on_unload():
    log("Plugin unloaded")

# --- Main loop ---

for line in sys.stdin:
    try:
        msg = json.loads(line.strip())
        method = msg.get("method", "")
        params = msg.get("params", {})

        if method == "onLoad":
            on_load()
        elif method == "onEnable":
            on_enable()
        elif method == "onDisable":
            on_disable()
        elif method == "onUnload":
            on_unload()
        elif method == "onAppStarted":
            log("App started")
        elif method == "onAppShuttingDown":
            log("App shutting down")
        elif method == "onProjectOpened":
            log("Project opened: " + params.get("data", {}).get("project", ""))
        elif method == "onProjectClosed":
            log("Project closed")
        elif method == "onThemeChanged":
            log("Theme changed: " + params.get("data", {}).get("theme", ""))

        send({"result": {}, "id": msg.get("id", 0)})
    except json.JSONDecodeError:
        pass
    except Exception as e:
        send({"method": "log", "params": {"message": str(e), "level": "error"}})
```

## Lifecycle Events

Events received from the host:

| Method | Description |
|--------|-------------|
| `onLoad` | Plugin script loaded |
| `onEnable` | Plugin enabled |
| `onDisable` | Plugin disabled |
| `onUnload` | Plugin about to be unloaded |
| `onAppStarted` | Application started |
| `onAppShuttingDown` | Application shutting down |
| `onProjectOpened` | Project opened (`params.data.project`) |
| `onProjectClosed` | Project closed |
| `onThemeChanged` | Theme changed (`params.data.theme`) |
| `onFileOpened` | File opened (`params.data.file`) |
| `onFileSaved` | File saved (`params.data.file`) |
| `onSettingsChanged` | Plugin settings changed |

## Sending Messages to Host

### Log

```python
send({"method": "log", "params": {"message": "text", "level": "info"}})
```

Levels: `"info"`, `"warn"`, `"error"`, `"debug"`

### Notify

```python
send({"method": "notify", "params": {"title": "Title", "message": "Body"}})
```

## Dependencies

Python plugins use `pyproject.toml` for dependency management. Dependencies
are automatically installed by `uv sync` when the plugin is first loaded.

```toml
[project]
name = "my-plugin"
version = "1.0.0"
dependencies = [
    "requests>=2.28.0",
    "rich",
]
```
