use iced::Task;
use std::path::{Path, PathBuf};
use crate::config::UiMode;
use crate::export::{ExportArtifact, ExportScope};
use crate::app::{RepackApp, Tab, Language, ProtonSource, ExportStatus};
use crate::{community_sync, config, database, detector, installer, mounter, presets, proton};
use crate::command_runner::CommandRunner;
use crate::dependencies::DependencyManager;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Message {
    SelectRepackPressed,
    SelectFilePressed,
    StartProductionPressed,
    LogAppended(String),
    ProductionFinished(Result<String, String>),
    TabChanged(Tab),
    ToggleDxvk(bool),
    ToggleVcrun(bool),
    ToggleXAudio(bool),
    ToggleWin32(bool),
    ToggleUltra(bool),
    ToggleMango(bool),
    ToggleGamemode(bool),
    ToggleAutoLaunch(bool),
    ToggleNoDxvk(bool),
    ToggleLegacyMode(bool),
    ToggleWindowsVersion(String),
    ToggleD3dx9(bool),
    ToggleVcrun2005(bool),
    ToggleVcrun2008(bool),
    ToggleXact(bool),
    TogglePhysx(bool),
    ToggleCsmt(bool),
    ProtonSelected(String),
    DownloadProtonPressed,
    ProtonDownloadFinished(Result<String, String>),
    UnmountISO,
    LanguageChanged(Language),
    SelectGameExePressed,
    RollbackLearnedPressed,
    CopyLogsToClipboard,
    FixPathPressed,
    ProgressUpdated(f32),
    InstallMissingPressed,
    OpenDebugShellPressed,
    QuickPresetPressed(String),
    ToggleUiMode(UiMode),
    ToggleExportStandalone(bool),
    ToggleExportArchive(bool),
    ToggleExportInstaller(bool),
    ToggleIncludeDeps(bool),
    ToggleSkipCleanup(bool),
    RunExportPressed,
    CloseModalPressed,
    ModalBackdropClicked,
    SaveLogsPressed,
    AnalyzeLogsPressed,
    SelectExportPathPressed,
    ExportPathSelected(PathBuf),
    ExportFinished(Result<ExportArtifact, String>),
    SyncCloudDatabase,
    CloudSyncFinished(Result<serde_json::Value, String>),
    ProcessCommunityQueue,
    CommunityQueueProcessed(Result<String, String>),
    CommunitySyncFinished(Result<String, String>),
    CheckForUpdates,
    UpdateCheckFinished(Result<Option<String>, String>),
    OpenReleasesPage,
    DismissUpdateBanner,
    InstallAppShortcutPressed,
    DismissWelcomePressed,
    ToggleWelcomeAnimation(bool),
    ToggleWelcomeScreen(bool),
    KillWinePressed,
    Tick,
    ExportScopeChanged(ExportScope),
    ToggleDryRun(bool),
}

const RELEASES_URL: &str = "https://github.com/KrystianG06/Repack2Linux/releases/latest";
const VERSION_URL: &str = "https://raw.githubusercontent.com/KrystianG06/Repack2Linux/main/version.txt";
const APP_VERSION: &str = "1.3.0";

