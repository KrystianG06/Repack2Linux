#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT_DIR"

APP_NAME="Repack2Linux"
BIN_NAME="${1:-repack2proton-rs}"
VERSION_TAG="${VERSION_TAG:-v1.01}"
ARCH="$(uname -m)"
OUT_DIR="$ROOT_DIR/dist"
APPDIR="$OUT_DIR/${APP_NAME}.AppDir"

if ! command -v appimagetool >/dev/null 2>&1; then
  echo "[R2L] Missing dependency: appimagetool"
  echo "[R2L] Install and re-run:"
  echo "  sudo apt install -y appimagetool"
  exit 1
fi

echo "[R2L] Building release binary: $BIN_NAME"
cargo build --release --bin "$BIN_NAME"

rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/share/applications" "$APPDIR/usr/share/icons/hicolor/scalable/apps"

cp "target/release/$BIN_NAME" "$APPDIR/usr/bin/$APP_NAME"
chmod +x "$APPDIR/usr/bin/$APP_NAME"

cat > "$APPDIR/repack2linux.svg" << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256" viewBox="0 0 256 256">
  <defs>
    <linearGradient id="g" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0%" stop-color="#0e5fae"/>
      <stop offset="60%" stop-color="#2c6dff"/>
      <stop offset="100%" stop-color="#ff4f58"/>
    </linearGradient>
  </defs>
  <rect width="256" height="256" rx="48" ry="48" fill="#05060f"/>
  <circle cx="128" cy="128" r="78" fill="url(#g)"/>
</svg>
EOF

cp "$APPDIR/repack2linux.svg" "$APPDIR/usr/share/icons/hicolor/scalable/apps/repack2linux.svg"

cat > "$APPDIR/repack2linux.desktop" << EOF
[Desktop Entry]
Type=Application
Name=Repack2Linux
Comment=Portable Windows game repack factory for Linux
Exec=Repack2Linux
Icon=repack2linux
Terminal=false
Categories=Game;Utility;
StartupWMClass=repack2linux
X-AppImage-Name=Repack2Linux
X-AppImage-Version=${VERSION_TAG}
EOF

cp "$APPDIR/repack2linux.desktop" "$APPDIR/usr/share/applications/repack2linux.desktop"

cat > "$APPDIR/AppRun" << 'EOF'
#!/usr/bin/env bash
HERE="$(cd "$(dirname "$0")" && pwd)"
exec "$HERE/usr/bin/Repack2Linux" "$@"
EOF
chmod +x "$APPDIR/AppRun"

APPIMAGE_NAME="${APP_NAME}-${VERSION_TAG}-${ARCH}.AppImage"
APPIMAGE_PATH="$OUT_DIR/$APPIMAGE_NAME"
SHA_PATH="$OUT_DIR/${APPIMAGE_NAME}.sha256"

ARCH="$ARCH" appimagetool "$APPDIR" "$APPIMAGE_PATH"
chmod +x "$APPIMAGE_PATH"
sha256sum "$APPIMAGE_PATH" | tee "$SHA_PATH"

echo "[R2L] Done"
echo "[R2L] AppImage: $APPIMAGE_PATH"
echo "[R2L] SHA256:   $SHA_PATH"
