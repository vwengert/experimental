# Dev Container

## Setup

This project uses an image-based dev container configuration in `.devcontainer/devcontainer.json`.

- base image: `mcr.microsoft.com/devcontainers/rust:1-1-bookworm` (configured directly in `devcontainer.json`)
- workspace path: `/workspaces/experimental`
- default user: `ubuntu`
- Podman mapping: `--userns=keep-id:uid=1000,gid=1000` keeps workspace writable without chowning host files

## Included Rust tooling

During `postCreateCommand`, the container installs and configures:

- `rustfmt`
- `clippy`
- `cargo-audit`
- `bacon`
- `cargo-nextest`

Rust is pinned to `1.90.0` via `rust-toolchain.toml`, and `postCreateCommand` also installs/sets toolchain `1.90.0`.

## Included system packages

During `postCreateCommand`, these packages are installed:

- `pkg-config`, `cmake`, `clang`
- `libfontconfig1-dev`, `libxkbcommon-x11-0`, `libxkbcommon-dev`
- `libwayland-dev`, `libegl1-mesa-dev`, `libgl1-mesa-dev`
- `libx11-dev`, `libxcursor-dev`, `libxi-dev`, `libxrandr-dev`, `libxrender-dev`, `libxcb1-dev`
- `libinput10`, `x11-apps`

## VS Code extensions

Auto-installed in container (`devcontainer.json`):

- `zerotaskx.rust-extension-pack`
- `slint.slint`
- `davidanson.vscode-markdownlint`
- `ms-vscode.test-adapter-converter`
- `hbenl.vscode-test-explorer`
- `esbenp.prettier-vscode`

Recommended in workspace (`.vscode/extensions.json`):

- `zerotaskx.rust-extension-pack`
- `slint.slint`
- `davidanson.vscode-markdownlint`
- `ms-vscode.test-adapter-converter`
- `hbenl.vscode-test-explorer`
- `esbenp.prettier-vscode`


## Open in container

Use **Dev Containers: Rebuild and Reopen in Container**.

## Verify tools inside container

```bash
cargo --version
cargo audit --version
```
