# Build, Test, and Run (Fedora)

## 1) Install prerequisites

### System packages
Install common Rust + native build dependencies used by this workspace and the GUI stack:

```bash
sudo dnf install -y \
  gcc gcc-c++ clang make cmake \
  pkgconf-pkg-config \
  fontconfig-devel \
  libxkbcommon-devel wayland-devel \
  mesa-libEGL-devel mesa-libGL-devel \
  libX11-devel libXcursor-devel libXi-devel libXrandr-devel libXrender-devel libxcb-devel
```

If you want to use a Qt backend for Slint (optional), also install:

```bash
sudo dnf install -y qt6-qtbase-devel
```

### Rust toolchain
Install Rust with `rustup` (if not installed yet):

```bash
sudo dnf install -y rustup
rustup-init -y
source "$HOME/.cargo/env"
rustup toolchain install stable
rustup default stable
```

## 2) Build

From the repository root:

```bash
cd /home/vwengert/Repos/experimental
cargo check
cargo build
```

Build only one crate:

```bash
cargo build -p domain
cargo build -p cli
cargo build -p gui
```

## 3) Test

Run all tests in the workspace:

```bash
cargo test
```

Run tests for a single crate:

```bash
cargo test -p domain
cargo test -p cli
cargo test -p gui
```

## 4) Run

Run the GUI app:

```bash
cargo run -p gui
```

Run the CLI app:

```bash
cargo run -p cli
```

## 5) `lists.config.json` format

The default built-in configuration is embedded from:

- `domain/assets/lists.config.json`

The file uses this top-level structure:

- `title`: human-readable config title
- `description`: short config description
- `properties`: actual config payload
  - `units`: map of unit groups to unit strings
  - `elements`: map of element names to element schemas

Each element schema supports:

- `allow_init` (optional, default `false`): whether it can be added on init
- `fields`: ordered list of field definitions
  - `name`: field name
  - `spec.ty`: value type (`Str`, `Int`, `Float`, `Bool`)
  - `spec.unit` (optional): references a unit group from `properties.units`

Example:

```json
{
  "title": "Lists Config",
  "description": "Default configuration",
  "properties": {
    "units": {
      "length": ["px", "em", "rem", "%"]
    },
    "elements": {
      "Container": {
        "allow_init": true,
        "fields": [
          { "name": "width", "spec": { "ty": "Float", "unit": "length" } },
          { "name": "height", "spec": { "ty": "Float", "unit": "length" } },
          { "name": "padding", "spec": { "ty": "Int", "unit": "length" } }
        ]
      }
    }
  }
}
```
