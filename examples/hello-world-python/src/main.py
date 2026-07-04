"""
Hello World Example (Python)
Demonstrates core plugin API and extensibility
"""

import sys
import json


def send(msg):
    sys.stdout.write(json.dumps(msg) + "\n")
    sys.stdout.flush()


def log(msg, level="info"):
    send({"method": "log", "params": {"message": msg, "level": level}})


def notify(title, message):
    send({"method": "notify", "params": {"title": title, "message": message}})


HANDLERS = {}

for line in sys.stdin:
    line = line.strip()
    if not line:
        continue

    try:
        msg = json.loads(line)
        method = msg.get("method", "")
        msg_id = msg.get("id", 0)
        params = msg.get("params", {})

        if method == "onEnable":
            log("Hello World! Python plugin is running.")
            notify("Hello World", "Your Python plugin is now active!")

            # Register command
            send({"method": "register_command",
                  "params": {"name": "hello:pygreet"}})

            # Write data
            send({"method": "create_file",
                  "params": {"path": "hello.txt", "content": "Hello from Python!\n"}})

            # Add menu item
            send({"method": "create_menu",
                  "params": {"parent": "Addons", "label": "Say Hello (Python)",
                             "command": "hello:pygreet"}})

        elif method == "execute_command":
            cmd_name = params.get("name", "")
            cmd_args = params.get("args", [])
            if cmd_name == "hello:pygreet":
                name = cmd_args[0] if cmd_args else "Python User"
                notify("Hello", f"Greetings, {name}!")

        elif method == "onDisable":
            log("Hello World plugin disabled.")
        elif method == "onAppStarted":
            log("LazyDesktop has started! Hello World plugin ready.")

        send({"result": {}, "id": msg_id})
    except json.JSONDecodeError:
        pass
    except Exception as e:
        log(f"Error: {e}", "error")
