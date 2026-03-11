use std::env;
use std::io;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, BufReader, BufWriter, AsyncWriteExt};
use tokio::process::Command;
use walkdir::WalkDir;
use regex::Regex;
use crate::installer::Installer;

impl Installer {
    pub(crate) async fn get_dir_size(path: &Path) -> io::Result<u64> {
        let mut total = 0;
        for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                total += entry.metadata()?.len();
            }
        }
        Ok(total)
    }

    pub(crate) async fn ensure_prefix_hives(portable_dir: &Path) -> io::Result<()> {
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

    pub async fn prepare_prefix_ext(&self, use_win32: bool) -> io::Result<Option<PathBuf>> {
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

        let status = Command::new("wine")
            .arg("wineboot")
            .arg("-i")
            .env("WINEPREFIX", self.prefix_path.to_string_lossy().to_string())
            .env("WINEARCH", arch)
            .status()
            .await?;

        if !status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "wineboot failed - prefix corrupted",
            ));
        }

        let _ = Command::new("wineserver")
            .arg("-w")
            .env("WINEPREFIX", self.prefix_path.to_string_lossy().to_string())
            .status()
            .await;

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

        self.sanitize_prefix().await?;

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

    async fn sanitize_prefix(&self) -> io::Result<()> {
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

        Self::scrub_prefix_hives(&self.prefix_path, &self._install_dir).await?;
        Ok(())
    }

    async fn scrub_prefix_hives(prefix_path: &Path, install_dir: &Path) -> io::Result<()> {
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
            
            let temp_path = path.with_extension("reg.tmp");
            let file_in = tokio::fs::File::open(&path).await?;
            let mut reader = BufReader::new(file_in);
            
            let file_out = tokio::fs::File::create(&temp_path).await?;
            let mut writer = BufWriter::new(file_out);
            
            let mut line = String::new();
            let mut changed = false;
            
            while reader.read_line(&mut line).await? > 0 {
                if monitor_re.is_match(&line) {
                    line.clear();
                    changed = true;
                    continue;
                }
                
                let mut new_line = line.clone();
                for (from, to) in &replacements {
                    if !from.is_empty() && new_line.contains(from) {
                        new_line = new_line.replace(from, to);
                        changed = true;
                    }
                }
                
                writer.write_all(new_line.as_bytes()).await?;
                line.clear();
            }
            
            writer.flush().await?;
            
            if changed {
                tokio::fs::rename(&temp_path, &path).await?;
            } else {
                let _ = tokio::fs::remove_file(&temp_path).await;
            }
        }
        Ok(())
    }

    pub async fn isolate_saves(&self) -> io::Result<()> {
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
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        "Failed to move saves to isolated storage",
                    ));
                }
            }
        }
        Ok(())
    }

    fn relative_path(target: &Path, base_dir: PathBuf) -> io::Result<PathBuf> {
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

    pub(crate) fn valid_prefix(path: &Path) -> bool {
        ["system.reg", "user.reg", "userdef.reg"]
            .iter()
            .all(|hive| path.join(hive).exists())
    }

    pub(crate) fn prefix_score(path: &Path, arch: &str) -> usize {
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
}
