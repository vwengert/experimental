# Dev Container

## Minimal setup

This project uses a minimal VS Code dev container configuration in `.devcontainer/devcontainer.json`.

- base image: `mcr.microsoft.com/devcontainers/rust:1-1-bookworm`
- workspace path: `/workspaces/experimental`
- default user: `vscode`
- UID/GID sync: disabled (`updateRemoteUserUID: false`) to avoid host repo ownership changes
- Podman mapping: `--userns=keep-id:uid=1000,gid=1000` keeps workspace writable without chowning host files

## Included Rust tooling

The image installs:

- `rustfmt`
- `clippy`
- `cargo-audit`

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

Rust is pinned to `1.90.0` via `rust-toolchain.toml`.

## Open in container

Use **Dev Containers: Rebuild and Reopen in Container**.

## Verify tools inside container

```bash
cargo --version
cargo audit --version
```
