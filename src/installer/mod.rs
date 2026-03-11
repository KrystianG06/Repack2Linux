use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;
use chrono::Utc;
use serde::{Deserialize, Serialize};

mod sfx;
mod payload;
mod script_gen;

pub struct Installer {
    pub _game_name: String,
    pub _install_dir: PathBuf,
    pub prefix_path: PathBuf,
}

#[derive(Serialize, Deserialize)]
pub struct PrefixSelectionEntry {
    pub game: String,
    pub prefix_path: String,
    pub arch: String,
    pub score: usize,
    pub reason: String,
    pub selected_at: String,
}

impl Installer {
    pub fn new(game_name: &str, install_dir: PathBuf) -> Self {
        let prefix_path = install_dir.join("pfx");
        Self {
            _game_name: game_name.to_string(),
            _install_dir: install_dir,
            prefix_path,
        }
    }

    fn app_config_dir() -> PathBuf {
        let mut dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        dir.push("repack2linux");
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    fn legacy_config_dir() -> PathBuf {
        let mut dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        dir.push("repack2proton");
        dir
    }

    fn prefix_registry_file() -> PathBuf {
        let mut path = Self::app_config_dir();
        path.push("prefix-records.json");
        if !path.exists() {
            let mut legacy = Self::legacy_config_dir();
            legacy.push("prefix-records.json");
            if legacy.exists() {
                let _ = std::fs::copy(&legacy, &path);
            }
        }
        path
    }

    async fn load_prefix_registry_map() -> io::Result<HashMap<String, String>> {
        let path = Self::prefix_registry_file();
        if !path.exists() {
            return Ok(HashMap::new());
        }
        let data = tokio::fs::read(&path).await?;
        serde_json::from_slice(&data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub async fn prefix_record_for(game: &str) -> io::Result<Option<PathBuf>> {
        let map = Self::load_prefix_registry_map().await?;
        Ok(map.get(game).map(|value| PathBuf::from(value)))
    }

    pub async fn best_recorded_prefix(arch: &str) -> io::Result<Option<PathBuf>> {
        let map = Self::load_prefix_registry_map().await?;
        let mut scored_candidates = Vec::new();

        for value in map.values() {
            let candidate = PathBuf::from(value);
            if candidate.exists() && Self::valid_prefix(&candidate) {
                let score = Self::prefix_score(&candidate, arch);
                scored_candidates.push((score, candidate));
            }
        }

        scored_candidates.sort_by(|a, b| b.0.cmp(&a.0));
        Ok(scored_candidates.into_iter().next().map(|(_, path)| path))
    }

    pub async fn record_prefix(game: &str, prefix: &Path) -> io::Result<()> {
        let path = Self::prefix_registry_file();
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let mut map = Self::load_prefix_registry_map().await?;
        map.insert(game.to_string(), prefix.to_string_lossy().to_string());
        let serialized = serde_json::to_string_pretty(&map)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        tokio::fs::write(&path, serialized).await?;
        Ok(())
    }

    pub async fn duplicate_prefix(src: &Path, dest: &Path) -> io::Result<bool> {
        tokio::fs::create_dir_all(dest).await?;
        let status = tokio::process::Command::new("cp")
            .arg("-a")
            .arg("--reflink=auto")
            .arg(format!("{}/.", src.to_string_lossy()))
            .arg(dest)
            .status()
            .await?;
        Ok(status.success())
    }

    fn prefix_selection_log_file() -> PathBuf {
        let mut path = Self::prefix_registry_file();
        path.set_file_name("prefix-selection.json");
        path
    }

    async fn append_selection_log(entry: &PrefixSelectionEntry) -> io::Result<()> {
        let path = Self::prefix_selection_log_file();
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await?;
        let payload =
            serde_json::to_string(entry).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        file.write_all(payload.as_bytes()).await?;
        file.write_all(b"\n").await?;
        Ok(())
    }

    fn make_selection_entry(
        game: &str,
        prefix: &Path,
        arch: &str,
        score: usize,
        reason: &str,
    ) -> PrefixSelectionEntry {
        PrefixSelectionEntry {
            game: game.to_string(),
            prefix_path: prefix.to_string_lossy().to_string(),
            arch: arch.to_string(),
            score,
            reason: reason.to_string(),
            selected_at: Utc::now().to_rfc3339(),
        }
    }

    pub fn check_tool(name: &str) -> bool {
        std::process::Command::new("which")
            .arg(name)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    pub fn validate_unified_sfx_environment() -> Result<(), String> {
        if Self::resolve_installer_gui_binary().is_some() {
            return Ok(());
        }

        let cargo_toml = PathBuf::from("Cargo.toml");
        let gui_source = PathBuf::from("src/bin/installer_gui.rs");
        let running_from_appimage = std::env::var_os("APPIMAGE").is_some();

        if running_from_appimage || !cargo_toml.exists() || !gui_source.exists() {
            return Err("Eksport Unified SFX (.sh) jest niedostępny w buildzie runtime/AppImage. Użyj eksportu Portable lub uruchom wersję deweloperską z repozytorium.".to_string());
        }

        if !Self::check_tool("cargo") {
            return Err(
                "Eksport Unified SFX (.sh) wymaga Cargo (rustup) do kompilacji modułu installer_gui."
                    .to_string(),
            );
        }

        Ok(())
    }

    fn resolve_installer_gui_binary() -> Option<PathBuf> {
        if let Ok(custom) = std::env::var("R2L_INSTALLER_GUI_BIN") {
            let p = PathBuf::from(custom);
            if p.exists() {
                return Some(p);
            }
        }

        let local_target = PathBuf::from("target/release/installer_gui");
        if local_target.exists() {
            return Some(local_target);
        }

        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let alongside = dir.join("installer_gui");
                if alongside.exists() {
                    return Some(alongside);
                }
                let hidden = dir.join(".r2l").join("installer_gui");
                if hidden.exists() {
                    return Some(hidden);
                }
            }
        }

        None
    }
}
