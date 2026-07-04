"""
My Python Plugin
A LazyAddons plugin template for Python

Demonstrates the full plugin API:
  - Lifecycle hooks
  - File CRUD within sandboxed data dir
  - Commands & menu contributions
  - Notifications & logging
"""

import sys
import json


def send(msg):
    """Send a JSON message to the host application."""
    sys.stdout.write(json.dumps(msg) + "\n")
    sys.stdout.flush()


def log(message, level="info"):
    """Log a message through the host application."""
    send({"method": "log", "params": {"message": message, "level": level}})


def notify(title, message):
    """Show a notification through the host application."""
    send({"method": "notify", "params": {"title": title, "message": message}})


# --- Extensibility helpers ---

def register_command(name):
    """Decorator: register a command callable from the Command Palette."""
    def wrapper(fn):
        send({"method": "register_command", "params": {"name": name}})
        HANDLERS["execute_command_" + name] = fn
        return fn
    return wrapper


def create_file(path, content):
    """Write a file in the plugin's sandboxed data directory."""
    send({"method": "create_file", "params": {"path": path, "content": content}})


def read_file(path):
    """Read a file from the plugin's data directory. Returns content string."""
    send({"method": "read_file", "params": {"path": path}})


def delete_file(path):
    """Delete a file from the plugin's data directory."""
    send({"method": "delete_file", "params": {"path": path}})


def list_files(dir=""):
    """List files in the plugin's data directory."""
    send({"method": "list_files", "params": {"dir": dir}})


def create_menu(parent, label, command_id):
    """Add a menu item to the application menu bar."""
    send({"method": "create_menu",
          "params": {"parent": parent, "label": label, "command": command_id}})


# --- Command handlers (registered on enable) ---

@register_command("python:greet")
def cmd_greet(args):
    name = args[0] if args else "world"
    log(f"Hello, {name}!")
    return f"Greeted {name}"


@register_command("python:write-data")
def cmd_write_data(args):
    text = args[0] if args else "default data"
    create_file("mydata.txt", text)
    log(f"Data saved: {text}")
    return "Saved"


@register_command("python:read-data")
def cmd_read_data(args):
    read_file("mydata.txt")
    return "Reading..."


@register_command("python:list-files")
def cmd_list_files(args):
    list_files()
    return "Listing..."


# --- Lifecycle Hooks ---

def on_load():
    log("Plugin loading...", "debug")


def on_enable():
    log("Hello from Python!", "info")
    notify("Plugin Enabled", "My Python Plugin is now active")

    create_menu("Addons", "Greet from Python", "python:greet")
    create_menu("Addons", "Write Data", "python:write-data")


def on_disable():
    log("Plugin disabled", "info")


def on_unload():
    log("Plugin unloading", "debug")


def on_app_started():
    log("Application started", "info")


def on_app_shutting_down():
    log("Application shutting down", "info")


def on_project_opened(data):
    project = (data or {}).get("project", "")
    if project:
        log(f"Project opened: {project}", "info")


def on_project_closed():
    log("Project closed", "info")


def on_theme_changed(data):
    theme = (data or {}).get("theme", "")
    if theme:
        log(f"Theme changed to: {theme}", "info")


# --- Main Loop ---

HANDLERS = {
    "onLoad": on_load,
    "onEnable": on_enable,
    "onDisable": on_disable,
    "onUnload": on_unload,
    "onAppStarted": on_app_started,
    "onAppShuttingDown": on_app_shutting_down,
    "onProjectOpened": on_project_opened,
    "onProjectClosed": on_project_closed,
    "onThemeChanged": on_theme_changed,
    "onSettingsChanged": lambda: log("Settings changed", "info"),
}

for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    try:
        msg = json.loads(line)
        method = msg.get("method", "")
        params = msg.get("params", {})
        data = params.get("data", {})
        msg_id = msg.get("id", 0)

        # Handle execute_command calls from the host
        if method == "execute_command":
            cmd_name = params.get("name", "")
            cmd_args = params.get("args", [])
            handler = HANDLERS.get("execute_command_" + cmd_name)
            if handler:
                result = handler(cmd_args)
                if result is not None:
                    send({"result": {"return": result}, "id": msg_id})
                continue

        handler = HANDLERS.get(method)
        if handler:
            if data:
                handler(data)
            else:
                handler()

        send({"result": {}, "id": msg_id})
    except json.JSONDecodeError:
        pass
    except Exception as e:
        log(f"Error: {e}", "error")
