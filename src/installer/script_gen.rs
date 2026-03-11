use std::path::PathBuf;
use crate::installer::Installer;

impl Installer {
    pub async fn generate_portable_script_ext(
        &self,
        exe_path: &str,
        mangohud: bool,
        gamemode: bool,
        no_dxvk: bool,
        proton_path: Option<PathBuf>,
        gpu_vendor: &str,
        preset_dll_overrides: Option<String>,
        preset_env_vars: Option<&'static [(&'static str, &'static str)]>,
    ) -> std::io::Result<()> {
        let script_path = self._install_dir.join("play.sh");
        let auto_script_path = self._install_dir.join("play_auto.sh");
        let safe_script_path = self._install_dir.join("play_safe.sh");
        let brand_icon_path = self._install_dir.join("r2l_brand.svg");
        let icon_png_path = self._install_dir.join("icon.png");
        let desktop_helper_path = self._install_dir.join("adddesktopicon.sh");
        let exe_rel = exe_path.trim_start_matches("./").replace('"', "\\\"");

        let mut script_content = String::from("#!/bin/bash\n");
        script_content.push_str("# Font: Noto Sans Mono (mirrors the GUI)\n");
        script_content.push_str("# Theme: blue / gray / red for status prompts\n\n");
        script_content.push_str("SCRIPT_DIR=\"$(cd \"$(dirname \"$0\")\" && pwd)\"\n");
        script_content.push_str("export WINEPREFIX=\"$SCRIPT_DIR/pfx\"\n");
        script_content.push_str("if [ -d \"$SCRIPT_DIR/r2p_userdata/Local\" ]; then\n");
        script_content.push_str("  for u in \"$WINEPREFIX/drive_c/users\"/*; do\n");
        script_content.push_str("    [ -d \"$u\" ] || continue\n");
        script_content.push_str("    base=\"$(basename \"$u\")\"\n");
        script_content.push_str("    if [ \"$base\" = \"Public\" ] || [ \"$base\" = \"public\" ] || [ \"$base\" = \"Default\" ] || [ \"$base\" = \"default\" ]; then\n");
        script_content.push_str("      continue\n");
        script_content.push_str("    fi\n");
        script_content.push_str("    local_dir=\"$u/AppData/Local\"\n");
        script_content.push_str("    if [ -L \"$local_dir\" ] && [ ! -e \"$local_dir\" ]; then\n");
        script_content.push_str("      rm -f \"$local_dir\"\n");
        script_content.push_str("      ln -s \"$SCRIPT_DIR/r2p_userdata/Local\" \"$local_dir\"\n");
        script_content.push_str("    fi\n");
        script_content.push_str("  done\n");
        script_content.push_str("fi\n");
        script_content.push_str("for hive in system.reg user.reg userdef.reg; do\n");
        script_content.push_str("  if [ ! -s \"$WINEPREFIX/$hive\" ]; then\n");
        script_content.push_str("    echo \"[R2L] Missing prefix hive: $WINEPREFIX/$hive\"\n");
        script_content.push_str("    echo \"[R2L] This package was exported without a valid prefix. Re-export with scope: Full or Prefix/Libs.\"\n");
        script_content.push_str("    exit 1\n");
        script_content.push_str("  fi\n");
        script_content.push_str("done\n");
        script_content.push_str("MISSING_DEPS=()\n");
        script_content.push_str("for dep in wine wineserver; do\n");
        script_content.push_str("  if ! command -v \"$dep\" >/dev/null 2>&1; then\n");
        script_content.push_str("    MISSING_DEPS+=(\"$dep\")\n");
        script_content.push_str("  fi\n");
        script_content.push_str("done\n");
        script_content.push_str("if [ ${#MISSING_DEPS[@]} -gt 0 ]; then\n");
        script_content
            .push_str("  echo \"[R2L] Missing runtime dependencies: ${MISSING_DEPS[*]}\"\n");
        script_content.push_str("  echo \"[R2L] Install them first, e.g. on Debian/Ubuntu: sudo apt install wine64 wine32\"\n");
        script_content.push_str("  exit 1\n");
        script_content.push_str("fi\n");
        script_content.push_str("if command -v glxinfo >/dev/null 2>&1; then\n");
        script_content
            .push_str("  if ! glxinfo 2>/dev/null | grep -qi \"direct rendering: yes\"; then\n");
        script_content.push_str(
            "    echo \"[R2L] OpenGL acceleration is not available (direct rendering: No).\"\n",
        );
        script_content.push_str("    echo \"[R2L] Fix GPU drivers/32-bit graphics stack first (mesa/lib32 or vendor drivers).\"\n");
        script_content.push_str("    exit 1\n");
        script_content.push_str("  fi\n");
        script_content.push_str("else\n");
        script_content
            .push_str("  echo \"[R2L] glxinfo not found; cannot verify OpenGL acceleration.\"\n");
        script_content.push_str("  echo \"[R2L] Recommended: install mesa-utils (Debian/Ubuntu: sudo apt install mesa-utils).\"\n");
        script_content.push_str("fi\n");
        script_content.push_str("export WINEDEBUG=-all\n");
        script_content.push_str("export R2L_RENDERER=\"dxvk\"\n");
        script_content.push_str("for arg in \"$@\"; do\n");
        script_content
            .push_str("  if [ \"$arg\" = \"--safe\" ] || [ \"$arg\" = \"--wined3d\" ]; then\n");
        script_content.push_str("    R2L_RENDERER=\"wined3d\"\n");
        script_content.push_str("  fi\n");
        script_content.push_str("done\n");
        script_content.push_str("if [ \"$R2L_RENDERER\" = \"dxvk\" ]; then\n");
        script_content.push_str("  if command -v vulkaninfo >/dev/null 2>&1; then\n");
        script_content.push_str("    if ! vulkaninfo --summary >/dev/null 2>&1; then\n");
        script_content.push_str(
            "      echo \"[R2L] Vulkan check failed; switching to Safe Mode (WineD3D).\"\n",
        );
        script_content.push_str("      R2L_RENDERER=\"wined3d\"\n");
        script_content.push_str("    fi\n");
        script_content.push_str("  else\n");
        script_content
            .push_str("    if ! ldconfig -p 2>/dev/null | grep -q \"libvulkan.so.1\"; then\n");
        script_content.push_str("      echo \"[R2L] Vulkan runtime library not found; switching to Safe Mode (WineD3D).\"\n");
        script_content.push_str("      R2L_RENDERER=\"wined3d\"\n");
        script_content.push_str("    else\n");
        script_content.push_str("      echo \"[R2L] vulkaninfo not found; continuing with DXVK (install vulkan-tools for diagnostics).\"\n");
        script_content.push_str("    fi\n");
        script_content.push_str("  fi\n");
        script_content.push_str("fi\n");
        if no_dxvk {
            script_content.push_str("R2L_RENDERER=\"wined3d\"\n");
        }
        script_content.push_str("if [ \"$R2L_RENDERER\" = \"wined3d\" ]; then\n");
        script_content.push_str("  export PROTON_USE_WINED3D=1\n");
        script_content.push_str("  unset DXVK_ASYNC\n");
        script_content.push_str("  unset DXVK_CONFIG\n");
        script_content.push_str("fi\n");

        if let Some(envs) = preset_env_vars {
            for (key, val) in envs {
                script_content.push_str(&format!("export {}={}\n", key, val));
            }
        }
        let _ = gpu_vendor;
        if let Some(overrides) = preset_dll_overrides {
            script_content.push_str(&format!(
                "export WINEDLLOVERRIDES=\"$WINEDLLOVERRIDES;{}\"\n",
                overrides
            ));
        }

        script_content.push_str("cd \"$SCRIPT_DIR\" || exit 1\n");
        script_content.push_str("\n# --- ENVIRONMENT DETECTION ---\n");
        script_content.push_str("LAUNCH_ARGS=(\"$@\")\n");
        script_content.push_str("FILTERED_ARGS=()\n");
        script_content.push_str("for arg in \"${LAUNCH_ARGS[@]}\"; do\n");
        script_content
            .push_str("  if [ \"$arg\" = \"--safe\" ] || [ \"$arg\" = \"--wined3d\" ]; then\n");
        script_content.push_str("    continue\n");
        script_content.push_str("  fi\n");
        script_content.push_str("  FILTERED_ARGS+=(\"$arg\")\n");
        script_content.push_str("done\n");
        script_content.push_str(&format!("GAME_EXE=\"$SCRIPT_DIR/{}\"\n", exe_rel));

        script_content.push_str("R2L_PREFIX=()\n");
        if gamemode {
            script_content.push_str("if command -v gamemoderun >/dev/null 2>&1; then\n");
            script_content.push_str("  R2L_PREFIX+=(\"gamemoderun\")\n");
            script_content.push_str("else\n");
            script_content
                .push_str("  echo \"[R2L] gamemoderun missing; continuing without GameMode.\"\n");
            script_content.push_str("fi\n");
        }
        if mangohud {
            script_content.push_str("if command -v mangohud >/dev/null 2>&1; then\n");
            script_content.push_str("  R2L_PREFIX+=(\"mangohud\")\n");
            script_content.push_str("else\n");
            script_content
                .push_str("  echo \"[R2L] mangohud missing; continuing without MangoHUD.\"\n");
            script_content.push_str("fi\n");
        }

        script_content.push_str("run_bundled() {\n");
        script_content.push_str("    export PROTONPATH=\"$1\"\n");
        script_content.push_str("    export PATH=\"$PROTONPATH/bin:$PATH\"\n");
        script_content.push_str("    export LD_LIBRARY_PATH=\"$PROTONPATH/lib64:$PROTONPATH/lib:$LD_LIBRARY_PATH\"\n");
        script_content.push_str("    export WINELOADER=\"$PROTONPATH/bin/wine\"\n");
        script_content.push_str("    export WINESERVER=\"$PROTONPATH/bin/wineserver\"\n");
        script_content.push_str("    echo \"[R2L] Using bundled runtime at: $PROTONPATH\"\n");
        script_content.push_str("    if [ -f \"$PROTONPATH/proton\" ]; then\n");
        script_content.push_str("        exec \"${R2L_PREFIX[@]}\" \"$PROTONPATH/proton\" run \"$GAME_EXE\" \"${FILTERED_ARGS[@]}\"\n");
        script_content.push_str("    else\n");
        script_content.push_str("        exec \"${R2L_PREFIX[@]}\" \"$WINELOADER\" \"$GAME_EXE\" \"${FILTERED_ARGS[@]}\"\n");
        script_content.push_str("    fi\n");
        script_content.push_str("}\n\n");

        script_content.push_str("# --- RUNTIME DETECTION ---\n");
        script_content.push_str("if [ -d \"$SCRIPT_DIR/wine\" ]; then\n");
        script_content.push_str("    run_bundled \"$SCRIPT_DIR/wine\"\n");
        script_content.push_str("elif [ -d \"$SCRIPT_DIR/../wine\" ]; then\n");
        script_content.push_str("    run_bundled \"$SCRIPT_DIR/../wine\"\n");
        script_content.push_str("fi\n");

        if let Some(p) = proton_path {
            script_content.push_str(&format!("export CUSTOM_PROTON=\"{}\"\n", p.display()));
            script_content.push_str("if [ -d \"$CUSTOM_PROTON\" ]; then exec \"$CUSTOM_PROTON/proton\" run \"$GAME_EXE\" \"${FILTERED_ARGS[@]}\"\n");
            script_content.push_str(
                "else exec \"${R2L_PREFIX[@]}\" wine \"$GAME_EXE\" \"${FILTERED_ARGS[@]}\"\nfi\n",
            );
        } else {
            script_content
                .push_str("exec \"${R2L_PREFIX[@]}\" wine \"$GAME_EXE\" \"${FILTERED_ARGS[@]}\"\n");
        }

        tokio::fs::write(&script_path, &script_content).await?;
        let auto_script = "#!/bin/bash\nSCRIPT_DIR=\"$(cd \"$(dirname \"$0\")\" && pwd)\"\n\"$SCRIPT_DIR/play.sh\" \"$@\"\nSTATUS=$?\nif [ $STATUS -ne 0 ]; then\n  echo \"[R2L] Auto-fallback: retrying with Safe Mode (WineD3D)...\"\n  exec \"$SCRIPT_DIR/play.sh\" --safe \"$@\"\nfi\nexit $STATUS\n";
        tokio::fs::write(&auto_script_path, auto_script).await?;
        let safe_script = "#!/bin/bash\nSCRIPT_DIR=\"$(cd \"$(dirname \"$0\")\" && pwd)\"\nexec \"$SCRIPT_DIR/play.sh\" --safe \"$@\"\n";
        tokio::fs::write(&safe_script_path, safe_script).await?;
        if !icon_png_path.exists() {
            tokio::fs::write(&brand_icon_path, Self::r2p_icon_svg()).await?;
        }
        tokio::fs::write(
            &desktop_helper_path,
            Self::desktop_icon_helper_script(&self._game_name),
        )
        .await?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = tokio::fs::metadata(&script_path).await?.permissions();
            perms.set_mode(0o755);
            tokio::fs::set_permissions(&script_path, perms).await?;
            let mut auto_perms = tokio::fs::metadata(&auto_script_path).await?.permissions();
            auto_perms.set_mode(0o755);
            tokio::fs::set_permissions(&auto_script_path, auto_perms).await?;
            let mut safe_perms = tokio::fs::metadata(&safe_script_path).await?.permissions();
            safe_perms.set_mode(0o755);
            tokio::fs::set_permissions(&safe_script_path, safe_perms).await?;
            let mut icon_perms = tokio::fs::metadata(&desktop_helper_path)
                .await?
                .permissions();
            icon_perms.set_mode(0o755);
            tokio::fs::set_permissions(&desktop_helper_path, icon_perms).await?;
        }
        Ok(())
    }

    pub fn r2p_icon_svg() -> &'static str {
        r###"<?xml version="1.0" encoding="UTF-8"?>
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
"###
    }

    pub fn desktop_icon_helper_script(game_name: &str) -> String {
        let escaped_game = game_name.replace('"', "\\\"");
        format!(
            r#"#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
GAME_NAME="{game_name}"
PLAY_SH="$SCRIPT_DIR/play.sh"
PLAY_AUTO_SH="$SCRIPT_DIR/play_auto.sh"
BRAND_SVG="$SCRIPT_DIR/r2l_brand.svg"
ICON_PNG="$SCRIPT_DIR/icon.png"
ICON_FILE="$BRAND_SVG"
EXEC_SH="$PLAY_SH"

if [ -f "$ICON_PNG" ]; then
  ICON_FILE="$ICON_PNG"
fi

if [ -f "$PLAY_AUTO_SH" ]; then
  EXEC_SH="$PLAY_AUTO_SH"
fi

if [ ! -f "$EXEC_SH" ]; then
  echo "Missing play.sh in $SCRIPT_DIR"
  exit 1
fi

sanitize_name() {{
  echo "$1" | tr '[:upper:]' '[:lower:]' | sed 's/[^a-z0-9]/_/g' | sed 's/__\+/_/g' | sed 's/^_//;s/_$//'
}}

SHORT_NAME="$(sanitize_name "$GAME_NAME")"
if [ -z "$SHORT_NAME" ]; then
  SHORT_NAME="r2p_game"
fi

DESKTOP_FILE_CONTENT="[Desktop Entry]
Version=1.0
Type=Application
Name=$GAME_NAME
Exec=\"$EXEC_SH\"
Path=$SCRIPT_DIR
Icon=$ICON_FILE
Terminal=false
Categories=Game;
StartupNotify=true
"

MODE="${{1:-both}}"
WRITE_DESKTOP=1
WRITE_MENU=1
if [ "$MODE" = "--desktop-only" ]; then
  WRITE_MENU=0
elif [ "$MODE" = "--menu-only" ]; then
  WRITE_DESKTOP=0
fi

DESKTOP_DIRS=()
if command -v xdg-user-dir >/dev/null 2>&1; then
  XDG_DESKTOP="$(xdg-user-dir DESKTOP 2>/dev/null || true)"
  if [ -n "$XDG_DESKTOP" ]; then
    DESKTOP_DIRS+=("$XDG_DESKTOP")
  fi
fi
DESKTOP_DIRS+=("$HOME/Desktop" "$HOME/Pulpit")

UNIQ_DESKTOP_DIRS=()
for d in "${{DESKTOP_DIRS[@]}}"; do
  skip=0
  for u in "${{UNIQ_DESKTOP_DIRS[@]}}"; do
    if [ "$u" = "$d" ]; then
      skip=1
      break
    fi
  done
  if [ $skip -eq 0 ]; then
    UNIQ_DESKTOP_DIRS+=("$d")
  fi
done

if [ $WRITE_DESKTOP -eq 1 ]; then
  for d in "${{UNIQ_DESKTOP_DIRS[@]}}"; do
    mkdir -p "$d"
    target="$d/$SHORT_NAME.desktop"
    printf "%s" "$DESKTOP_FILE_CONTENT" > "$target"
    chmod +x "$target"
  done
fi

if [ $WRITE_MENU -eq 1 ]; then
  MENU_DIR="$HOME/.local/share/applications"
  mkdir -p "$MENU_DIR"
  MENU_TARGET="$MENU_DIR/$SHORT_NAME.desktop"
  printf "%s" "$DESKTOP_FILE_CONTENT" > "$MENU_TARGET"
  chmod +x "$MENU_TARGET"
fi

echo "Desktop entries created for $GAME_NAME"
"#,
            game_name = escaped_game
        )
    }

    pub async fn generate_readme(&self) -> std::io::Result<()> {
        let readme_path = self._install_dir.join("README_LINUX.txt");
        let content = format!(
            "--- {} ---\nCreated with R2L ULTIMATE v6.8.",
            self._game_name
        );
        tokio::fs::write(&readme_path, content).await?;

        let auto_readme_path = self._install_dir.join("README_AUTO.txt");
        let auto_content = format!(
            "R2L AUTO INFO\n\
             =============\n\
             Game: {game}\n\n\
             This package was built with Repack2Linux (R2L).\n\
             Runtime mode: Isolated / Portable.\n\n\
             Save files and runtime data stay inside this package.\n\
             Main locations:\n\
             - Prefix: ./pfx\n\
             - User data: ./r2p_userdata\n\
             - Typical saves: ./r2p_userdata/Local\n\n\
             Launchers:\n\
             - ./play_auto.sh  (recommended)\n\
             - ./play.sh\n\
             - ./play_safe.sh\n\n\
             If you remove this game folder, no save data should remain in your home directory.\n",
            game = self._game_name
        );
        tokio::fs::write(&auto_readme_path, auto_content).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn generate_portable_script_ext_includes_font_annotation() {
        let temp_dir = tempdir().unwrap();
        let install_path = temp_dir.path().join("install");
        tokio::fs::create_dir_all(&install_path).await.unwrap();
        let installer = Installer::new("Test Game", install_path.clone());
        installer
            .generate_portable_script_ext(
                "game.exe", false, false, false, None, "GENERIC", None, None,
            )
            .await
            .unwrap();
        let script = install_path.join("play.sh");
        let content = tokio::fs::read_to_string(&script).await.unwrap();
        assert!(content.contains("# Font: Noto Sans Mono"));
    }
}
