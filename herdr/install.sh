#!/usr/bin/env bash
set -euo pipefail

NAME="herdr-tab-smart-rename-rs"
REPO="EmmetZ/herdr-tab-smart-rename-rs"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN_DIR="$ROOT/bin"

VERSION="$(sed -nE 's/^version[[:space:]]*=[[:space:]]*"([^"]+)".*/\1/p' "$ROOT/herdr-plugin.toml" | head -n1)"
if [ -z "$VERSION" ]; then
  echo "$NAME: cannot read version from herdr-plugin.toml" >&2
  exit 1
fi

case "$(uname -s)-$(uname -m)" in
  Darwin-arm64) target="aarch64-apple-darwin" ;;
  Darwin-x86_64) target="x86_64-apple-darwin" ;;
  Linux-aarch64 | Linux-arm64) target="aarch64-unknown-linux-musl" ;;
  Linux-x86_64) target="x86_64-unknown-linux-musl" ;;
  *)
    echo "$NAME: no prebuilt binary for $(uname -s)-$(uname -m) — build locally with 'cargo build --release'." >&2
    exit 1
    ;;
esac

TAG="v$VERSION"
ARCHIVE="$NAME-$target.tar.gz"
CHECKSUM="$NAME-$target.sha256"
BASE_URL="https://github.com/$REPO/releases/download/$TAG"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

download() {
  url="$1"
  out="$2"
  attempt=1

  while :; do
    if curl -fsSL "$url" -o "$out"; then
      return
    fi

    if [ "$attempt" -ge 5 ]; then
      return 1
    fi

    attempt=$((attempt + 1))
    sleep 3
  done
}

echo "$NAME: downloading $ARCHIVE from $REPO@$TAG"
download "$BASE_URL/$ARCHIVE" "$TMP_DIR/$ARCHIVE"
download "$BASE_URL/$CHECKSUM" "$TMP_DIR/$CHECKSUM"

expected="$(awk '{print $1}' "$TMP_DIR/$CHECKSUM")"
if command -v sha256sum >/dev/null 2>&1; then
  actual="$(sha256sum "$TMP_DIR/$ARCHIVE" | awk '{print $1}')"
else
  actual="$(shasum -a 256 "$TMP_DIR/$ARCHIVE" | awk '{print $1}')"
fi

if [ "$expected" != "$actual" ]; then
  echo "$NAME: checksum mismatch (expected $expected, got $actual)" >&2
  exit 1
fi

mkdir -p "$BIN_DIR"
tar -xzf "$TMP_DIR/$ARCHIVE" -C "$TMP_DIR"
install -m 0755 "$TMP_DIR/$NAME" "$BIN_DIR/$NAME"

echo "$NAME: installed $BIN_DIR/$NAME"
