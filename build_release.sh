#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT_DIR"

BIN_NAME="${1:-repack2proton-rs}"
APP_NAME="Repack2Linux"
HOST_TRIPLE="$(rustc -vV | awk '/host:/ {print $2}')"
OUT_DIR="$ROOT_DIR/dist"
STAGE_DIR="$OUT_DIR/${APP_NAME}-${HOST_TRIPLE}"
VERSION_TAG="${VERSION_TAG:-v1.01}"

echo "[R2L] Building release binary: $BIN_NAME"
cargo build --release --bin "$BIN_NAME"

rm -rf "$STAGE_DIR"
mkdir -p "$STAGE_DIR"

cp "target/release/$BIN_NAME" "$STAGE_DIR/$APP_NAME"
chmod +x "$STAGE_DIR/$APP_NAME"

cp README.md "$STAGE_DIR/README.md"
cp PROGRESS.md "$STAGE_DIR/PROGRESS.md"

if [[ -f "Repack2Proton/LICENSE" ]]; then
  cp "Repack2Proton/LICENSE" "$STAGE_DIR/LICENSE"
fi

ARCHIVE_BASENAME="${APP_NAME}-${VERSION_TAG}-${HOST_TRIPLE}"
ARCHIVE_PATH="$OUT_DIR/${ARCHIVE_BASENAME}.tar.gz"
SHA_PATH="$OUT_DIR/${ARCHIVE_BASENAME}.sha256"

tar -C "$OUT_DIR" -czf "$ARCHIVE_PATH" "${APP_NAME}-${HOST_TRIPLE}"
sha256sum "$ARCHIVE_PATH" | tee "$SHA_PATH"

echo "[R2L] Done"
echo "[R2L] Binary:  $STAGE_DIR/$APP_NAME"
echo "[R2L] Archive: $ARCHIVE_PATH"
echo "[R2L] SHA256:  $SHA_PATH"
echo "[R2L] Upload both files to GitHub Release assets."
