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
TOOLS_DIR="$ROOT_DIR/.tools"

APPIMAGETOOL_BIN=""
if command -v appimagetool >/dev/null 2>&1; then
  APPIMAGETOOL_BIN="$(command -v appimagetool)"
else
  mkdir -p "$TOOLS_DIR"
  case "$ARCH" in
    x86_64) APPIMAGETOOL_URL="https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage" ;;
    aarch64) APPIMAGETOOL_URL="https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-aarch64.AppImage" ;;
    *)
      echo "[R2L] Unsupported arch for auto-download: $ARCH"
      echo "[R2L] Install appimagetool manually and rerun."
      exit 1
      ;;
  esac
  APPIMAGETOOL_BIN="$TOOLS_DIR/appimagetool-$ARCH.AppImage"
  if [[ ! -x "$APPIMAGETOOL_BIN" ]]; then
    echo "[R2L] Downloading appimagetool for $ARCH..."
    curl -fsSL "$APPIMAGETOOL_URL" -o "$APPIMAGETOOL_BIN"
    chmod +x "$APPIMAGETOOL_BIN"
  fi
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

if [[ "$APPIMAGETOOL_BIN" == *.AppImage ]]; then
  ARCH="$ARCH" APPIMAGE_EXTRACT_AND_RUN=1 "$APPIMAGETOOL_BIN" "$APPDIR" "$APPIMAGE_PATH"
else
  ARCH="$ARCH" "$APPIMAGETOOL_BIN" "$APPDIR" "$APPIMAGE_PATH"
fi
chmod +x "$APPIMAGE_PATH"
sha256sum "$APPIMAGE_PATH" | tee "$SHA_PATH"

echo "[R2L] Done"
echo "[R2L] AppImage: $APPIMAGE_PATH"
echo "[R2L] SHA256:   $SHA_PATH"
