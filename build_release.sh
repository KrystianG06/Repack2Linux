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

echo "[R2L] Building release binaries: $BIN_NAME + installer_gui"
cargo build --release --bin "$BIN_NAME"
cargo build --release --bin installer_gui

rm -rf "$STAGE_DIR"
mkdir -p "$STAGE_DIR"

cp "target/release/$BIN_NAME" "$STAGE_DIR/$APP_NAME"
chmod +x "$STAGE_DIR/$APP_NAME"
cp "target/release/installer_gui" "$STAGE_DIR/installer_gui"
chmod +x "$STAGE_DIR/installer_gui"

cp README.md "$STAGE_DIR/README.md"
cp PROGRESS.md "$STAGE_DIR/PROGRESS.md"

if [[ -f "Repack2Proton/LICENSE" ]]; then
  cp "Repack2Proton/LICENSE" "$STAGE_DIR/LICENSE"
fi

cat > "$STAGE_DIR/repack2linux.svg" << 'EOF'
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

cat > "$STAGE_DIR/install_desktop_icon.sh" << 'EOF'
#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
HOME_DIR="${HOME:-$SCRIPT_DIR}"
ICON_DIR="$HOME_DIR/.local/share/icons/hicolor/scalable/apps"
APP_DIR="$HOME_DIR/.local/share/applications"
ICON_PATH="$ICON_DIR/repack2linux.svg"
DESKTOP_PATH="$APP_DIR/repack2linux.desktop"

mkdir -p "$ICON_DIR" "$APP_DIR"
cp "$SCRIPT_DIR/repack2linux.svg" "$ICON_PATH"

cat > "$DESKTOP_PATH" <<DESKTOP
[Desktop Entry]
Version=1.0
Type=Application
Name=Repack2Linux
Exec=$SCRIPT_DIR/Repack2Linux
Icon=$ICON_PATH
Terminal=false
Categories=Game;Utility;
StartupNotify=true
DESKTOP

chmod +x "$DESKTOP_PATH"
update-desktop-database "$APP_DIR" >/dev/null 2>&1 || true
gtk-update-icon-cache "$HOME_DIR/.local/share/icons/hicolor" >/dev/null 2>&1 || true

echo "[R2L] Desktop entry installed:"
echo "  $DESKTOP_PATH"
echo "[R2L] If icon is not visible immediately, relogin or run:"
echo "  killall -SIGUSR1 gnome-shell 2>/dev/null || true"
EOF
chmod +x "$STAGE_DIR/install_desktop_icon.sh"

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
