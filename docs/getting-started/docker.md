# Docker

The Docker image builds the GPUI app and the bundled Rust crates (`ai_core`,
`vcs_core`), then ships only the runtime — no toolchain. The window either
connects to your host's X server or falls back to a browser-accessible VNC
session, so you can run LazyDesktop on macOS, Windows, or a headless server.

## Quick start

```bash
docker compose build            # build the image (first time only)

# Linux host with an X server -> native window
docker compose up -d

# Any OS (macOS/Windows) or headless -> open http://localhost:6080 in a browser
VNC_MODE=1 docker compose up -d
```

The `justfile` shortcuts cover the same workflows:

```bash
just docker-build     # docker compose build
just docker-up        # docker compose up -d
just docker-up-vnc    # VNC_MODE=1 docker compose up -d
just docker-down      # docker compose down
just docker-shell     # docker compose exec lazydesktop /bin/bash
just docker-logs      # docker compose logs -f
```

## How the display backend is chosen

The entrypoint (`docker/entrypoint.sh`) picks a backend at startup:

- **Host X11** — when an X socket is mounted and `DISPLAY` is set, the app
  opens as a native window on your desktop.
- **Xvfb + VNC/noVNC** — otherwise, or when `VNC_MODE=1`, the app renders to
  a virtual display and you connect through the browser at
  `http://localhost:6080`.

## Volumes and environment

By default the container mounts:

| Mount | Purpose |
|-------|---------|
| `/tmp/.X11-unix` | Host X socket for the native window |
| `lazydesktop-config` (named volume) | Settings, projects, themes, models |
| `./repos` → `/workspace` | Your Git repositories (bind-mount a folder here) |

Mount your own repositories with `REPO_DIR`:

```bash
REPO_DIR=~/code/myrepo docker compose up -d
```

Relevant environment variables:

| Variable | Default | Purpose |
|----------|---------|---------|
| `DISPLAY` | `:0` | Host X display |
| `PUID` / `PGID` | `1000` / `1000` | Re-map the app user so mounted config is writable |
| `VNC_MODE` | `0` | Force browser/VNC mode (`1`) |
| `RESOLUTION` | `1280x800x24` | Virtual display resolution |
| `VNC_PORT` | `5900` | VNC port inside the container |
| `NOVNC_PORT` | `6080` | noVNC (browser) port inside the container |

If the mounted config is owned by root, set `PUID`/`PGID` to your host UID
and GID:

```bash
PUID=$(id -u) PGID=$(id -g) docker compose up -d
```

## Notes

- The image ships only the runtime — no toolchain. Build deps (cmake, ninja,
  Rust) live in the multi-stage builder.
- Local model files downloaded inside the container are stored in the named
  volume and persist across container recreations.
