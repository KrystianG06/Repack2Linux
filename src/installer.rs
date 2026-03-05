use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;

use chrono::Utc;
use dirs;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json;

use crate::export::{ExportArtifact, ExportAudit, ExportScope};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

pub struct Installer {
    pub _game_name: String,
    pub _install_dir: PathBuf,
    pub prefix_path: PathBuf,
}

#[derive(Serialize, Deserialize)]
struct PrefixSelectionEntry {
    game: String,
    prefix_path: String,
    arch: String,
    score: usize,
    reason: String,
    selected_at: String,
}

enum TarSelection {
    All,
    Exclude(&'static [&'static str]),
    Only(&'static [&'static str]),
}

impl Installer {
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

    pub fn new(game_name: &str, install_dir: PathBuf) -> Self {
        let prefix_path = install_dir.join("pfx");
        Self {
            _game_name: game_name.to_string(),
            _install_dir: install_dir,
            prefix_path,
        }
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

    async fn prefix_record_for(game: &str) -> io::Result<Option<PathBuf>> {
        let map = Self::load_prefix_registry_map().await?;
        Ok(map.get(game).map(|value| PathBuf::from(value)))
    }

    async fn best_recorded_prefix(arch: &str) -> io::Result<Option<PathBuf>> {
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

    async fn record_prefix(game: &str, prefix: &Path) -> io::Result<()> {
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

    async fn duplicate_prefix(src: &Path, dest: &Path) -> io::Result<bool> {
        tokio::fs::create_dir_all(dest).await?;
        let status = Command::new("cp")
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
        let gui_bin_path = PathBuf::from("target/release/installer_gui");
        if gui_bin_path.exists() {
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

    fn scope_subset(scope: ExportScope) -> Option<&'static [&'static str]> {
        match scope {
            ExportScope::Full => None,
            ExportScope::PrefixOnly => Some(&["pfx"]),
            ExportScope::GameOnly => Some(&[
                "play.sh",
                "play_auto.sh",
                "play_safe.sh",
                "adddesktopicon.sh",
                "README_LINUX.txt",
                "README_AUTO.txt",
            ]),
            ExportScope::LibsOnly => Some(&[
                "pfx",
                "play.sh",
                "play_auto.sh",
                "play_safe.sh",
                "adddesktopicon.sh",
                "README_LINUX.txt",
                "README_AUTO.txt",
                "r2p-icon.svg",
            ]),
        }
    }

    pub(crate) async fn collect_export_audits(
        portable_dir: &Path,
        scope: ExportScope,
        dry_run: bool,
        proton_path: Option<&Path>,
    ) -> io::Result<Vec<ExportAudit>> {
        Self::ensure_prefix_hives(portable_dir).await?;
        let subset_hint = Self::scope_subset(scope);

        if dry_run {
            let subset =
                subset_hint.unwrap_or(&["pfx", "play.sh", "README_LINUX.txt", "README_AUTO.txt"]);
            let subset_hash = Self::hash_subset_entries(portable_dir, subset).await?;
            let mut audits = vec![ExportAudit::new("Dry run: prefix i skrypt", subset_hash)];
            if let Some(proton) = proton_path {
                let proton_hash = Self::hash_directory(proton).await?;
                audits.push(ExportAudit::new("Bundled Proton", proton_hash));
            }
            return Ok(audits);
        }

        let scope_label = match scope {
            ExportScope::Full => "Pełna paczka",
            ExportScope::PrefixOnly => "Prefix",
            ExportScope::GameOnly => "Pliki gry",
            ExportScope::LibsOnly => "Biblioteki i prefix",
        };
        let scope_hash = if let Some(entries) = subset_hint {
            Self::hash_subset_entries(portable_dir, entries).await?
        } else {
            Self::hash_directory(portable_dir).await?
        };

        let mut audits = vec![ExportAudit::new(scope_label, scope_hash)];
        if let Some(proton) = proton_path {
            let proton_hash = Self::hash_directory(proton).await?;
            audits.push(ExportAudit::new("Bundled Proton", proton_hash));
        }
        Ok(audits)
    }

    pub async fn prepare_prefix_ext(&self, use_win32: bool) -> std::io::Result<Option<PathBuf>> {
        if self.prefix_path.exists() {
            return Ok(None);
        }
        let arch = if use_win32 { "win32" } else { "win64" };
        if let Some(recorded) = Self::prefix_record_for(&self._game_name).await? {
            if recorded.exists() && Self::valid_prefix(&recorded) {
                if Self::duplicate_prefix(&recorded, &self.prefix_path).await? {
                    Self::record_prefix(&self._game_name, &recorded).await?;
                    let score = Self::prefix_score(&recorded, arch);
                    let entry = Self::make_selection_entry(
                        &self._game_name,
                        &recorded,
                        arch,
                        score,
                        "history",
                    );
                    Self::append_selection_log(&entry).await?;
                    return Ok(Some(recorded));
                }
            }
        }
        let home = std::env::var("HOME").unwrap_or_default();
        let base_pfx_path = PathBuf::from(&home).join(format!("Games/R2L/base_pfx_{}", arch));

        if base_pfx_path.exists() {
            tokio::fs::create_dir_all(&self.prefix_path).await?;
            let status = Command::new("cp")
                .arg("-a")
                .arg("--reflink=auto")
                .arg(format!("{}/.", base_pfx_path.to_string_lossy()))
                .arg(&self.prefix_path)
                .status()
                .await?;

            if status.success() {
                self.sanitize_prefix().await?;
                Self::record_prefix(&self._game_name, &base_pfx_path).await?;
                let score = Self::prefix_score(&base_pfx_path, arch);
                let entry = Self::make_selection_entry(
                    &self._game_name,
                    &base_pfx_path,
                    arch,
                    score,
                    "base",
                );
                Self::append_selection_log(&entry).await?;
                return Ok(Some(base_pfx_path));
            }
            // Jeśli cp zawiedzie, próbujemy zbudować od nowa
        }

        if let Some(candidate) = Self::search_existing_base_prefix(&home, arch) {
            tokio::fs::create_dir_all(&self.prefix_path).await?;
            let status = Command::new("cp")
                .arg("-a")
                .arg("--reflink=auto")
                .arg(format!("{}/.", candidate.to_string_lossy()))
                .arg(&self.prefix_path)
                .status()
                .await?;

            if status.success() {
                self.sanitize_prefix().await?;
                Self::record_prefix(&self._game_name, &candidate).await?;
                let score = Self::prefix_score(&candidate, arch);
                let entry = Self::make_selection_entry(
                    &self._game_name,
                    &candidate,
                    arch,
                    score,
                    "candidate",
                );
                Self::append_selection_log(&entry).await?;
                return Ok(Some(candidate));
            }
        }

        if let Some(recorded_prefix) = Self::best_recorded_prefix(arch).await? {
            tokio::fs::create_dir_all(&self.prefix_path).await?;
            let status = Command::new("cp")
                .arg("-a")
                .arg("--reflink=auto")
                .arg(format!("{}/.", recorded_prefix.to_string_lossy()))
                .arg(&self.prefix_path)
                .status()
                .await?;

            if status.success() {
                self.sanitize_prefix().await?;
                Self::record_prefix(&self._game_name, &recorded_prefix).await?;
                let score = Self::prefix_score(&recorded_prefix, arch);
                let entry = Self::make_selection_entry(
                    &self._game_name,
                    &recorded_prefix,
                    arch,
                    score,
                    "recorded-fallback",
                );
                Self::append_selection_log(&entry).await?;
                return Ok(Some(recorded_prefix));
            }
        }

        tokio::fs::create_dir_all(&self.prefix_path).await?;

        // KRYTYCZNE: wineboot
        let status = Command::new("wine")
            .arg("wineboot")
            .arg("-i")
            .env("WINEPREFIX", self.prefix_path.to_string_lossy().to_string())
            .env("WINEARCH", arch)
            .status()
            .await?;

        if !status.success() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "wineboot failed - prefix corrupted",
            ));
        }

        // Czekamy na wineserver
        let _ = Command::new("wineserver")
            .arg("-w")
            .env("WINEPREFIX", self.prefix_path.to_string_lossy().to_string())
            .status()
            .await;

        // Rejestr - DllOverrides
        for dll in &["mscoree", "mshtml"] {
            let _ = Command::new("wine")
                .arg("reg")
                .arg("add")
                .arg("HKEY_CURRENT_USER\\Software\\Wine\\DllOverrides")
                .arg("/v")
                .arg(dll)
                .arg("/t")
                .arg("REG_SZ")
                .arg("/d")
                .arg("")
                .arg("/f")
                .env("WINEPREFIX", self.prefix_path.to_string_lossy().to_string())
                .status()
                .await;
        }

        // SANITYZACJA (Anonimizacja i naprawa ścieżek + MOUSE FIX)
        self.sanitize_prefix().await?;

        // Zapisujemy jako bazę dla przyszłych projektów
        let _ = tokio::fs::create_dir_all(&base_pfx_path).await;
        let _ = Command::new("cp")
            .arg("-a")
            .arg("--reflink=auto")
            .arg(format!("{}/.", self.prefix_path.to_string_lossy()))
            .arg(&base_pfx_path)
            .status()
            .await;

        Self::record_prefix(&self._game_name, &self.prefix_path).await?;
        let score = Self::prefix_score(&self.prefix_path, arch);
        let entry =
            Self::make_selection_entry(&self._game_name, &self.prefix_path, arch, score, "created");
        Self::append_selection_log(&entry).await?;

        Ok(None)
    }

    async fn sanitize_prefix(&self) -> std::io::Result<()> {
        let users_dir = self.prefix_path.join("drive_c/users");
        if let Ok(mut entries) = tokio::fs::read_dir(&users_dir).await {
            while let Some(entry) = entries.next_entry().await? {
                if entry.file_type().await?.is_dir() {
                    let user_path = entry.path();
                    let folders = [
                        "Desktop",
                        "Documents",
                        "Downloads",
                        "Music",
                        "Pictures",
                        "Videos",
                        "Saved Games",
                    ];
                    for folder in folders {
                        let target = user_path.join(folder);
                        if let Ok(meta) = tokio::fs::symlink_metadata(&target).await {
                            if meta.file_type().is_symlink() {
                                let _ = tokio::fs::remove_file(&target).await;
                            } else if meta.is_dir() {
                                let _ = tokio::fs::remove_dir_all(&target).await;
                            } else {
                                let _ = tokio::fs::remove_file(&target).await;
                            }
                        }
                        let _ = tokio::fs::create_dir_all(&target).await;
                    }
                }
            }
        }

        let reg_content = "Windows Registry Editor Version 5.00\n\n\
            [HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Shell Folders]\n\
            \"AppData\"=\"C:\\\\users\\\\steamuser\\\\AppData\\\\Roaming\"\n\
            \"Desktop\"=\"C:\\\\users\\\\steamuser\\\\Desktop\"\n\
            \"Personal\"=\"C:\\\\users\\\\steamuser\\\\Documents\"\n\
            \"My Video\"=\"C:\\\\users\\\\steamuser\\\\Videos\"\n\
            \"My Pictures\"=\"C:\\\\users\\\\steamuser\\\\Pictures\"\n\
            \"My Music\"=\"C:\\\\users\\\\steamuser\\\\Music\"\n\n\
            [HKEY_CURRENT_USER\\Software\\Wine\\DirectInput]\n\
            \"MouseWarpOverride\"=\"force\"\n\n\
            [HKEY_CURRENT_USER\\Software\\Wine\\X11 Driver]\n\
            \"ShowCursor\"=\"Y\"\n";

        let reg_file = self._install_dir.join("sanitize.reg");
        tokio::fs::write(&reg_file, reg_content).await?;
        let _ = Command::new("wine")
            .arg("regedit")
            .arg("/s")
            .arg("sanitize.reg")
            .env("WINEPREFIX", self.prefix_path.to_string_lossy().to_string())
            .status()
            .await;
        let _ = tokio::fs::remove_file(reg_file).await;

        // Agresywna relatywizacja hive'ów: usuwamy ścieżki hosta/użytkownika.
        Self::scrub_prefix_hives(&self.prefix_path, &self._install_dir).await?;
        Ok(())
    }

    async fn scrub_prefix_hives(prefix_path: &Path, install_dir: &Path) -> std::io::Result<()> {
        let home = env::var("HOME").unwrap_or_default();
        let user = env::var("USER").unwrap_or_default();
        let host = env::var("HOSTNAME").unwrap_or_default();
        let install_unix = install_dir.to_string_lossy().to_string();
        let mut replacements: Vec<(String, String)> = Vec::new();

        let home_windows = home.replace('/', "\\");
        let home_windows_reg = home.replace('/', "\\\\");
        let install_windows = install_unix.replace('/', "\\");
        let install_windows_reg = install_unix.replace('/', "\\\\");

        if !home.is_empty() {
            replacements.push((
                format!("Z:\\\\{}", home_windows_reg.trim_start_matches('\\')),
                "Z:\\\\portable".into(),
            ));
            replacements.push((
                format!("Z:\\{}", home_windows.trim_start_matches('\\')),
                "Z:\\portable".into(),
            ));
            replacements.push((home.clone(), "/portable".into()));
        }
        if !install_unix.is_empty() {
            replacements.push((
                format!("Z:\\\\{}", install_windows_reg.trim_start_matches('\\')),
                "Z:\\\\portable\\\\game".into(),
            ));
            replacements.push((
                format!("Z:\\{}", install_windows.trim_start_matches('\\')),
                "Z:\\portable\\game".into(),
            ));
            replacements.push((install_unix, "/portable/game".into()));
        }
        if !user.is_empty() {
            replacements.push((
                format!("\\\\users\\\\{}", user),
                "\\\\users\\\\steamuser".into(),
            ));
            replacements.push((format!("\\users\\{}", user), "\\users\\steamuser".into()));
            replacements.push((format!("/home/{}", user), "/home/steamuser".into()));
            replacements.push((user, "steamuser".into()));
        }
        if !host.is_empty() {
            replacements.push((host, "r2p-host".into()));
        }

        let monitor_re =
            Regex::new("(?i)edid|monitorid|adapterluid|videopcivendorid|videopcideviceid")
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        for hive in ["system.reg", "user.reg", "userdef.reg"] {
            let path = prefix_path.join(hive);
            if !path.exists() {
                continue;
            }
            let mut content = match tokio::fs::read_to_string(&path).await {
                Ok(c) => c,
                Err(_) => continue,
            };
            let original = content.clone();

            for (from, to) in &replacements {
                if !from.is_empty() {
                    content = content.replace(from, to);
                }
            }

            let filtered = content
                .lines()
                .filter(|line| !monitor_re.is_match(line))
                .collect::<Vec<_>>()
                .join("\n");

            if filtered != original {
                tokio::fs::write(&path, filtered).await?;
            }
        }
        Ok(())
    }

    pub async fn isolate_saves(&self) -> std::io::Result<()> {
        let save_root = self._install_dir.join("r2p_userdata");
        tokio::fs::create_dir_all(&save_root).await?;

        let users_dir = self.prefix_path.join("drive_c/users");
        let mut target_user_path = None;

        if let Ok(mut entries) = tokio::fs::read_dir(&users_dir).await {
            while let Some(entry) = entries.next_entry().await? {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                if name != "public" && name != "default" {
                    target_user_path = Some(entry.path());
                    break;
                }
            }
        }

        if let Some(user_path) = target_user_path {
            let appdata = user_path.join("AppData/Local");
            if appdata.exists() && !appdata.is_symlink() {
                let target = save_root.join("Local");
                // PANCERNE PRZENOSZENIE
                let status = Command::new("mv")
                    .arg(&appdata)
                    .arg(&target)
                    .status()
                    .await?;
                if status.success() {
                    #[cfg(unix)]
                    {
                        let link_base = appdata
                            .parent()
                            .map(Path::to_path_buf)
                            .unwrap_or_else(|| user_path.clone());
                        let rel = Self::relative_path(&target, link_base)?;
                        std::os::unix::fs::symlink(rel, &appdata)?;
                    }
                } else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Failed to move saves to isolated storage",
                    ));
                }
            }
        }
        Ok(())
    }

    fn relative_path(target: &Path, base_dir: PathBuf) -> std::io::Result<PathBuf> {
        let target_components: Vec<_> = target.components().collect();
        let base_components: Vec<_> = base_dir.components().collect();

        let mut common = 0usize;
        while common < target_components.len()
            && common < base_components.len()
            && target_components[common] == base_components[common]
        {
            common += 1;
        }

        if common == 0 {
            return Ok(target.to_path_buf());
        }

        let mut rel = PathBuf::new();
        for _ in common..base_components.len() {
            rel.push("..");
        }
        for comp in &target_components[common..] {
            rel.push(comp.as_os_str());
        }
        if rel.as_os_str().is_empty() {
            rel.push(".");
        }
        Ok(rel)
    }

    fn search_existing_base_prefix(home: &str, arch: &str) -> Option<PathBuf> {
        if home.is_empty() {
            return None;
        }

        let base_path = Path::new(home);
        let mut candidates = Vec::new();
        for root in Self::prefix_roots(base_path) {
            if root.exists() && root.is_dir() {
                Self::collect_prefix_candidates(&root, 2, &mut candidates);
            }
        }

        candidates.sort();
        candidates.dedup();
        candidates.sort_by(|a, b| {
            let score_a = Self::prefix_score(a, arch);
            let score_b = Self::prefix_score(b, arch);
            score_b.cmp(&score_a)
        });

        candidates.into_iter().find(|path| Self::valid_prefix(path))
    }

    fn prefix_roots(home: &Path) -> Vec<PathBuf> {
        let mut roots = vec![
            home.join("Games/R2L/base_pfx_win32"),
            home.join("Games/R2L/base_pfx_win64"),
            home.join("Games/R2L/prefixes"),
            home.join("Games/R2P/base_pfx_win32"),
            home.join("Games/R2P/base_pfx_win64"),
            home.join("Games/R2P/prefixes"),
            home.join(".wine"),
            home.join(".local/share/wineprefixes"),
            home.join(".local/share/lutris/wineprefixes"),
            home.join(".local/share/Steam/compatibilitytools.d"),
            home.join(".steam/root/compatibilitytools.d"),
            home.join(".local/share/Steam/steamapps/compatdata"),
        ];
        roots.extend(Self::var_app_roots(home));
        roots.extend(Self::steam_compatdata_roots(home));
        if let Ok(wine_prefix) = env::var("WINEPREFIX") {
            roots.push(PathBuf::from(wine_prefix));
        }
        if let Ok(lutris_prefix) = env::var("LUTRIS_PREFIX") {
            roots.push(PathBuf::from(lutris_prefix));
        }
        if let Ok(prefix_list) = env::var("LUTRIS_PREFIXES") {
            for entry in prefix_list.split(':') {
                let trimmed = entry.trim();
                if !trimmed.is_empty() {
                    roots.push(PathBuf::from(trimmed));
                }
            }
        }
        if let Ok(custom_root) =
            env::var("R2L_PREFIX_ROOT").or_else(|_| env::var("R2P_PREFIX_ROOT"))
        {
            roots.push(PathBuf::from(custom_root));
        }
        roots
    }

    fn var_app_roots(home: &Path) -> Vec<PathBuf> {
        let mut extra = Vec::new();
        let var_app = home.join(".var/app");
        if let Ok(entries) = std::fs::read_dir(var_app) {
            for entry in entries.flatten() {
                let wine = entry.path().join(".wine");
                if wine.is_dir() {
                    extra.push(wine);
                }
            }
        }
        extra
    }

    fn steam_compatdata_roots(home: &Path) -> Vec<PathBuf> {
        let mut extra = Vec::new();
        let compat = home.join(".steam/root/steamapps/compatdata");
        if let Ok(entries) = std::fs::read_dir(compat) {
            for entry in entries.flatten() {
                if entry.path().join("pfx").is_dir() {
                    extra.push(entry.path().join("pfx"));
                }
                if entry.path().join("pfx64").is_dir() {
                    extra.push(entry.path().join("pfx64"));
                }
            }
        }
        extra
    }

    fn collect_prefix_candidates(base: &Path, depth: u8, candidates: &mut Vec<PathBuf>) {
        if Self::valid_prefix(base) {
            candidates.push(base.to_path_buf());
        }

        let pfx_dir = base.join("pfx");
        if pfx_dir.is_dir() && Self::valid_prefix(&pfx_dir) {
            candidates.push(pfx_dir);
        }

        if depth == 0 {
            return;
        }

        if let Ok(entries) = std::fs::read_dir(base) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    Self::collect_prefix_candidates(&path, depth - 1, candidates);
                }
            }
        }
    }

    fn valid_prefix(path: &Path) -> bool {
        ["system.reg", "user.reg", "userdef.reg"]
            .iter()
            .all(|hive| path.join(hive).exists())
    }

    fn prefix_score(path: &Path, arch: &str) -> usize {
        let mut score = 0;
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            let lower = name.to_lowercase();
            if lower.contains("base_pfx") {
                score += 20;
            }
            if lower.contains(arch) {
                score += 30;
            }
            if lower.contains("wine") || lower.contains("proton") {
                score += 15;
            }
            if lower.contains("prefix") {
                score += 5;
            }
        }

        if path.join("drive_c").is_dir() {
            score += 10;
        }
        if path.join("drive_c/windows/system32/kernel32.dll").exists() {
            score += 20;
        }

        if let Ok(meta) = std::fs::metadata(path) {
            if let Ok(modified) = meta.modified() {
                if let Ok(age) = modified.elapsed() {
                    if age.as_secs() < 30 * 24 * 60 * 60 {
                        score += 15;
                    } else if age.as_secs() < 90 * 24 * 60 * 60 {
                        score += 7;
                    }
                }
            }
        }

        if let Ok(content) = std::fs::read_to_string(path.join("system.reg")) {
            if content.contains(&format!("WINEARCH={}", arch)) {
                score += 25;
            }
            if content.contains("Windows 10") || content.contains("Windows 11") {
                score += 5;
            }
            if content.contains("Proton") {
                score += 10;
            }
        }

        if let Ok(content) = std::fs::read_to_string(path.join("user.reg")) {
            if content.contains("XP") && arch == "win32" {
                score += 5;
            }
            if content.contains("Proton") {
                score += 8;
            }
        }

        score
    }

    pub async fn ensure_prefix_hives(portable_dir: &Path) -> io::Result<()> {
        let prefix = portable_dir.join("pfx");
        for hive in &["system.reg", "user.reg", "userdef.reg"] {
            let path = prefix.join(hive);
            let meta = tokio::fs::metadata(&path).await.map_err(|_| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Missing or locked prefix hive: {}", hive),
                )
            })?;
            if meta.len() == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Prefix hive {} is empty", hive),
                ));
            }
        }
        Ok(())
    }

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
        let icon_path = self._install_dir.join("r2p-icon.svg");
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

        script_content.push_str("run_bundled() {\n    export PROTONPATH=\"$1\"\n    export PATH=\"$PROTONPATH/bin:$PATH\"\n    export WINELOADER=\"$PROTONPATH/bin/wine\"\n");
        script_content.push_str(&format!(
            "    exec \"${{R2L_PREFIX[@]}}\" \"$WINELOADER\" \"$GAME_EXE\" \"${{FILTERED_ARGS[@]}}\"\n}}\n"
        ));

        script_content.push_str(
            "if [ -d \"$SCRIPT_DIR/../wine\" ]; then run_bundled \"$SCRIPT_DIR/../wine\"\n",
        );
        script_content.push_str(
            "elif [ -d \"$SCRIPT_DIR/wine\" ]; then run_bundled \"$SCRIPT_DIR/wine\"\nfi\n",
        );

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
        tokio::fs::write(&icon_path, Self::r2p_icon_svg()).await?;
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

    fn r2p_icon_svg() -> &'static str {
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

    fn desktop_icon_helper_script(game_name: &str) -> String {
        let escaped_game = game_name.replace('"', "\\\"");
        format!(
            r#"#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
GAME_NAME="{game_name}"
PLAY_SH="$SCRIPT_DIR/play.sh"
PLAY_AUTO_SH="$SCRIPT_DIR/play_auto.sh"
ICON_SVG="$SCRIPT_DIR/r2p-icon.svg"
ICON_PNG="$SCRIPT_DIR/icon.png"
ICON_FILE="$ICON_SVG"
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
Path=\"$SCRIPT_DIR\"
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

    pub async fn generate_unified_sfx<F>(
        &self,
        source_path: &Path,
        portable_dir: &Path,
        output_sh: &Path,
        proton_path: Option<&Path>,
        is_64bit: bool,
        scope: ExportScope,
        dry_run: bool,
        mut on_progress: F,
    ) -> std::io::Result<ExportArtifact>
    where
        F: FnMut(f32) + Send + 'static,
    {
        let game_name = &self._game_name;
        let total_files = WalkDir::new(portable_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .count();

        if let Some(proton) = proton_path {
            let target_wine = portable_dir.join("wine");
            tokio::fs::create_dir_all(&target_wine).await?;
            let status = Command::new("cp")
                .arg("-a")
                .arg("--reflink=auto")
                .arg(format!("{}/.", proton.display()))
                .arg(&target_wine)
                .status()
                .await?;
            if !status.success() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Failed to bundle Proton environment",
                ));
            }
        }

        Self::validate_unified_sfx_environment()
            .map_err(|msg| std::io::Error::new(std::io::ErrorKind::Other, msg))?;

        let gui_bin_path = PathBuf::from("target/release/installer_gui");
        if !gui_bin_path.exists() {
            let status = Command::new("cargo")
                .arg("build")
                .arg("--release")
                .arg("--bin")
                .arg("installer_gui")
                .status()
                .await?;
            if !status.success() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Failed to compile installer GUI",
                ));
            }
        }

        let total_size_bytes = Self::get_dir_size(portable_dir).await?;
        let total_size_mb = total_size_bytes / 1024 / 1024;

        let header = format!(
            r##"#!/bin/bash
GAME_NAME="{}"
TOTAL_FILES={}
IS_64BIT={}
REQ_SPACE_MB={}
THIS_SCRIPT="$(readlink -f "$0")"
# Natychmiastowe wyliczanie offsetow (Pancerny Parser R2L - Ultra Fast)
GREP_GUI=$(grep -aob -m 1 "R2L_GUI_""BIN_START" "$THIS_SCRIPT" | cut -d: -f1)
GUI_OFFSET=$((GREP_GUI + 19))
GREP_DATA=$(grep -aob -m 1 "R2L_DATA_""BIN_START" "$THIS_SCRIPT" | cut -d: -f1)
DATA_SIZE=$((GREP_DATA - GREP_GUI - 19))
DATA_OFFSET=$((GREP_DATA + 20))
TMP_GUI="/tmp/r2p_gui_$(date +%s)"
tail -c +$GUI_OFFSET "$THIS_SCRIPT" | head -c $DATA_SIZE > "$TMP_GUI"
sync
chmod +x "$TMP_GUI"
"$TMP_GUI" "$GAME_NAME" "$THIS_SCRIPT" "$DATA_OFFSET" "$TOTAL_FILES" "$IS_64BIT" "$REQ_SPACE_MB"
rm -f "$TMP_GUI"
exit 0
R2L_GUI_BIN_START
"##,
            game_name, total_files, is_64bit, total_size_mb
        );

        let mut out_file = tokio::fs::File::create(output_sh).await?;
        out_file.write_all(header.as_bytes()).await?;
        let gui_bytes = tokio::fs::read(&gui_bin_path).await?;
        out_file.write_all(&gui_bytes).await?;
        out_file.write_all(b"\nR2L_DATA_BIN_START\n").await?;

        let audits =
            Self::collect_export_audits(portable_dir, scope, dry_run, proton_path.as_deref())
                .await?;

        if dry_run {
            on_progress(1.0);
            return Ok(ExportArtifact {
                installer_path: self._install_dir.clone(),
                audits,
                scope,
                dry_run: true,
                source_path: source_path.to_path_buf(),
                prefix_path: portable_dir.join("pfx"),
            });
        }

        let selection = match scope {
            ExportScope::Full => TarSelection::All,
            ExportScope::PrefixOnly => TarSelection::Only(&["pfx"]),
            ExportScope::GameOnly => TarSelection::Exclude(&["pfx"]),
            ExportScope::LibsOnly => TarSelection::Only(&[
                "pfx",
                "play.sh",
                "play_auto.sh",
                "play_safe.sh",
                "adddesktopicon.sh",
                "README_LINUX.txt",
                "README_AUTO.txt",
                "r2p-icon.svg",
            ]),
        };

        let mut child = Command::new("tar");
        child
            .arg("-I")
            .arg("zstd -1 -T0")
            .arg("-c")
            .arg("-C")
            .arg(portable_dir)
            .stdout(std::process::Stdio::piped());

        match selection {
            TarSelection::All => {
                child.arg(".");
            }
            TarSelection::Exclude(excludes) => {
                for exclude in excludes {
                    child.arg("--exclude").arg(exclude);
                }
                child.arg(".");
            }
            TarSelection::Only(entries) => {
                for entry in entries {
                    child.arg(entry);
                }
            }
        }

        let mut child = child.spawn()?;

        if let Some(mut tar_out) = child.stdout.take() {
            let mut buffer = [0u8; 262144];
            let mut written = 0u64;
            let total_size = Self::get_dir_size(portable_dir).await?;
            while let Ok(n) = tar_out.read(&mut buffer).await {
                if n == 0 {
                    break;
                }
                out_file.write_all(&buffer[..n]).await?;
                written += n as u64;
                on_progress((written as f32 / total_size as f32).min(0.99));
            }
        }
        child.wait().await?;
        on_progress(1.0);

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = out_file.metadata().await?.permissions();
            perms.set_mode(0o755);
            out_file.set_permissions(perms).await?;
        }

        Ok(ExportArtifact {
            installer_path: output_sh.to_path_buf(),
            audits,
            scope,
            dry_run: false,
            source_path: source_path.to_path_buf(),
            prefix_path: portable_dir.join("pfx"),
        })
    }

    async fn hash_directory(path: &Path) -> std::io::Result<String> {
        let path = path.to_path_buf();
        tokio::task::spawn_blocking(move || {
            let entries = WalkDir::new(&path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .map(|e| e.path().to_path_buf())
                .collect::<Vec<_>>();
            Self::hash_paths_sync(path, entries)
        })
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
    }

    fn hash_paths_sync(base: PathBuf, mut files: Vec<PathBuf>) -> io::Result<String> {
        files.sort();
        let mut hasher = Sha256::new();
        for file_path in files {
            let rel = file_path.strip_prefix(&base).unwrap_or(&file_path);
            hasher.update(rel.to_string_lossy().as_bytes());
            hasher.update(&[0]);
            let mut file = File::open(&file_path)?;
            let mut buf = [0u8; 8192];
            loop {
                let n = file.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                hasher.update(&buf[..n]);
            }
        }
        Ok(format!("{:x}", hasher.finalize()))
    }

    async fn hash_subset_entries(base: &Path, entries: &[&str]) -> std::io::Result<String> {
        let base = base.to_path_buf();
        let entries = entries.iter().map(|rel| base.join(rel)).collect::<Vec<_>>();
        tokio::task::spawn_blocking(move || {
            let mut files = Vec::new();
            for entry_path in entries {
                if entry_path.is_file() {
                    files.push(entry_path);
                } else if entry_path.is_dir() {
                    for entry in WalkDir::new(&entry_path)
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .filter(|e| e.file_type().is_file())
                    {
                        files.push(entry.path().to_path_buf());
                    }
                }
            }
            Self::hash_paths_sync(base, files)
        })
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
    }

    async fn get_dir_size(path: &Path) -> std::io::Result<u64> {
        let mut total = 0;
        for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                total += entry.metadata()?.len();
            }
        }
        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn prepare_prefix(path: &Path, name: &str, arch: &str) -> PathBuf {
        let candidate = path.join(name);
        std::fs::create_dir_all(&candidate).unwrap();
        for hive in &["system.reg", "user.reg", "userdef.reg"] {
            std::fs::write(candidate.join(hive), format!("WINEARCH={}", arch)).unwrap();
        }
        candidate
    }

    #[test]
    fn search_existing_base_prefix_prefers_match() {
        let temp_dir = tempdir().unwrap();
        let home = temp_dir.path().to_string_lossy().to_string();
        let candidate = prepare_prefix(temp_dir.path(), "Games/R2L/base_pfx_win64", "win64");
        let found = Installer::search_existing_base_prefix(&home, "win64");
        assert_eq!(found.unwrap(), candidate);
    }

    #[test]
    fn prefix_score_prioritizes_architecture() {
        let temp_dir = tempdir().unwrap();
        let preferred = prepare_prefix(temp_dir.path(), "prefix_win64", "win64");
        let fallback = prepare_prefix(temp_dir.path(), "prefix_old", "win32");
        let score_preferred = Installer::prefix_score(&preferred, "win64");
        let score_fallback = Installer::prefix_score(&fallback, "win64");
        assert!(score_preferred > score_fallback);
    }

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
