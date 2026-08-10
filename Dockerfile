# syntax=docker/dockerfile:1
# LazyDesktop — multi-stage image
#
#   docker compose up -d                # X11 (Linux host) or VNC fallback
#   VNC_MODE=1 docker compose up -d     # force browser/VNC mode
#
# Stage 1 builds the app (Qt 6 + C++23 + bundled Rust ai_core and vcs_core
# crates) and Stage 2 ships only the runtime bits (no toolchain).

FROM ubuntu:24.04 AS builder

# gcc-14 / g++-14 provide C++23; the rest are needed by Qt, yaml-cpp,
# and the Rust build (llama-cpp-2 and aws-lc-sys compile C from source).
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    gcc-14 \
    g++-14 \
    cmake \
    ninja-build \
    pkg-config \
    perl \
    git \
    curl \
    ca-certificates \
    qt6-base-dev \
    libyaml-cpp-dev \
    libgl-dev \
    && rm -rf /var/lib/apt/lists/*

# xmake build system (official installer -> ~/.local/bin)
RUN curl -fsSL https://xmake.io/shget.text | bash

# Rust stable toolchain (builds the bundled ai_core and vcs_core crates)
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal

ENV PATH="/root/.local/bin:/root/.cargo/bin:/usr/lib/qt6/bin:${PATH}" \
    CC=gcc-14 \
    CXX=g++-14

COPY . /src
WORKDIR /src

# Cache cargo + xmake artifacts so rebuilds stay fast.
RUN --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/root/.cargo/git \
    --mount=type=cache,target=/src/target \
    --mount=type=cache,target=/src/.xmake \
    xmake f -y -m release && \
    xmake -m release

# ────────────────────────────────────────────────────────────────

FROM ubuntu:24.04 AS runtime

ENV DEBIAN_FRONTEND=noninteractive

# Qt runtime, git, and the display stack (Xvfb + VNC + noVNC) so the
# app can run against the host X server or fall back to the browser.
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    qt6-base \
    libyaml-cpp0.8 \
    libgl1 \
    libgomp1 \
    git \
    openssh-client \
    shared-mime-info \
    fontconfig \
    fonts-noto-core \
    libxkbcommon-x11-0 \
    libxcb-cursor0 \
    libxcb-xinerama0 \
    xvfb \
    x11vnc \
    websockify \
    novnc \
    tini \
    && rm -rf /var/lib/apt/lists/*

# Non-root user that owns ~/.config/lazydesktop and /workspace.
RUN useradd -m -u 1000 -g 1000 -s /bin/bash lazydesktop \
    && mkdir -p /workspace \
    && chown -R lazydesktop:lazydesktop /home/lazydesktop /workspace

COPY --from=builder /src/build/linux/*/release/lazydesktop /usr/local/bin/lazydesktop
COPY docker/entrypoint.sh /usr/local/bin/lazydesktop-entrypoint

WORKDIR /workspace

EXPOSE 6080

ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/lazydesktop-entrypoint"]
CMD ["lazydesktop"]