impl RepackApp {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick => {
                self.animation_tick = (self.animation_tick + 1) % 4;
                Task::none()
            }
            Message::LogAppended(line) => self.handle_log_appended(line),
            Message::ProgressUpdated(p) => {
                self.progress = p;
                Task::none()
            }
            Message::FixPathPressed => self.handle_fix_path(),
            Message::OpenDebugShellPressed => self.handle_open_debug_shell(),
            Message::DownloadProtonPressed => self.handle_download_proton(),
            Message::ProtonDownloadFinished(res) => self.handle_proton_download_finished(res),
            Message::ToggleDxvk(v) => {
                self.opt_dxvk = v;
                Task::none()
            }
            Message::ToggleVcrun(v) => {
                self.opt_vcrun2022 = v;
                Task::none()
            }
            Message::ToggleXAudio(v) => {
                self.opt_xaudio = v;
                Task::none()
            }
            Message::ToggleWin32(v) => {
                self.opt_win32 = v;
                Task::none()
            }
            Message::ToggleUltra(v) => {
                self.opt_ultra_compat = v;
                Task::none()
            }
            Message::ToggleNoDxvk(v) => {
                self.opt_no_dxvk = v;
                Task::none()
            }
            Message::ToggleMango(v) => {
                self.opt_mangohud = v;
                Task::none()
            }
            Message::ToggleGamemode(v) => {
                self.opt_gamemode = v;
                Task::none()
            }
            Message::ToggleAutoLaunch(v) => {
                self.opt_auto_launch = v;
                Task::none()
            }
            Message::ToggleLegacyMode(v) => self.handle_toggle_legacy(v),
            Message::ToggleWindowsVersion(v) => {
                self.opt_windows_version = v;
                Task::none()
            }
            Message::ToggleD3dx9(v) => {
                self.opt_d3dx9 = v;
                Task::none()
            }
            Message::ToggleVcrun2005(v) => {
                self.opt_vcrun2005 = v;
                Task::none()
            }
            Message::ToggleVcrun2008(v) => {
                self.opt_vcrun2008 = v;
                Task::none()
            }
            Message::ToggleXact(v) => {
                self.opt_xact = v;
                Task::none()
            }
            Message::TogglePhysx(v) => {
                self.opt_physx = v;
                Task::none()
            }
            Message::ToggleCsmt(v) => {
                self.opt_csmt = v;
                Task::none()
            }
            Message::ProtonSelected(v) => {
                self.selected_proton = Some(v);
                self.proton_source = ProtonSource::User;
                Task::none()
            }
            Message::SelectRepackPressed => self.handle_select_repack(),
            Message::SelectGameExePressed => self.handle_select_game_exe(),
            Message::RollbackLearnedPressed => self.handle_rollback_learned(),
            Message::SelectFilePressed => self.handle_select_file(),
            Message::StartProductionPressed => self.handle_start_production(),
            Message::ProductionFinished(res) => self.handle_production_finished(res),
            Message::InstallMissingPressed => self.handle_install_missing(),
            Message::TabChanged(t) => {
                self.current_tab = t;
                Task::none()
            }
            Message::LanguageChanged(l) => self.handle_language_changed(l),
            Message::UnmountISO => self.handle_unmount_iso(),
            Message::CopyLogsToClipboard => self.handle_copy_logs(),
            Message::QuickPresetPressed(preset_name) => self.handle_quick_preset(preset_name),
            Message::ToggleUiMode(mode) => self.handle_toggle_ui_mode(mode),
            Message::ToggleExportStandalone(v) => {
                self.opt_export_standalone = v;
                Task::none()
            }
            Message::ToggleExportArchive(v) => {
                self.opt_export_archive = v;
                Task::none()
            }
            Message::ToggleExportInstaller(v) => {
                self.opt_export_installer = v;
                Task::none()
            }
            Message::ToggleIncludeDeps(v) => {
                self.opt_include_deps = v;
                Task::none()
            }
            Message::ToggleSkipCleanup(v) => {
                self.opt_skip_cleanup = v;
                Task::none()
            }
            Message::ExportScopeChanged(scope) => {
                self.export_scope = scope;
                Task::none()
            }
            Message::ToggleDryRun(v) => {
                self.export_dry_run = v;
                Task::none()
            }
            Message::CloseModalPressed => {
                self.show_export_modal = false;
                Task::none()
            }
            Message::ModalBackdropClicked => Task::none(),
            Message::RunExportPressed => self.handle_run_export(),
            Message::SaveLogsPressed => self.handle_save_logs(),
            Message::AnalyzeLogsPressed => self.handle_analyze_logs(),
            Message::SelectExportPathPressed => self.handle_select_export_path(),
            Message::ExportPathSelected(p) => {
                self.export_dest_path = Some(p);
                Task::none()
            }
            Message::ExportFinished(res) => self.handle_export_finished(res),
            Message::SyncCloudDatabase => self.handle_sync_cloud_db(),
            Message::CloudSyncFinished(res) => self.handle_cloud_sync_finished(res),
            Message::ProcessCommunityQueue => self.handle_process_community_queue(),
            Message::CommunityQueueProcessed(res) => self.handle_community_queue_processed(res),
            Message::CommunitySyncFinished(res) => self.handle_community_sync_finished(res),
            Message::CheckForUpdates => self.handle_check_for_updates(),
            Message::UpdateCheckFinished(res) => self.handle_update_check_finished(res),
            Message::OpenReleasesPage => {
                if open::that(RELEASES_URL).is_err() {
                    self.logs
                        .push("[WARN] Nie udalo sie otworzyc strony releases.".into());
                }
                Task::none()
            }
            Message::DismissUpdateBanner => {
                self.available_update = None;
                Task::none()
            }
            Message::InstallAppShortcutPressed => {
                let result = Self::ensure_app_shortcut(&mut self.cfg);
                match result {
                    Ok(()) => self.logs.push(format!(
                        "[OK] {}",
                        if self.lang == Language::Polish {
                            "Skrót aplikacji został dodany do menu i pulpitu."
                        } else {
                            "Application shortcut was added to menu and desktop."
                        }
                    )),
                    Err(err) => self.logs.push(format!(
                        "[WARN] {}: {}",
                        if self.lang == Language::Polish {
                            "Nie udalo sie dodac skrotu aplikacji"
                        } else {
                            "Failed to add application shortcut"
                        },
                        err
                    )),
                }
                Task::none()
            }
            Message::DismissWelcomePressed => self.handle_dismiss_welcome(),
            Message::ToggleWelcomeAnimation(v) => {
                self.cfg.welcome_animation_enabled = v;
                let _ = config::ConfigManager::save(&self.cfg);
                Task::none()
            }
            Message::ToggleWelcomeScreen(v) => {
                self.cfg.welcome_screen_enabled = v;
                if !v {
                    self.show_welcome_overlay = false;
                } else {
                    self.show_welcome_overlay = true;
                }
                let _ = config::ConfigManager::save(&self.cfg);
                Task::none()
            }
            Message::KillWinePressed => self.handle_kill_wine(),
        }
    }

    fn handle_log_appended(&mut self, line: String) -> Task<Message> {
        let lower = line.to_lowercase();
        if !self.opt_no_dxvk
            && (lower.contains("vk_khr_external_memory_win32 not supported")
                || lower.contains("failed to create shared resource")
                || lower.contains("d3d11: failed to write shared resource"))
        {
            self.opt_no_dxvk = true;
            self.opt_dxvk = false;
            self.logs.push(format!(
                "[AUTO] DXVK issue detected for '{}'; enabling Safe renderer profile.",
                self.game_name
            ));
        }

        let now = chrono::Local::now().format("%H:%M:%S");
        let log_entry = format!("[{}] {}", now, line);

        if line.starts_with("[OPEN]") {
            let path_str = line.replace("[OPEN] Opening: ", "");
            let path = Path::new(&path_str);
            let dir = if path.exists() {
                if path.is_file() {
                    path.parent().unwrap_or(path)
                } else {
                    path
                }
            } else {
                path
            };
            let mut runner = CommandRunner::new("xdg-open");
            runner.allow_failure(true).arg(dir);
            let _ = runner.spawn();
        }

        self.logs.push(log_entry);
        if self.logs.len() > 100 {
            self.logs.remove(0);
        }
        Task::none()
    }

    fn handle_fix_path(&mut self) -> Task<Message> {
        if let Some(p) = self.repack_path.clone() {
            let mut parts: Vec<String> = p
                .to_string_lossy()
                .split('/')
                .map(|s| s.to_string())
                .collect();
            let mut current_base = PathBuf::from("/");
            for part in &mut parts {
                if part.is_empty() {
                    continue;
                }
                let old_name = part.clone();
                let new_name = old_name.trim().replace(" ", "_");
                let old_path = current_base.join(&old_name);
                let new_path = current_base.join(&new_name);
                if old_name != new_name {
                    let _ = std::fs::rename(&old_path, &new_path);
                }
                current_base = new_path;
                *part = new_name;
            }
            self.repack_path = Some(current_base.clone());
            self.is_path_dangerous = false;
            let _info = detector::Detector::detect(&current_base);
            self.game_exe_override = detector::Detector::find_game_exe(&current_base);
            self.logs.push(format!(
                "[SYSTEM] Sanitized Path: {}",
                current_base.display()
            ));
        }
        Task::none()
    }

    fn handle_open_debug_shell(&self) -> Task<Message> {
        if let Some(p) = &self.repack_path {
            let pfx = p.join("pfx");
            let exe = self.game_exe_override.clone().unwrap_or_default();
            let exe_dir = exe.parent().unwrap_or(p);
            let exe_name = exe.file_name().unwrap_or_default().to_string_lossy();

            let cmd = format!(
                "export WINEPREFIX='{}'; cd '{}'; echo '--- R2L DEBUG CONSOLE ---'; echo 'Prefix: {}'; echo 'Uruchamianie: {}'; wine '{}'; echo '--- PROCES ZAKONCZONY ---'; exec bash",
                pfx.display(), exe_dir.display(), pfx.display(), exe_name, exe_name
            );

            let terminal_cmd = format!(
                "gnome-terminal -- bash -c \"{0}\" || konsole -e bash -c \"{0}\" || xterm -e bash -c \"{0}\" || x-terminal-emulator -e bash -c \"{0}\"",
                cmd.replace("\"", "\\\"")
            );

            let mut runner = CommandRunner::new("sh");
            runner.allow_failure(true).args(&["-c", &terminal_cmd]);
            let _ = runner.spawn();
        }
        Task::none()
    }

    fn handle_download_proton(&mut self) -> Task<Message> {
        self.logs.push("[CORE] Syncing GE-Proton...".into());
        Task::perform(
            proton::ProtonManager::download_latest_ge(),
            Message::ProtonDownloadFinished,
        )
    }

    fn handle_proton_download_finished(&mut self, res: Result<String, String>) -> Task<Message> {
        match res {
            Ok(tag) => {
                self.logs.push(format!("[CORE] Integrated: {}", tag));
                self.ge_protons = proton::ProtonManager::list_ge_protons();
                self.ge_protons
                    .insert(0, "System Wine (Default)".to_string());
            }
            Err(e) => self.logs.push(format!("[ERROR] Proton Failed: {}", e)),
        }
        Task::none()
    }

    fn handle_toggle_legacy(&mut self, v: bool) -> Task<Message> {
        self.opt_legacy_mode = v;
        if v {
            self.opt_windows_version = "winxp".to_string();
            self.opt_win32 = true;
            self.opt_d3dx9 = true;
            self.opt_vcrun2005 = true;
            self.opt_vcrun2008 = true;
        }
        Task::none()
    }

    pub fn current_requirements_from_options(&self) -> detector::GameRequirements {
        detector::GameRequirements {
            needs_dxvk: self.opt_dxvk && !self.opt_no_dxvk,
            needs_xaudio: self.opt_xaudio,
            needs_vcrun: self.opt_vcrun2022,
            is_64bit: !self.opt_win32,
            needs_d3dx9: self.opt_d3dx9,
            needs_vcrun2005: self.opt_vcrun2005,
            needs_vcrun2008: self.opt_vcrun2008,
            needs_physx: self.opt_physx,
            needs_xact: self.opt_xact,
            engine_type: "UserDefined".into(),
            engine_version: None,
            has_anticheat: false,
        }
    }

    pub fn resolve_saved_exe(base_path: &Path, exe_hint: &str) -> Option<PathBuf> {
        let hint = exe_hint.trim();
        if hint.is_empty() {
            return None;
        }
        let candidate = PathBuf::from(hint);
        let resolved = if candidate.is_absolute() {
            candidate
        } else {
            base_path.join(candidate)
        };
        if resolved.exists() {
            Some(resolved)
        } else {
            None
        }
    }

    pub fn preferred_exe_hint_for_source(&self, source_path: &Path) -> Option<String> {
        let exe = self.game_exe_override.as_ref()?;
        if let Ok(rel) = exe.strip_prefix(source_path) {
            return Some(rel.to_string_lossy().to_string());
        }
        exe.file_name().map(|f| f.to_string_lossy().to_string())
    }

    pub fn remember_learned_profile(&mut self, source_path: &Path, prefix_path: Option<&Path>) {
        let reqs = self.current_requirements_from_options();
        let source_str = source_path.to_string_lossy().to_string();
        let exe_hint = self.preferred_exe_hint_for_source(source_path);
        let selected_proton = self.selected_proton.clone();
        let prefix_str = prefix_path.map(|p| p.to_string_lossy().to_string());

        self.db
            .save_preset(&source_str, &self.game_name, exe_hint.as_deref(), &reqs);

        self.db.save_learned_profile_json(
            &source_str,
            &self.game_name,
            exe_hint.as_deref(),
            selected_proton.as_deref(),
            prefix_str.as_deref(),
            &reqs,
            Some(&self.gpu_vendor),
        );

        self.logs.push(
            "[LEARN] Zapisano profil gry (prefix + biblioteki + Proton) do learned-profiles.json."
                .into(),
        );
    }

    pub fn refresh_community_status(&mut self) {
        let snapshot = community_sync::queue_snapshot();
        self.community_queue_pending = snapshot.pending;
        self.community_queue_attempts = snapshot.attempts;
        self.community_last_retry_at = snapshot.last_attempt_at;
        self.community_last_error = snapshot.last_error;
        self.community_repo_root = snapshot.repo_root;
        self.community_remote_enabled = snapshot.remote_enabled;
    }

    pub fn set_preset_inspector(
        &mut self,
        source: &str,
        confidence: u8,
        reason: impl Into<String>,
        matched: impl Into<String>,
    ) {
        self.preset_inspector_source = source.to_string();
        self.preset_inspector_confidence = confidence;
        self.preset_inspector_reason = reason.into();
        self.preset_inspector_match = matched.into();
    }

    fn handle_select_repack(&mut self) -> Task<Message> {
        if let Some(p) = rfd::FileDialog::new().pick_folder() {
            let info = detector::Detector::detect(&p);
            self.game_name = if info.clean_name != "Unknown" {
                info.clean_name.clone()
            } else {
                p.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Unknown Game".to_string())
            };
            self.repack_type = info.repack_type.clone();
            self.repack_path = Some(p.clone());
            self.game_exe_override = detector::Detector::find_game_exe(&p);
            
            // Wyodrębniamy ikonę gry i ładujemy do GUI
            if let Some(exe_path) = &self.game_exe_override {
                let icon_path = p.join("icon.png");
                if detector::Detector::extract_icon(exe_path, &icon_path) {
                    self.game_icon = Some(iced::widget::image::Handle::from_path(&icon_path));
                } else {
                    self.game_icon = None;
                }
            } else {
                self.game_icon = None;
            }

            self.is_path_dangerous = info.is_path_dangerous;

            let mut reqs = info.requirements.clone();
            let mut db_source = "Detector";
            let mut p_source = ProtonSource::Default;

            if let Some((
                learned_reqs,
                learned_exe,
                learned_proton,
                learned_prefix,
                learned_name,
                confidence,
                reason,
            )) = self.db.load_learned_profile_json(
                &p.to_string_lossy(),
                &self.game_name,
                Some(&self.gpu_vendor),
            ) {
                reqs = learned_reqs;
                if let Some(exe_str) = learned_exe {
                    if let Some(exe_path) = Self::resolve_saved_exe(&p, &exe_str) {
                        self.game_exe_override = Some(exe_path);
                    }
                }
                if let Some(proton_name) = learned_proton {
                    self.selected_proton = Some(proton_name);
                }
                if let Some(prefix) = learned_prefix {
                    self.logs
                        .push(format!("[LEARN] Reusing learned prefix hint: {}", prefix));
                }
                self.logs.push(format!(
                    "[LEARN] Applied learned JSON profile for {}.",
                    learned_name
                ));
                db_source = "LearnedJSON";
                p_source = ProtonSource::Learned;
                self.set_preset_inspector("LearnedJSON-GPU", confidence, reason, learned_name);
            } else if let Some((learned_reqs, learned_exe)) =
                self.db.get_preset(&p.to_string_lossy())
            {
                reqs = learned_reqs;
                if let Some(exe_str) = learned_exe {
                    if let Some(exe_path) = Self::resolve_saved_exe(&p, &exe_str) {
                        self.game_exe_override = Some(exe_path);
                    }
                }
                self.logs.push("[DB] Applied learned preset.".into());
                db_source = "Learned";
                p_source = ProtonSource::Learned;
                self.set_preset_inspector(
                    "LearnedSQLite",
                    52,
                    "Legacy path-based preset",
                    self.game_name.clone(),
                );
            } else if let Some((cloud_reqs, cloud_exe, cloud_proton, db_name, cloud_score)) =
                self.db.find_cloud_preset(&self.game_name)
            {
                reqs = cloud_reqs;
                if let Some(exe_str) = cloud_exe {
                    if !exe_str.is_empty() {
                        let exe_path = p.join(&exe_str);
                        if exe_path.exists() {
                            self.game_exe_override = Some(exe_path);
                        }
                    }
                }
                if let Some(proton_name) = cloud_proton {
                    if !proton_name.is_empty() {
                        self.selected_proton = Some(proton_name);
                        p_source = ProtonSource::Cloud;
                    }
                }
                self.logs.push(format!(
                    "[DB] DOPASOWANO: {} (Zastosowano optymalne ustawienia)",
                    db_name
                ));
                db_source = "Cloud";
                self.set_preset_inspector(
                    "Cloud",
                    ((cloud_score.saturating_mul(75)) / 100)
                        .saturating_add(20)
                        .min(92),
                    format!("Cloud fuzzy match score: {}", cloud_score),
                    db_name.clone(),
                );
            } else {
                self.set_preset_inspector(
                    "Detector",
                    28,
                    "No DB match; heuristic detector used",
                    self.game_name.clone(),
                );
            }

            self.opt_win32 = !reqs.is_64bit;
            self.opt_dxvk = reqs.needs_dxvk;
            self.opt_no_dxvk = !reqs.needs_dxvk;
            self.opt_xaudio = reqs.needs_xaudio;
            self.opt_vcrun2022 = reqs.needs_vcrun;
            self.opt_d3dx9 = reqs.needs_d3dx9;
            self.opt_vcrun2005 = reqs.needs_vcrun2005;
            self.opt_vcrun2008 = reqs.needs_vcrun2008;
            self.opt_physx = reqs.needs_physx;
            self.opt_xact = reqs.needs_xact;

            if let Some(profile) = self.db.load_game_profile(&p.to_string_lossy()) {
                if let Some(proton_name) = profile.selected_proton {
                    self.selected_proton = Some(proton_name);
                    self.proton_source = ProtonSource::Learned;
                }
                self.export_scope = profile.export_scope;
                self.opt_skip_cleanup = profile.skip_cleanup;
                self.opt_export_installer = profile.export_installer;
                self.opt_export_standalone = profile.export_standalone;
                self.export_dry_run = profile.dry_run;
                self.logs.push(format!(
                    "[PROFILE] Przeładowano profil {} (hash {}).",
                    profile.last_exported, profile.audit_hash
                ));
            }

            let engine_display = if let Some(v) = &reqs.engine_version {
                format!("{} v{}", reqs.engine_type, v)
            } else {
                reqs.engine_type.clone()
            };
            let ac_warning = if reqs.has_anticheat {
                " | 🛡️ ANTI-CHEAT DETECTED!"
            } else {
                ""
            };

            self.engine_insight = format!(
                "[{}] ENGINE: {} | ARCH: {} | REQS: DX{}, XA={}{}",
                db_source,
                engine_display,
                if reqs.is_64bit { "Win64" } else { "Win32" },
                info.suggested_dx,
                reqs.needs_xaudio,
                ac_warning
            );

            if p_source == ProtonSource::Default {
                if let Some(preset) = presets::GamePresets::get_preset(&self.game_name) {
                    let suggested = preset.suggested_proton;
                    if suggested == "GE-Proton" {
                        if let Some(latest_ge) =
                            self.ge_protons.iter().find(|p| p.contains("GE-Proton"))
                        {
                            self.selected_proton = Some(latest_ge.clone());
                            p_source = ProtonSource::Internal;
                        }
                    } else {
                        self.selected_proton = Some("System Wine (Default)".to_string());
                        p_source = ProtonSource::Internal;
                    }
                }
            }

            if p_source == ProtonSource::Default && reqs.is_64bit == false {
                p_source = ProtonSource::Heuristic;
            }

            self.proton_source = p_source;
        }
        Task::none()
    }

    fn handle_select_game_exe(&mut self) -> Task<Message> {
        if let Some(p) = self.repack_path.clone() {
            if let Some(f) = rfd::FileDialog::new()
                .set_directory(&p)
                .add_filter("Binary", &["exe"])
                .pick_file()
            {
                self.game_exe_override = Some(f.clone());
                
                // Aktualizujemy ikonę po ręcznej zmianie EXE
                let icon_path = p.join("icon.png");
                if detector::Detector::extract_icon(&f, &icon_path) {
                    self.game_icon = Some(iced::widget::image::Handle::from_path(&icon_path));
                }

                self.engine_insight = format!(
                    "MANUAL OVERRIDE | EXE: {:?}",
                    self.game_exe_override
                        .as_ref()
                        .map(|x| x.file_name().unwrap())
                );
            }
        }
        Task::none()
    }

    fn handle_rollback_learned(&mut self) -> Task<Message> {
        match self
            .db
            .rollback_learned_profile_json(&self.game_name, Some(&self.gpu_vendor))
        {
            Ok(msg) => {
                self.logs.push(format!("[LEARN] {}", msg));
                if let Some(source) = &self.repack_path {
                    if let Some((reqs, exe, proton, prefix, name, confidence, reason)) =
                        self.db.load_learned_profile_json(
                            &source.to_string_lossy(),
                            &self.game_name,
                            Some(&self.gpu_vendor),
                        )
                    {
                        self.opt_win32 = !reqs.is_64bit;
                        self.opt_dxvk = reqs.needs_dxvk;
                        self.opt_no_dxvk = !reqs.needs_dxvk;
                        self.opt_xaudio = reqs.needs_xaudio;
                        self.opt_vcrun2022 = reqs.needs_vcrun;
                        self.opt_d3dx9 = reqs.needs_d3dx9;
                        self.opt_vcrun2005 = reqs.needs_vcrun2005;
                        self.opt_vcrun2008 = reqs.needs_vcrun2008;
                        self.opt_physx = reqs.needs_physx;
                        self.opt_xact = reqs.needs_xact;

                        if let Some(exe_str) = exe {
                            if let Some(exe_path) = Self::resolve_saved_exe(source, &exe_str) {
                                self.game_exe_override = Some(exe_path);
                            }
                        }
                        if let Some(proton_name) = proton {
                            self.selected_proton = Some(proton_name);
                        }
                        if let Some(prefix_hint) = prefix {
                            self.logs
                                .push(format!("[LEARN] Rollback prefix hint: {}", prefix_hint));
                        }
                        self.set_preset_inspector("Rollback", confidence, reason, name);
                    }
                }
            }
            Err(err) => self.logs.push(format!("[WARN] Rollback failed: {}", err)),
        }
        Task::none()
    }

    fn handle_select_file(&mut self) -> Task<Message> {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Game Files", &["exe", "iso"])
            .pick_file()
        {
            let info = if path.extension().map(|e| e == "iso").unwrap_or(false) {
                match mounter::Mounter::mount(&path) {
                    Ok((mp, ld)) => {
                        self.mount_point = Some(mp.clone());
                        self.loop_dev = Some(ld);
                        let i = detector::Detector::detect(Path::new(&mp));
                        self.game_name = i.clean_name.clone();
                        self.repack_type = i.repack_type.clone();
                        self.repack_path = Some(PathBuf::from(mp));
                        self.opt_win32 = !i.is_64bit;
                        i
                    }
                    Err(e) => {
                        self.logs.push(format!("[FS] Mount Failed: {}", e));
                        return Task::none();
                    }
                }
            } else {
                self.repack_path = path.parent().map(|p| p.to_path_buf());
                self.game_name = path
                    .file_stem()
                    .map(|s| detector::Detector::clean_name(&s.to_string_lossy()))
                    .unwrap_or_else(|| "Unknown".to_string());
                let i = detector::Detector::detect(&path);
                self.opt_win32 = !i.is_64bit;
                self.repack_type = i.repack_type.clone();
                i
            };

            self.game_exe_override = self
                .repack_path
                .as_ref()
                .and_then(|p| detector::Detector::find_game_exe(p));

            let mut reqs = info.requirements.clone();
            let mut db_source = "Detector";
            let mut p_source = ProtonSource::Default;

            if let Some((
                learned_reqs,
                learned_exe,
                learned_proton,
                learned_prefix,
                learned_name,
                confidence,
                reason,
            )) = self.db.load_learned_profile_json(
                &self.repack_path.as_ref().unwrap().to_string_lossy(),
                &self.game_name,
                Some(&self.gpu_vendor),
            ) {
                reqs = learned_reqs;
                if let Some(exe_str) = learned_exe {
                    if let Some(base) = &self.repack_path {
                        if let Some(exe_path) = Self::resolve_saved_exe(base, &exe_str) {
                            self.game_exe_override = Some(exe_path);
                        }
                    }
                }
                if let Some(proton_name) = learned_proton {
                    self.selected_proton = Some(proton_name);
                }
                if let Some(prefix) = learned_prefix {
                    self.logs
                        .push(format!("[LEARN] Reusing learned prefix hint: {}", prefix));
                }
                self.logs.push(format!(
                    "[LEARN] Applied learned JSON profile for {}.",
                    learned_name
                ));
                db_source = "LearnedJSON";
                p_source = ProtonSource::Learned;
                self.set_preset_inspector("LearnedJSON-GPU", confidence, reason, learned_name);
            } else if let Some((learned_reqs, learned_exe)) = self
                .db
                .get_preset(&self.repack_path.as_ref().unwrap().to_string_lossy())
            {
                reqs = learned_reqs;
                if let Some(exe_str) = learned_exe {
                    if let Some(base) = &self.repack_path {
                        if let Some(exe_path) = Self::resolve_saved_exe(base, &exe_str) {
                            self.game_exe_override = Some(exe_path);
                        }
                    }
                }
                self.logs.push("[DB] Applied learned preset.".into());
                db_source = "Learned";
                p_source = ProtonSource::Learned;
                self.set_preset_inspector(
                    "LearnedSQLite",
                    52,
                    "Legacy path-based preset",
                    self.game_name.clone(),
                );
            } else if let Some((cloud_reqs, cloud_exe, cloud_proton, db_name, cloud_score)) =
                self.db.find_cloud_preset(&self.game_name)
            {
                reqs = cloud_reqs;
                if let Some(exe_str) = cloud_exe {
                    if !exe_str.is_empty() {
                        if let Some(p) = &self.repack_path {
                            let exe_path = p.join(&exe_str);
                            if exe_path.exists() {
                                self.game_exe_override = Some(exe_path);
                            }
                        }
                    }
                }
                if let Some(proton_name) = cloud_proton {
                    if !proton_name.is_empty() {
                        self.selected_proton = Some(proton_name);
                        p_source = ProtonSource::Cloud;
                    }
                }
                self.logs.push(format!(
                    "[DB] DOPASOWANO: {} (Zastosowano optymalne ustawienia)",
                    db_name
                ));
                db_source = "Cloud";
                self.set_preset_inspector(
                    "Cloud",
                    ((cloud_score.saturating_mul(75)) / 100)
                        .saturating_add(20)
                        .min(92),
                    format!("Cloud fuzzy match score: {}", cloud_score),
                    db_name.clone(),
                );
            } else {
                self.set_preset_inspector(
                    "Detector",
                    28,
                    "No DB match; heuristic detector used",
                    self.game_name.clone(),
                );
            }

            self.opt_win32 = !reqs.is_64bit;
            self.opt_dxvk = reqs.needs_dxvk;
            self.opt_no_dxvk = !reqs.needs_dxvk;
            self.opt_xaudio = reqs.needs_xaudio;
            self.opt_vcrun2022 = reqs.needs_vcrun;
            self.opt_d3dx9 = reqs.needs_d3dx9;
            self.opt_vcrun2005 = reqs.needs_vcrun2005;
            self.opt_vcrun2008 = reqs.needs_vcrun2008;
            self.opt_physx = reqs.needs_physx;
            self.opt_xact = reqs.needs_xact;

            let engine_display = if let Some(v) = &reqs.engine_version {
                format!("{} v{}", reqs.engine_type, v)
            } else {
                reqs.engine_type.clone()
            };
            let ac_warning = if reqs.has_anticheat {
                " | 🛡️ ANTI-CHEAT DETECTED!"
            } else {
                ""
            };

            self.engine_insight = format!(
                "[{}] ENGINE: {} | ARCH: {} | REQS: DX{}, XA={}{}",
                db_source,
                engine_display,
                if reqs.is_64bit { "Win64" } else { "Win32" },
                info.suggested_dx,
                reqs.needs_xaudio,
                ac_warning
            );

            if p_source == ProtonSource::Default && reqs.is_64bit == false {
                p_source = ProtonSource::Heuristic;
            }

            self.proton_source = p_source;
        }
        Task::none()
    }

    fn handle_start_production(&mut self) -> Task<Message> {
        let Some(source_path) = self.repack_path.clone() else {
            return Task::none();
        };
        self.is_producing = true;
        self.last_source_path_before_production = Some(source_path.clone());
        self.progress = 0.1;
        self.logs.clear();
        let game_name = self.game_name.clone();

        let options = crate::engine::ProductionOptions {
            dxvk: self.opt_dxvk,
            vcrun: self.opt_vcrun2022,
            win32: self.opt_win32,
            ultra_compat: self.opt_ultra_compat,
            mangohud: self.opt_mangohud,
            gamemode: self.opt_gamemode,
            no_dxvk: self.opt_no_dxvk,
            legacy_mode: self.opt_legacy_mode,
            d3dx9: self.opt_d3dx9,
            vcrun2005: self.opt_vcrun2005,
            vcrun2008: self.opt_vcrun2008,
            physx: self.opt_physx,
            xact: self.opt_xact,
        };

        let selected_proton = self.selected_proton.clone();
        let exe_override = self.game_exe_override.clone();
        let gpu = self.gpu_vendor.clone();
        let cfg = self.cfg.clone();

        let home = std::env::var("HOME").unwrap_or_default();
        let project_dir = PathBuf::from(&home).join(format!(
            "Games/R2L/Projects/{}",
            game_name.replace(" ", "_")
        ));

        Task::stream(crate::engine::Engine::run_production(
            game_name,
            source_path,
            project_dir,
            cfg,
            options,
            selected_proton,
            exe_override,
            gpu,
        ))
    }

    fn handle_production_finished(&mut self, res: Result<String, String>) -> Task<Message> {
        self.is_producing = false;
        let mut post_tasks: Vec<Task<Message>> = Vec::new();
        match res {
            Ok(path) => {
                self.progress = 1.0;
                if let Some(source_path) = self.last_source_path_before_production.clone() {
                    let learned_prefix = PathBuf::from(&path).join("pfx");
                    self.remember_learned_profile(&source_path, Some(&learned_prefix));

                    let reqs = self.current_requirements_from_options();
                    let game_name = self.game_name.clone();
                    let game_id = database::Database::normalize_game_id(&game_name);
                    let preferred_exe = self.preferred_exe_hint_for_source(&source_path);
                    let selected_proton = self.selected_proton.clone();
                    if let Some(repo_root) = community_sync::resolve_repo_root() {
                        post_tasks.push(Task::perform(
                            community_sync::sync_learned_preset(
                                repo_root,
                                game_id,
                                game_name,
                                preferred_exe,
                                selected_proton,
                                reqs,
                            ),
                            Message::CommunitySyncFinished,
                        ));
                    } else {
                        self.logs.push(
                            "[WARN] Community sync skipped: repository root not found.".into(),
                        );
                    }
                }
                self.repack_path = Some(PathBuf::from(&path));
                self.logs
                    .push(format!("[SUCCESS] Project synchronized in: {}", path));
                self.final_archive_path = Some(path);
                self.show_export_modal = true;
            }
            Err(e) => {
                self.progress = 0.0;
                self.logs.push(format!("[CRITICAL] Error: {}", e));
            }
        }
        self.last_source_path_before_production = None;
        if post_tasks.is_empty() {
            Task::none()
        } else {
            Task::batch(post_tasks)
        }
    }

    fn handle_install_missing(&mut self) -> Task<Message> {
        self.logs
            .push("[SYSTEM] Checking and fixing dependencies...".into());
        let mut missing = vec![];
        if !installer::Installer::check_tool("winetricks") {
            missing.push("winetricks");
        }
        if !installer::Installer::check_tool("ntlm_auth") {
            missing.push("winbind");
        }

        let sys_libs = DependencyManager::check_system_libs();
        if !sys_libs.is_empty() {
            missing.push("libvulkan1:i386");
            missing.push("libgl1:i386");
            missing.push("libasound2:i386");
        }
        if missing.is_empty() {
            self.logs.push("[OK] All dependencies OK.".into());
        } else {
            self.logs
                .push(format!("[INSTALL] Installing: {}", missing.join(", ")));
            let install_cmd = format!(
                "gnome-terminal -- bash -c 'echo Installing dependencies... && sudo dpkg --add-architecture i386 && sudo apt update && sudo apt install -y {}; echo Done! Press enter to close'; read",
                missing.join(" ")
            );
            let mut runner = CommandRunner::new("sh");
            runner.allow_failure(true).args(&["-c", &install_cmd]);
            let _ = runner.spawn();
        }
        Task::none()
    }

    fn handle_language_changed(&mut self, l: Language) -> Task<Message> {
        self.lang = l;
        self.cfg.language = format!("{:?}", l);
        if self.repack_path.is_none() && !self.is_producing {
            self.engine_insight = self.tr("factory_waiting").to_string();
        }
        if self.preset_inspector_confidence == 0 {
            self.preset_inspector_source = self.tr("factory_idle").to_string();
            self.preset_inspector_reason = self.tr("factory_no_source_selected").to_string();
            self.preset_inspector_match = "-".to_string();
        }
        let _ = config::ConfigManager::save(&self.cfg);
        Task::none()
    }

    fn handle_unmount_iso(&mut self) -> Task<Message> {
        if let Some(ld) = &self.loop_dev {
            let _ = mounter::Mounter::unmount(ld);
            self.mount_point = None;
            self.loop_dev = None;
            self.logs.push("[FS] ISO Released.".into());
        }
        Task::none()
    }

    fn handle_kill_wine(&mut self) -> Task<Message> {
        self.logs
            .push("[SYSTEM] Killing all Wine processes...".into());
        let commands = [
            ("wineserver", &["-k"][..], false),
            ("pkill", &["-9", "wine"][..], true),
            ("pkill", &["-9", "wineserver"][..], true),
        ];

        for (program, args, allow_failure) in commands {
            let mut runner = CommandRunner::new(program);
            runner.allow_failure(allow_failure).args(args);
            if let Err(err) = runner.run() {
                self.logs.push(format!("[WARN] {}", err));
            }
        }
        self.logs.push("[OK] Wine processes terminated.".into());
        Task::none()
    }

    fn handle_copy_logs(&self) -> Task<Message> {
        let all_logs = self.logs.join("\n");
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(all_logs);
        }
        Task::none()
    }

    fn handle_quick_preset(&mut self, preset_name: String) -> Task<Message> {
        if let Some(preset) = presets::GamePresets::get_preset(&preset_name) {
            self.opt_dxvk = preset.dxvk;
            self.opt_win32 = preset.win32;
            self.opt_d3dx9 = preset.deps.contains(&"d3dx9");
            self.opt_vcrun2005 = preset.deps.contains(&"vcrun2005");
            self.opt_vcrun2008 = preset.deps.contains(&"vcrun2008");
            self.opt_vcrun2022 = preset.deps.contains(&"vcrun2015")
                || preset.deps.contains(&"vcrun2017")
                || preset.deps.contains(&"vcrun2019")
                || preset.deps.contains(&"vcrun2022");
            self.opt_physx = preset.deps.contains(&"physx");
            self.opt_legacy_mode = preset.win32;

            self.logs.push(format!(
                "[PRESET] Applied: {} ({})",
                preset.name, preset.notes
            ));
            self.engine_insight =
                format!("PRESET: {} | DLLs: {}", preset.name, preset.dll_overrides);

            let suggested = preset.suggested_proton;
            if suggested == "GE-Proton" {
                if let Some(latest_ge) = self.ge_protons.iter().find(|p| p.contains("GE-Proton")) {
                    self.selected_proton = Some(latest_ge.clone());
                    self.logs
                        .push(format!("[AUTO] Wybrano zalecane srodowisko: {}", latest_ge));
                }
            } else {
                self.selected_proton = Some("System Wine (Default)".to_string());
                self.logs
                    .push("[AUTO] Wybrano zalecane srodowisko: System Wine (Default)".into());
            }
        } else {
            self.logs.push(format!("[WARN] Not found: {}", preset_name));
        }
        Task::none()
    }

    fn handle_toggle_ui_mode(&mut self, mode: UiMode) -> Task<Message> {
        self.ui_mode = mode.clone();
        self.cfg.ui_mode = mode;
        let _ = config::ConfigManager::save(&self.cfg);
        let mode_str = if self.ui_mode == UiMode::Simple {
            "SIMPLE"
        } else {
            "ADVANCED"
        };
        self.logs
            .push(format!("[UI] Switched to {} mode", mode_str));
        Task::none()
    }

    fn handle_save_logs(&mut self) -> Task<Message> {
        let target = self.dest_path.join("r2p_live_logs.txt");
        match std::fs::write(&target, self.logs.join("\n")) {
            Ok(_) => self
                .logs
                .push(format!("[LOG] Saved logs: {}", target.display())),
            Err(e) => self.logs.push(format!("[ERROR] Save failed: {}", e)),
        }
        Task::none()
    }

    fn handle_analyze_logs(&mut self) -> Task<Message> {
        let errors = self
            .logs
            .iter()
            .filter(|l| l.contains("[ERROR]") || l.contains("[CRITICAL]") || l.contains("[WINE]"))
            .count();
        let oks = self
            .logs
            .iter()
            .filter(|l| l.contains("[OK]") || l.contains("[SUCCESS]"))
            .count();
        let verdict = if errors == 0 {
            "STABLE"
        } else if errors <= 4 {
            "CHECK WARNINGS"
        } else {
            "NEEDS FIX"
        };
        self.logs.push(format!(
            "[ANALYZE] {} | success:{} | issues:{}",
            verdict, oks, errors
        ));
        Task::none()
    }

    fn handle_sync_cloud_db(&mut self) -> Task<Message> {
        self.logs.push("[DB] Synchronizacja bazy wiedzy...".into());
        Task::perform(
            async move {
                if let Ok(local_data) = tokio::fs::read_to_string("cloud/games.sample.json").await {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&local_data) {
                        return Ok(json);
                    }
                }

                let client = reqwest::Client::new();
                let urls = [
                    "https://raw.githubusercontent.com/KrystianG06/Repack2Linux/master/presets.json",
                    "https://raw.githubusercontent.com/KrystianG06/Repack2Proton/master/presets.json",
                ];
                for url in urls {
                    if let Ok(res) = client.get(url).send().await {
                        if let Ok(data) = res.json::<serde_json::Value>().await {
                            return Ok(data);
                        }
                    }
                }
                Err("Cloud sync failed for both Repack2Linux and legacy Repack2Proton URLs".into())
            },
            Message::CloudSyncFinished,
        )
    }

    fn handle_cloud_sync_finished(
        &mut self,
        res: Result<serde_json::Value, String>,
    ) -> Task<Message> {
        match res {
            Ok(data) => {
                let count = self.db.update_cloud_database(data);
                self.logs.push(format!(
                    "[DB] Synchronizacja zakonczona. Zaimportowano {} gier.",
                    count
                ));
            }
            Err(e) => {
                self.logs.push(format!(
                    "[WARN] Synchronizacja cloud niedostepna: {}. Uzywam lokalnej bazy.",
                    e
                ));
            }
        }
        Task::none()
    }

    fn handle_check_for_updates(&mut self) -> Task<Message> {
        Task::perform(
            async move {
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(4))
                    .build()
                    .map_err(|e| format!("http client: {}", e))?;
                let resp = client
                    .get(VERSION_URL)
                    .send()
                    .await
                    .map_err(|e| format!("download: {}", e))?;
                if !resp.status().is_success() {
                    return Err(format!("status {}", resp.status()));
                }
                let remote = resp
                    .text()
                    .await
                    .map_err(|e| format!("read body: {}", e))?
                    .trim()
                    .to_string();
                if remote.is_empty() {
                    return Ok(None);
                }
                if Self::is_remote_version_newer(APP_VERSION, &remote) {
                    Ok(Some(remote))
                } else {
                    Ok(None)
                }
            },
            Message::UpdateCheckFinished,
        )
    }

    fn handle_update_check_finished(
        &mut self,
        res: Result<Option<String>, String>,
    ) -> Task<Message> {
        match res {
            Ok(Some(v)) => self.available_update = Some(v),
            Ok(None) => {}
            Err(_) => {}
        }
        Task::none()
    }

    pub fn is_remote_version_newer(current: &str, remote: &str) -> bool {
        fn parse(v: &str) -> Vec<u32> {
            v.trim()
                .trim_start_matches(['v', 'V'])
                .split('.')
                .map(|p| p.parse::<u32>().unwrap_or(0))
                .collect()
        }

        let a = parse(current);
        let b = parse(remote);
        let max_len = a.len().max(b.len());
        for i in 0..max_len {
            let av = *a.get(i).unwrap_or(&0);
            let bv = *b.get(i).unwrap_or(&0);
            if bv > av {
                return true;
            }
            if bv < av {
                return false;
            }
        }
        false
    }

    fn handle_community_sync_finished(&mut self, res: Result<String, String>) -> Task<Message> {
        match res {
            Ok(msg) => self.logs.push(format!("[COMMUNITY] {}", msg)),
            Err(err) => self
                .logs
                .push(format!("[WARN] Community sync failed: {}", err)),
        }
        self.refresh_community_status();
        Task::none()
    }

    fn handle_process_community_queue(&mut self) -> Task<Message> {
        let Some(repo_root) = community_sync::resolve_repo_root() else {
            self.logs
                .push("[COMMUNITY] Retry queue skipped: repository root not found.".into());
            self.refresh_community_status();
            return Task::none();
        };

        Task::perform(
            community_sync::process_retry_queue(repo_root),
            Message::CommunityQueueProcessed,
        )
    }

    fn handle_dismiss_welcome(&mut self) -> Task<Message> {
        self.show_welcome_overlay = false;
        if !self.cfg.first_launch_completed {
            self.cfg.first_launch_completed = true;
            let _ = config::ConfigManager::save(&self.cfg);
        }
        Task::none()
    }

    fn handle_community_queue_processed(&mut self, res: Result<String, String>) -> Task<Message> {
        match res {
            Ok(msg) => {
                if msg != "queue empty" {
                    self.logs.push(format!("[COMMUNITY] {}", msg));
                }
            }
            Err(err) => self
                .logs
                .push(format!("[WARN] Community retry failed: {}", err)),
        }
        self.refresh_community_status();
        Task::none()
    }

    fn handle_select_export_path(&mut self) -> Task<Message> {
        if let Some(p) = rfd::FileDialog::new().pick_folder() {
            return Task::done(Message::ExportPathSelected(p));
        }
        Task::none()
    }

    fn handle_run_export(&mut self) -> Task<Message> {
        if self.export_dest_path.is_none()
            && (self.opt_export_standalone || self.opt_export_installer)
        {
            self.logs
                .push("[ERROR] Wybierz najpierw folder docelowy!".into());
            return Task::none();
        }

        if self.opt_export_installer {
            if let Err(msg) = installer::Installer::validate_unified_sfx_environment() {
                self.logs.push(format!("[ERROR] {}", msg));
                self.export_status = ExportStatus::Error(msg);
                return Task::none();
            }
        }

        self.export_status = ExportStatus::Running("Przygotowywanie plików do eksportu...".into());
        let game_name = self.game_name.clone();
        let source_dir = self.repack_path.clone().unwrap_or_default();
        let export_dir = self.export_dest_path.clone().unwrap_or(source_dir.clone());
        let selected_proton = self.selected_proton.clone();

        let do_standalone = self.opt_export_standalone;
        let do_installer = self.opt_export_installer;
        let auto_launch = self.opt_auto_launch;
        let scope = self.export_scope;
        let dry_run = self.export_dry_run;
        let skip_cleanup = self.opt_skip_cleanup;
        let mangohud = self.opt_mangohud;
        let gamemode = self.opt_gamemode;
        let no_dxvk = self.opt_no_dxvk;
        let gpu_vendor = self.gpu_vendor.clone();
        let (preset_dll_overrides, preset_env_vars) =
            if let Some(preset) = presets::GamePresets::get_preset(&game_name) {
                (
                    Some(preset.dll_overrides.to_string()),
                    Some(preset.env_vars),
                )
            } else {
                (None, None)
            };

        let exe_abs = self.game_exe_override.clone().unwrap_or_default();
        let mut exe_rel = if let Ok(rel) = exe_abs.strip_prefix(&source_dir) {
            rel.to_path_buf()
        } else {
            exe_abs.file_name().map(PathBuf::from).unwrap_or(exe_abs)
        };
        if exe_rel.as_os_str().is_empty() || !source_dir.join(&exe_rel).exists() {
            if let Some(auto_exe) = detector::Detector::find_game_exe(&source_dir) {
                if let Ok(rel) = auto_exe.strip_prefix(&source_dir) {
                    exe_rel = rel.to_path_buf();
                }
            }
        }

        let is_64 = !self.opt_win32;
        let exe_for_icon = if self.game_exe_override.is_some() {
            self.game_exe_override.clone()
        } else {
            detector::Detector::find_game_exe(&source_dir)
        };

        Task::stream(async_stream::stream! {
            let final_portable_folder = export_dir.join(format!("{}_Portable", game_name.replace(" ", "_")));
            let final_installer_sh = export_dir.join(format!("{}.sh", game_name.replace(" ", "_")));

            let _ = tokio::process::Command::new("wineserver").arg("-k").status().await;
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            yield Message::ProgressUpdated(0.05);

            let _ = tokio::fs::create_dir_all(&final_portable_folder).await;

            yield Message::LogAppended("[EXPORT] Kopiowanie plików gry...".into());
            let _ = tokio::process::Command::new("cp")
                .arg("-a").arg("--reflink=auto")
                .arg(format!("{}/.", source_dir.to_string_lossy()))
                .arg(&final_portable_folder).status().await;

            yield Message::ProgressUpdated(0.20);
            
            let proton_path = if selected_proton.as_deref() == Some("System Wine (Default)") {
                None
            } else {
                crate::proton::ProtonManager::get_path(&selected_proton.clone().unwrap_or_default())
            };

            if let Some(ref p_path) = proton_path {
                yield Message::LogAppended("[EXPORT] Bundling Wine/Proton runtime...".into());
                let wine_dest = final_portable_folder.join("wine");
                let _ = tokio::fs::create_dir_all(&wine_dest).await;
                let _ = tokio::process::Command::new("cp")
                    .arg("-a").arg("--reflink=auto")
                    .arg(format!("{}/.", p_path.to_string_lossy()))
                    .arg(&wine_dest).status().await;
            }

            let inst = installer::Installer::new(&game_name, final_portable_folder.clone());

            let _ = inst.generate_portable_script_ext(
                &exe_rel.to_string_lossy(),
                mangohud,
                gamemode,
                no_dxvk,
                proton_path.clone(),
                &gpu_vendor,
                preset_dll_overrides.clone(),
                preset_env_vars,
            ).await;
            let _ = inst.generate_readme().await;
            
            yield Message::LogAppended("[EXPORT] Wyciąganie ikony gry...".into());
            let icon_dest = final_portable_folder.join("icon.png");
            if let Some(exe_path) = &exe_for_icon {
                let _ = crate::detector::Detector::extract_icon(exe_path, &icon_dest);
            }

            let mut export_artifact = ExportArtifact {
                installer_path: final_portable_folder.clone(),
                audits: Vec::new(),
                scope,
                dry_run,
                source_path: source_dir.clone(),
                prefix_path: final_portable_folder.join("pfx"),
            };

            if do_installer {
                yield Message::LogAppended("[EXPORT] Budowanie instalatora SFX...".into());
                let (tx, mut rx) = tokio::sync::mpsc::channel(100);
                let inst_clone = inst;
                let temp_clone = final_portable_folder.clone();
                let sh_clone = final_installer_sh.clone();
                let proton_clone = proton_path.clone();
                let source_clone = source_dir.clone();

                let handle = tokio::spawn(async move {
                    inst_clone.generate_unified_sfx(
                        &source_clone,
                        &temp_clone,
                        &sh_clone,
                        proton_clone.as_deref(),
                        is_64,
                        scope,
                        dry_run,
                        move |p| {
                            let _ = tx.try_send(p);
                        },
                    ).await
                });

                while let Some(p) = rx.recv().await {
                    yield Message::ProgressUpdated(0.25 + (p * 0.70));
                }

                let artifact = match handle.await {
                    Ok(Ok(artifact)) => artifact,
                    Ok(Err(err)) => {
                        yield Message::ExportFinished(Err(err.to_string()));
                        return;
                    }
                    Err(_) => {
                        yield Message::ExportFinished(Err("Błąd podczas generowania SFX".into()));
                        return;
                    }
                };
                export_artifact = artifact;
                if !dry_run && !do_standalone {
                    if skip_cleanup {
                        yield Message::LogAppended(
                            "[CLEANUP] Zachowano folder eksportu zgodnie z ustawieniem.".into(),
                        );
                    } else {
                        let _ = tokio::fs::remove_dir_all(&final_portable_folder).await;
                    }
                }
            }

            if !do_installer {
                match installer::Installer::collect_export_audits(
                    &final_portable_folder,
                    scope,
                    dry_run,
                    None,
                )
                .await
                {
                    Ok(audits) => export_artifact.audits = audits,
                    Err(err) => {
                        let message = err.to_string();
                        yield Message::ExportFinished(Err(message));
                        return;
                    }
                }
            }

            yield Message::ProgressUpdated(1.0);

            if auto_launch && !dry_run {
                yield Message::LogAppended("[EXPORT] Uruchamianie gry...".into());
                let launch_path = if do_installer {
                    export_artifact.installer_path.clone()
                } else {
                    final_portable_folder.join("play_auto.sh")
                };
                let mut runner = CommandRunner::new("bash");
                runner.arg(launch_path);
                let _ = runner.spawn();
            }

            yield Message::ExportFinished(Ok(export_artifact));
        })
    }

    fn handle_export_finished(&mut self, res: Result<ExportArtifact, String>) -> Task<Message> {
        match res {
            Ok(artifact) => {
                let path_str = artifact.installer_path.to_string_lossy().to_string();
                self.export_status = ExportStatus::Success {
                    path: path_str.clone(),
                    audits: artifact.audits.clone(),
                    scope: artifact.scope,
                    dry_run: artifact.dry_run,
                };
                self.logs
                    .push(format!("[SUCCESS] Eksport zakonczony w: {}", path_str));

                let audit_hash = artifact
                    .audits
                    .first()
                    .map(|a| a.sha256.clone())
                    .unwrap_or_default();
                let profile = database::GameProfile {
                    game_id: database::Database::normalize_game_id(&self.game_name),
                    game_name: self.game_name.clone(),
                    source_path: artifact.source_path.to_string_lossy().to_string(),
                    prefix_path: artifact.prefix_path.to_string_lossy().to_string(),
                    selected_proton: self.selected_proton.clone(),
                    export_scope: artifact.scope,
                    skip_cleanup: self.opt_skip_cleanup,
                    export_installer: self.opt_export_installer,
                    export_standalone: self.opt_export_standalone,
                    dry_run: artifact.dry_run,
                    audit_hash,
                    last_exported: chrono::Local::now().to_rfc3339(),
                };
                self.db.save_game_profile(&profile);

                if let Some(project_path) = self.repack_path.take() {
                    if self.opt_skip_cleanup {
                        self.logs.push(format!(
                            "[CLEANUP] Zachowano projekt: {} (skip cleanup).",
                            project_path.display()
                        ));
                    } else {
                        self.logs.push(format!(
                            "[CLEANUP] Usuwanie plików roboczych projektu: {}...",
                            project_path.display()
                        ));
                        return Task::perform(
                            async move {
                                let _ = tokio::fs::remove_dir_all(project_path).await;
                            },
                            |_| {
                                Message::LogAppended(
                                    "[CLEANUP] Projekt wyczyszczony pomyślnie.".into(),
                                )
                            },
                        );
                    }
                }
            }
            Err(e) => {
                self.export_status = ExportStatus::Error(e.clone());
                self.logs.push(format!("[ERROR] Eksport nieudany: {}", e));
            }
        }
        Task::none()
    }
}
