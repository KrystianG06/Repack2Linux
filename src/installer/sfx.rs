use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use walkdir::WalkDir;
use sha2::{Digest, Sha256};
use crate::export::{ExportArtifact, ExportAudit, ExportScope};
use crate::installer::Installer;

pub enum TarSelection {
    All,
    Exclude(&'static [&'static str]),
    Only(&'static [&'static str]),
}

impl Installer {
    pub(crate) fn scope_subset(scope: ExportScope) -> Option<&'static [&'static str]> {
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
                "r2l_brand.svg",
                "icon.png",
            ]),
            ExportScope::LibsOnly => Some(&[
                "pfx",
                "play.sh",
                "play_auto.sh",
                "play_safe.sh",
                "adddesktopicon.sh",
                "README_LINUX.txt",
                "README_AUTO.txt",
                "r2l_brand.svg",
                "icon.png",
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
    ) -> io::Result<ExportArtifact>
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
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Failed to bundle Proton environment",
                ));
            }
        }

        Self::validate_unified_sfx_environment()
            .map_err(|msg| io::Error::new(io::ErrorKind::Other, msg))?;

        let mut gui_bin_path = Self::resolve_installer_gui_binary();
        if gui_bin_path.is_none() {
            let status = Command::new("cargo")
                .arg("build")
                .arg("--release")
                .arg("--bin")
                .arg("installer_gui")
                .status()
                .await?;
            if !status.success() {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Failed to compile installer GUI",
                ));
            }
            gui_bin_path = Self::resolve_installer_gui_binary();
        }
        let gui_bin_path = gui_bin_path.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::Other,
                "Installer GUI binary not found after build.",
            )
        })?;

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
        let gui_bytes = tokio::fs::read(gui_bin_path).await?;
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
                "r2l_brand.svg",
                "icon.png",
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
            let meta = out_file.metadata().await?;
            let mut perms = meta.permissions();
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

    async fn hash_directory(path: &Path) -> io::Result<String> {
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

    async fn hash_subset_entries(base: &Path, entries: &[&str]) -> io::Result<String> {
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
}
