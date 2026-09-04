#!/usr/bin/env bash
# Build an amd64 .deb for lpkg from the current release binary.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION="$(grep -m1 '^version' "$ROOT/Cargo.toml" | cut -d'"' -f2)"
ARCH="${ARCH:-amd64}"
PKG_NAME="lpkg_${VERSION}_${ARCH}"
STAGE="$ROOT/dist/${PKG_NAME}"

rm -rf "$STAGE"
mkdir -p "$STAGE/DEBIAN" \
  "$STAGE/usr/bin" \
  "$STAGE/usr/share/lpkg" \
  "$STAGE/usr/share/doc/lpkg"

if [[ ! -x "$ROOT/target/release/lpkg" ]]; then
  echo "Building release binary..."
  (cd "$ROOT" && cargo build --release)
fi

install -m 755 "$ROOT/target/release/lpkg" "$STAGE/usr/bin/lpkg"
install -m 644 "$ROOT/data/aliases.toml" "$STAGE/usr/share/lpkg/aliases.toml"
install -m 644 "$ROOT/data/direct-deb-index.json" "$STAGE/usr/share/lpkg/direct-deb-index.json"
install -m 644 "$ROOT/README.md" "$STAGE/usr/share/doc/lpkg/README.md"

# Adapt control for this build
sed -e "s/^Version:.*/Version: ${VERSION}/" \
    -e "s/^Architecture:.*/Architecture: ${ARCH}/" \
    "$ROOT/packaging/deb/control" > "$STAGE/DEBIAN/control"

# Installed size in KiB
SIZE_KB="$(du -sk "$STAGE/usr" | cut -f1)"
echo "Installed-Size: ${SIZE_KB}" >> "$STAGE/DEBIAN/control"

mkdir -p "$ROOT/dist"
dpkg-deb --build --root-owner-group "$STAGE" "$ROOT/dist/${PKG_NAME}.deb"
echo "Built $ROOT/dist/${PKG_NAME}.deb"
