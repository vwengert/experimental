#!/bin/bash
set -e # Bricht das Skript ab, falls ein Befehl fehlschlägt

echo "=== Starte Devcontainer-Setup ==="

curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash

cargo binstall --no-confirm --disable-telemetry bacon cargo-audit cargo-nextest ripgrep

echo "Installiere lazygit..."
LAZYGIT_VERSION=$(curl -s "https://api.github.com/repos/jesseduffield/lazygit/releases/latest" | grep -Po '"tag_name": *"v\K[^"]*')
LAZYGIT_ARCH=$(uname -m | sed -e 's/aarch64/arm64/')

curl -Lo lazygit.tar.gz "https://github.com/jesseduffield/lazygit/releases/download/v${LAZYGIT_VERSION}/lazygit_${LAZYGIT_VERSION}_Linux_${LAZYGIT_ARCH}.tar.gz"
tar xf lazygit.tar.gz lazygit
sudo install lazygit -D -t /usr/local/bin/
rm lazygit.tar.gz lazygit

sudo apt-get update
sudo apt-get install -y --no-install-recommends \
        pkg-config cmake clang libfontconfig1-dev libxkbcommon-x11-0 libxkbcommon-dev \
        libwayland-dev libegl1-mesa-dev libgl1-mesa-dev libx11-dev libxcursor-dev \
        libxi-dev libxrandr-dev libxrender-dev libxcb1-dev libinput10 x11-apps

sudo apt-get clean
sudo rm -rf /var/lib/apt/lists/*

echo "=== Setup erfolgreich beendet ==="
