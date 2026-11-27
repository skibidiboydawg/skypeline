#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# Wireline Linux build & install helper (Ubuntu / Linux Mint / Arch)
#
# Usage:
#   ./linux/build-and-install.sh              # install deps, build, install binary + desktop entry
#   ./linux/build-and-install.sh --no-install # build only (no /usr/local/bin copy)
#   ./linux/build-and-install.sh --help       # show this help
# ---------------------------------------------------------------------------
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

INSTALL=true
case "${1:-}" in
  --no-install) INSTALL=false ;;
  -h|--help)
    grep -E '^# (Wireline|Usage:|  )' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
    exit 0
    ;;
  "")
    ;;
  *)
    echo "Unknown option: $1" >&2
    echo "Try: $0 [--no-install|--help]" >&2
    exit 2
    ;;
esac

APT_PACKAGES=(
  build-essential cmake pkg-config libasound2-dev libgtk-3-dev libssl-dev
  libx11-dev libxrandr-dev libxinerama-dev libxcursor-dev libxi-dev
  libwayland-dev
)
PACMAN_PACKAGES=(
  base-devel cmake pkgconf alsa-lib gtk3 openssl libx11 libxrandr
  libxinerama libxcursor libxi wayland
)

if [[ $EUID -eq 0 ]]; then
  SUDO=()
else
  SUDO=(sudo)
fi

echo "==> Detecting package manager"
if command -v apt-get >/dev/null 2>&1; then
  echo "    apt detected (Ubuntu / Mint / Debian)"
  "${SUDO[@]}" apt-get update
  "${SUDO[@]}" apt-get install -y "${APT_PACKAGES[@]}"
elif command -v pacman >/dev/null 2>&1; then
  echo "    pacman detected (Arch / Arch-based)"
  "${SUDO[@]}" pacman -Syu --needed --noconfirm "${PACMAN_PACKAGES[@]}"
else
  echo "Error: neither apt-get nor pacman was found." >&2
  echo "Install the build dependencies manually and rerun with --no-install." >&2
  exit 1
fi

echo "==> Building Wireline client (release)"
cargo build --release -p skypeline-client

SRC="$ROOT_DIR/target/release/skypeline-client"
DST="$ROOT_DIR/target/release/wireline"
if [[ ! -f "$SRC" ]]; then
  echo "Error: expected binary not found at $SRC" >&2
  exit 1
fi
cp "$SRC" "$DST"
chmod +x "$DST"
echo "    staged: $DST"

if [[ "$INSTALL" == true ]]; then
  echo "==> Installing to /usr/local/bin/wireline"
  "${SUDO[@]}" install -m 0755 "$DST" /usr/local/bin/wireline

  DESKTOP_FILE="$(mktemp)"
  cat > "$DESKTOP_FILE" <<'EOF'
[Desktop Entry]
Name=Wireline
Comment=Desktop chat, voice calls, and screen sharing
Exec=/usr/local/bin/wireline
Terminal=false
Type=Application
Categories=Network;InstantMessaging;Chat;
EOF
  "${SUDO[@]}" install -Dm 0644 "$DESKTOP_FILE" /usr/share/applications/wireline.desktop
  rm -f "$DESKTOP_FILE"
  echo "    You can now launch Wireline from your app menu or run: wireline"
else
  echo "==> --no-install given; skipped system install"
fi

echo "==> Done."
