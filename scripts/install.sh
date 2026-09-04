#!/usr/bin/env bash
# Install lpkg from a GitHub release binary, or build from source if cargo is available.
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/MatoloJr/lpkg/main/scripts/install.sh | bash
#   PREFIX=/usr/local bash scripts/install.sh
set -euo pipefail

REPO="${LPKG_REPO:-MatoloJr/lpkg}"
PREFIX="${PREFIX:-/usr/local}"
BINDIR="${BINDIR:-$PREFIX/bin}"
DATADIR="${DATADIR:-$PREFIX/share/lpkg}"
TMPDIR="${TMPDIR:-/tmp}"
INSTALL_DIR="$(mktemp -d "${TMPDIR}/lpkg-install.XXXXXX")"
trap 'rm -rf "$INSTALL_DIR"' EXIT

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "error: required command '$1' not found" >&2
    exit 1
  }
}

detect_arch() {
  local m
  m="$(uname -m)"
  case "$m" in
    x86_64|amd64) echo "x86_64" ;;
    aarch64|arm64) echo "aarch64" ;;
    *)
      echo "error: unsupported architecture: $m" >&2
      exit 1
      ;;
  esac
}

install_files() {
  local bin="$1"
  local data_src="$2"
  echo "Installing to $BINDIR/lpkg ..."
  if [[ -w "$BINDIR" ]] || [[ -w "$PREFIX" ]]; then
    install -d "$BINDIR" "$DATADIR"
    install -m 755 "$bin" "$BINDIR/lpkg"
    if [[ -d "$data_src" ]]; then
      install -m 644 "$data_src/aliases.toml" "$DATADIR/aliases.toml" 2>/dev/null || true
      install -m 644 "$data_src/direct-deb-index.json" "$DATADIR/direct-deb-index.json" 2>/dev/null || true
    fi
  else
    need_cmd sudo
    sudo install -d "$BINDIR" "$DATADIR"
    sudo install -m 755 "$bin" "$BINDIR/lpkg"
    if [[ -d "$data_src" ]]; then
      sudo install -m 644 "$data_src/aliases.toml" "$DATADIR/aliases.toml" 2>/dev/null || true
      sudo install -m 644 "$data_src/direct-deb-index.json" "$DATADIR/direct-deb-index.json" 2>/dev/null || true
    fi
  fi
  echo "Installed lpkg $($BINDIR/lpkg --version 2>/dev/null || echo ok)"
}

install_from_release() {
  need_cmd curl
  need_cmd tar
  local arch tag asset url
  arch="$(detect_arch)"
  echo "Fetching latest release from GitHub (${REPO})..."
  tag="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | sed -n 's/.*"tag_name":[[:space:]]*"\([^"]*\)".*/\1/p' | head -1)"
  if [[ -z "$tag" ]]; then
    return 1
  fi
  asset="lpkg-${tag}-linux-${arch}.tar.gz"
  url="https://github.com/${REPO}/releases/download/${tag}/${asset}"
  echo "Downloading ${url} ..."
  if ! curl -fsSL "$url" -o "$INSTALL_DIR/lpkg.tar.gz"; then
    return 1
  fi
  tar -xzf "$INSTALL_DIR/lpkg.tar.gz" -C "$INSTALL_DIR"
  local bin
  bin="$(find "$INSTALL_DIR" -type f -name lpkg | head -1)"
  if [[ -z "$bin" ]]; then
    echo "error: archive did not contain lpkg binary" >&2
    return 1
  fi
  chmod +x "$bin"
  local data_src
  data_src="$(find "$INSTALL_DIR" -type d -name data | head -1)"
  install_files "$bin" "${data_src:-}"
  return 0
}

install_from_source() {
  need_cmd cargo
  need_cmd git
  local src="$INSTALL_DIR/src"
  echo "Building lpkg from source..."
  git clone --depth 1 "https://github.com/${REPO}.git" "$src"
  (cd "$src" && cargo build --release)
  install_files "$src/target/release/lpkg" "$src/data"
}

main() {
  if [[ "$(uname -s)" != "Linux" ]]; then
    echo "error: lpkg currently supports Linux only" >&2
    exit 1
  fi

  if install_from_release; then
    exit 0
  fi

  echo "Release binary unavailable; falling back to source build..."
  install_from_source
}

main "$@"
