# syntax=docker/dockerfile:1
# LazyDesktop — multi-stage image
#
#   docker compose up -d                # X11 (Linux host) or VNC fallback
#   VNC_MODE=1 docker compose up -d     # force browser/VNC mode
#
# Stage 1 builds the Rust workspace (GPUI app + ai_core + vcs_core crates)
# and Stage 2 ships only the runtime bits (no toolchain).

FROM ubuntu:24.04 AS builder

# Needed by the Rust build: ai_core (llama-cpp-2) and aws-lc-sys compile C/C++
# from source, so a full toolchain plus cmake must be present.
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    cmake \
    ninja-build \
    pkg-config \
    perl \
    git \
    curl \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Rust stable toolchain (builds the GPUI app and the bundled crates)
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal

ENV PATH="/root/.cargo/bin:${PATH}"

COPY . /src
WORKDIR /src

# Cache cargo artifacts so rebuilds stay fast.
RUN --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/root/.cargo/git \
    --mount=type=cache,target=/src/target \
    cargo build --workspace --release

# ────────────────────────────────────────────────────────────────

FROM ubuntu:24.04 AS runtime

ENV DEBIAN_FRONTEND=noninteractive

# Runtime libraries for the GPUI app (X11/GL stack, fonts), git, and the
# display stack (Xvfb + VNC + noVNC) so the app can run against the host X
# server or fall back to the browser.
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libgl1 \
    libxkbcommon-x11-0 \
    shared-mime-info \
    fontconfig \
    fonts-noto-core \
    git \
    openssh-client \
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

COPY --from=builder /src/target/release/lazydesktop /usr/local/bin/lazydesktop
COPY docker/entrypoint.sh /usr/local/bin/lazydesktop-entrypoint

WORKDIR /workspace

EXPOSE 6080