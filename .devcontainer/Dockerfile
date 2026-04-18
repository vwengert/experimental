FROM mcr.microsoft.com/devcontainers/rust:1-1-bookworm

ARG DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        pkg-config \
        cmake \
        clang \
        libfontconfig1-dev \
        libxkbcommon-x11-0 \
        libxkbcommon-dev \
        libwayland-dev \
        libegl1-mesa-dev \
        libgl1-mesa-dev \
        libx11-dev \
        libxcursor-dev \
        libxi-dev \
        libxrandr-dev \
        libxrender-dev \
        libxcb1-dev \
        libinput10 \
        x11-apps \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

RUN su vscode -c "rustup toolchain install 1.90.0 --profile minimal --component rustfmt --component clippy" \
    && su vscode -c "rustup default 1.90.0" \
    && su vscode -c "cargo install cargo-audit --locked"
