#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# Wireline Linux web installer (Arch / Linux Mint / Ubuntu)
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/skibidiboydawg/wireline/main/linux/install.sh | bash
# ---------------------------------------------------------------------------
set -euo pipefail

REPO="skibidiboydawg/wireline"
TARBALL_URL="https://github.com/${REPO}/releases/latest/download/wireline-linux-x86_64.tar.gz"

if [[ $EUID -eq 0 ]]; then
  SUDO=()
else
  SUDO=(sudo)
fi

install_runtime_deps() {
  if command -v apt-get >/dev/null 2>&1; then
    "${SUDO[@]}" apt-get update
    "${SUDO[@]}" apt-get install -y libasound2t64 libgtk-3-0 libssl3 libx11-6 libxrandr2 libxinerama1 libxcursor1 libxi6 libwayland-client0 libxdo3 libdbus-1-3 libxcb1 || \
    "${SUDO[@]}" apt-get install -y libasound2 libgtk-3-0 libssl3 libx11-6 libxrandr2 libxinerama1 libxcursor1 libxi6 libwayland-client0 libxdo3 libdbus-1-3 libxcb1
  elif command -v pacman >/dev/null 2>&1; then
    "${SUDO[@]}" pacman -Syu --needed --noconfirm alsa-lib gtk3 openssl libx11 libxrandr libxinerama libxcursor libxi wayland xdotool dbus libxcb
  fi
}

install_desktop_entry() {
  local desktop_file
  desktop_file="$(mktemp)"
  cat > "$desktop_file" <<'EOF'
[Desktop Entry]
Name=Wireline
Comment=Desktop chat, voice calls, and screen sharing
Exec=/usr/local/bin/wireline
Terminal=false
Type=Application
Categories=Network;InstantMessaging;Chat;
EOF
  "${SUDO[@]}" install -Dm 0644 "$desktop_file" /usr/share/applications/wireline.desktop
  rm -f "$desktop_file"
}

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

echo "==> Downloading Wireline for Linux..."
if curl -fsSL "$TARBALL_URL" -o "$TMP_DIR/wireline.tar.gz"; then
  install_runtime_deps
  tar -xzf "$TMP_DIR/wireline.tar.gz" -C "$TMP_DIR"
  "${SUDO[@]}" install -m 0755 "$TMP_DIR/wireline" /usr/local/bin/wireline
  install_desktop_entry
  echo "==> Wireline installed! Launch it from your app menu or run: wireline"
  exit 0
fi

echo "==> Prebuilt release not found; building from source..."
if ! command -v git >/dev/null 2>&1 || ! command -v cargo >/dev/null 2>&1; then
  echo "Please install git and Rust (rustup) first, then re-run this script." >&2
  exit 1
fi

git clone --depth 1 "https://github.com/${REPO}.git" "$TMP_DIR/src"
bash "$TMP_DIR/src/linux/build-and-install.sh"
