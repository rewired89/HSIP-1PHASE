#!/usr/bin/env sh
# HSIP installer — macOS and Linux
# Usage: curl -sSf https://raw.githubusercontent.com/rewired89/HSIP-1PHASE/main/install.sh | sh
#
# What this does:
#   1. Detects your OS and architecture
#   2. Downloads the correct HSIP binary from GitHub Releases
#   3. Installs it to /usr/local/bin/hsip (or ~/.local/bin/hsip if no sudo)
#   4. Verifies the binary runs
#
# To uninstall: rm $(which hsip)

set -e

REPO="rewired89/HSIP-1PHASE"
BINARY_NAME="hsip"
INSTALL_DIR="/usr/local/bin"
FALLBACK_INSTALL_DIR="$HOME/.local/bin"

# Colors (only if terminal supports it)
if [ -t 1 ]; then
  BOLD="\033[1m"
  GREEN="\033[0;32m"
  YELLOW="\033[0;33m"
  RED="\033[0;31m"
  RESET="\033[0m"
else
  BOLD="" GREEN="" YELLOW="" RED="" RESET=""
fi

say()     { printf "${BOLD}%s${RESET}\n" "$1"; }
success() { printf "${GREEN}✓ %s${RESET}\n" "$1"; }
warn()    { printf "${YELLOW}! %s${RESET}\n" "$1"; }
err()     { printf "${RED}✗ %s${RESET}\n" "$1" >&2; exit 1; }

# Detect OS
detect_os() {
  OS="$(uname -s)"
  case "$OS" in
    Linux*)  echo "linux" ;;
    Darwin*) echo "macos" ;;
    *)       err "Unsupported OS: $OS. Install manually from https://github.com/$REPO/releases" ;;
  esac
}

# Detect architecture
detect_arch() {
  ARCH="$(uname -m)"
  case "$ARCH" in
    x86_64|amd64) echo "x64" ;;
    aarch64|arm64) echo "arm64" ;;
    *) err "Unsupported architecture: $ARCH. Install manually from https://github.com/$REPO/releases" ;;
  esac
}

# Get latest release tag from GitHub
get_latest_version() {
  if command -v curl >/dev/null 2>&1; then
    curl -sSf "https://api.github.com/repos/$REPO/releases/latest" \
      | grep '"tag_name"' \
      | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/'
  elif command -v wget >/dev/null 2>&1; then
    wget -qO- "https://api.github.com/repos/$REPO/releases/latest" \
      | grep '"tag_name"' \
      | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/'
  else
    err "curl or wget is required. Please install one and try again."
  fi
}

# Download a file
download() {
  URL="$1"
  DEST="$2"
  if command -v curl >/dev/null 2>&1; then
    curl -sSfL "$URL" -o "$DEST"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO "$DEST" "$URL"
  else
    err "curl or wget is required."
  fi
}

# Determine install directory (with or without sudo)
pick_install_dir() {
  if [ -w "$INSTALL_DIR" ]; then
    echo "$INSTALL_DIR"
  elif command -v sudo >/dev/null 2>&1 && sudo -n true 2>/dev/null; then
    echo "$INSTALL_DIR"
  else
    mkdir -p "$FALLBACK_INSTALL_DIR"
    echo "$FALLBACK_INSTALL_DIR"
  fi
}

main() {
  say "HSIP Installer"
  say "=============="

  OS="$(detect_os)"
  ARCH="$(detect_arch)"

  say "Detected: $OS / $ARCH"

  say "Fetching latest version..."
  VERSION="$(get_latest_version)"
  if [ -z "$VERSION" ]; then
    err "Could not determine latest version. Check your internet connection or visit https://github.com/$REPO/releases"
  fi
  say "Latest version: $VERSION"

  # Build asset name
  # Expected release asset names: hsip-macos-arm64, hsip-macos-x64, hsip-linux-x64
  case "$OS" in
    macos) ASSET="hsip-macos-$ARCH" ;;
    linux) ASSET="hsip-linux-$ARCH" ;;
  esac

  DOWNLOAD_URL="https://github.com/$REPO/releases/download/$VERSION/$ASSET"
  DEST_DIR="$(pick_install_dir)"
  DEST="$DEST_DIR/$BINARY_NAME"

  say "Downloading $ASSET..."
  TMP="$(mktemp)"
  download "$DOWNLOAD_URL" "$TMP" || err "Download failed. Check that $VERSION has a $ASSET asset at:\n  $DOWNLOAD_URL"

  chmod +x "$TMP"

  say "Installing to $DEST..."
  if [ -w "$DEST_DIR" ]; then
    mv "$TMP" "$DEST"
  else
    sudo mv "$TMP" "$DEST"
  fi

  # Verify
  if "$DEST" --version >/dev/null 2>&1 || "$DEST" --help >/dev/null 2>&1; then
    success "HSIP installed successfully at $DEST"
  else
    success "HSIP installed at $DEST"
  fi

  # PATH hint if using fallback dir
  if [ "$DEST_DIR" = "$FALLBACK_INSTALL_DIR" ]; then
    if ! echo "$PATH" | grep -q "$FALLBACK_INSTALL_DIR"; then
      warn "$FALLBACK_INSTALL_DIR is not in your PATH."
      warn "Add this to your shell config (~/.bashrc, ~/.zshrc, etc.):"
      printf "\n  export PATH=\"\$HOME/.local/bin:\$PATH\"\n\n"
    fi
  fi

  say ""
  say "Run HSIP:"
  printf "  ${BOLD}hsip${RESET}      — starts the server, opens your browser automatically\n"
  printf "  ${BOLD}hsip --help${RESET} — CLI reference\n"
  say ""
  say "Your API key will be saved to: ~/.hsip/admin.key"
  say "Docs and API reference: http://127.0.0.1:7777/docs  (once running)"
  say ""
  success "Done. Run: hsip"
}

main "$@"
