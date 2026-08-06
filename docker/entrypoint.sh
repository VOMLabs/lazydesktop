#!/usr/bin/env bash
# LazyDesktop container entrypoint.
#
# Picks a display backend:
#   - host X11 when an X socket is mounted and DISPLAY is set
#   - Xvfb + VNC/noVNC otherwise (or when VNC_MODE=1)
#
# PUID/PGID re-map the app user so mounted config is not root-owned.

set -euo pipefail

APP_USER="${APP_USER:-lazydesktop}"
APP_UID="${PUID:-1000}"
APP_GID="${PGID:-1000}"
APP_HOME="/home/${APP_USER}"

DISPLAY_NUM="${DISPLAY_NUM:-99}"
RESOLUTION="${RESOLUTION:-1280x800x24}"
VNC_PORT="${VNC_PORT:-5900}"
NOVNC_PORT="${NOVNC_PORT:-6080}"
VNC_DISPLAY=":${DISPLAY_NUM}"

# ─── User setup ────────────────────────────────────────────────
if [ "$(id -u)" = "0" ]; then
    if ! getent group "$APP_GID" >/dev/null 2>&1; then
        groupadd -g "$APP_GID" "$APP_USER" || true
    fi
    if getent passwd "$APP_USER" >/dev/null 2>&1; then
        usermod -u "$APP_UID" -g "$APP_GID" "$APP_USER"
    else
        useradd -m -u "$APP_UID" -g "$APP_GID" -s /bin/bash "$APP_USER"
    fi
    mkdir -p "${APP_HOME}/.config" "${APP_HOME}/.cache"
    chown -R "$APP_UID:$APP_GID" "$APP_HOME" /workspace
fi

# ─── Display selection ──────────────────────────────────────────
USE_VNC=0
if [ "${VNC_MODE:-0}" = "1" ]; then
    USE_VNC=1
elif [ -z "${DISPLAY:-}" ] || ! ls /tmp/.X11-unix/X* >/dev/null 2>&1; then
    USE_VNC=1
fi

if [ "$USE_VNC" = "1" ]; then
    Xvfb "$VNC_DISPLAY" -screen 0 "$RESOLUTION" -nolisten tcp >/tmp/xvfb.log 2>&1 &
    export DISPLAY="$VNC_DISPLAY"
    for _ in $(seq 1 60); do
        [ -e "/tmp/.X11-unix/X${DISPLAY_NUM}" ] && break
        sleep 0.5
    done
    x11vnc -display "$DISPLAY" -forever -shared -nopw -quiet >/tmp/x11vnc.log 2>&1 &
    websockify --web=/usr/share/novnc "$NOVNC_PORT" "localhost:${VNC_PORT}" >/tmp/websockify.log 2>&1 &
    echo "[lazydesktop] VNC :${VNC_PORT} | noVNC: http://localhost:${NOVNC_PORT}/vnc.html"
else
    echo "[lazydesktop] connecting to host X11 display: ${DISPLAY:-:0}"
fi

# ─── Qt environment ─────────────────────────────────────────────
export LIBGL_ALWAYS_SOFTWARE="${LIBGL_ALWAYS_SOFTWARE:-1}"
export QT_QPA_PLATFORM="${QT_QPA_PLATFORM:-xcb}"
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/tmp/runtime-${APP_USER}}"
mkdir -p "$XDG_RUNTIME_DIR"
chown "$APP_UID:$APP_GID" "$XDG_RUNTIME_DIR" 2>/dev/null || true
chmod 700 "$XDG_RUNTIME_DIR" 2>/dev/null || true

# ─── Drop privileges and run ────────────────────────────────────
if [ "$(id -u)" = "0" ]; then
    exec setpriv --reuid="$APP_UID" --regid="$APP_GID" --init-groups "$@"
fi
exec "$@"
