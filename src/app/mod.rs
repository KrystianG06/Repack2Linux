use std::path::PathBuf;
use crate::config::{AppConfig, UiMode};
use crate::database::Database;
use crate::detector::RepackType;
use crate::export::{ExportAudit, ExportScope};
use crate::command_runner::CommandRunner;

pub mod i18n;
pub mod update;
pub mod view;

pub use i18n::Language;
pub use update::Message;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProtonSource {
    Cloud,
    Learned,
    Internal,
    Heuristic,
    Default,
    User,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExportStatus {
    Idle,
    Running(String),
    Success {
        path: String,
        audits: Vec<ExportAudit>,
        scope: ExportScope,
        dry_run: bool,
    },
    Error(String),
}

#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub enum Tab {
    #[default]
    Factory,
    Tools,
    Settings,
}

#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub enum FactoryMode {
    #[default]
    Live,
}

impl std::fmt::Display for FactoryMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FactoryMode::Live => write!(f, "LIVE (Quick Play)"),
        }
    }
}

pub struct RepackApp {
    pub game_name: String,
    pub repack_path: Option<PathBuf>,
    pub dest_path: PathBuf,
    pub game_exe_override: Option<PathBuf>,
    pub repack_type: RepackType,
    pub logs: Vec<String>,
    pub progress: f32,
    pub current_tab: Tab,
    pub cfg: AppConfig,
    pub ui_mode: UiMode,
    pub opt_dxvk: bool,
    pub opt_vcrun2022: bool,
    pub opt_xaudio: bool,
    pub opt_win32: bool,
    pub opt_ultra_compat: bool,
    pub opt_mangohud: bool,
    pub opt_gamemode: bool,
    pub opt_auto_launch: bool,
    pub opt_no_dxvk: bool,
    pub opt_legacy_mode: bool,
    pub opt_windows_version: String,
    pub opt_d3dx9: bool,
    pub opt_vcrun2005: bool,
    pub opt_vcrun2008: bool,
    pub opt_xact: bool,
    pub opt_physx: bool,
    pub opt_csmt: bool,
    pub mount_point: Option<String>,
    pub loop_dev: Option<String>,
    pub ge_protons: Vec<String>,
    pub selected_proton: Option<String>,
    pub lang: Language,
    pub factory_mode: FactoryMode,
    pub engine_insight: String,
    pub final_archive_path: Option<String>,
    pub gpu_vendor: String,
    pub is_path_dangerous: bool,
    pub db: Database,
    // EXPORT MODAL STATE
    pub show_export_modal: bool,
    pub opt_export_standalone: bool,
    pub opt_export_archive: bool,
    pub opt_export_installer: bool,
    pub opt_include_deps: bool,
    pub opt_skip_cleanup: bool,
    pub export_dest_path: Option<PathBuf>,
    pub export_status: ExportStatus,
    pub export_scope: ExportScope,
    pub export_dry_run: bool,
    pub animation_tick: u32,
    pub is_producing: bool,
    pub preset_count: u32,
    pub proton_source: ProtonSource,
    pub last_source_path_before_production: Option<PathBuf>,
    pub preset_inspector_source: String,
    pub preset_inspector_confidence: u8,
    pub preset_inspector_reason: String,
    pub preset_inspector_match: String,
    pub community_queue_pending: bool,
    pub community_queue_attempts: u32,
    pub community_last_retry_at: Option<String>,
    pub community_last_error: Option<String>,
    pub community_repo_root: Option<String>,
    pub community_remote_enabled: bool,
    pub show_welcome_overlay: bool,
    pub available_update: Option<String>,
    pub game_icon: Option<iced::widget::image::Handle>,
}

impl RepackApp {
    pub fn detect_gpu() -> String {
        if let Ok(result) = CommandRunner::new("lspci").run() {
            let s = result.stdout.to_lowercase();
            if s.contains("nvidia") {
                return "NVIDIA".to_string();
            }
            if s.contains("amd") || s.contains("radeon") {
                return "AMD".to_string();
            }
            if s.contains("intel") {
                return "INTEL".to_string();
            }
        }
        "GENERIC".to_string()
    }

    pub fn resolve_desktop_dir(home: &str) -> PathBuf {
        let out = std::process::Command::new("xdg-user-dir")
            .arg("DESKTOP")
            .output();
        if let Ok(out) = out {
            if out.status.success() {
                let candidate = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !candidate.is_empty() {
                    return PathBuf::from(candidate);
                }
            }
        }
        PathBuf::from(home).join("Desktop")
    }

    pub fn ensure_app_shortcut(cfg: &mut AppConfig) -> Result<(), String> {
        let home = std::env::var("HOME").unwrap_or_default();
        if home.is_empty() {
            return Err("HOME is empty".into());
        }

        let icon_svg = r###"<?xml version="1.0" encoding="UTF-8"?>
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
"###;

        let icons_dir = PathBuf::from(&home).join(".local/share/icons/hicolor/scalable/apps");
        let applications_dir = PathBuf::from(&home).join(".local/share/applications");
        let desktop_dir = Self::resolve_desktop_dir(&home);
        let icon_path = icons_dir.join("repack2linux.svg");
        let desktop_path = applications_dir.join("repack2linux.desktop");
        let desktop_shortcut_path = desktop_dir.join("Repack2Linux.desktop");

        let exe = std::env::current_exe()
            .ok()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "repack2linux".to_string());

        let desktop_content = format!(
            "[Desktop Entry]\nVersion=1.0\nType=Application\nName=Repack2Linux\nExec=\"{}\"\nIcon={}\nTerminal=false\nCategories=Game;Utility;\nStartupNotify=true\nStartupWMClass=repack2linux\n",
            exe,
            icon_path.display()
        );

        std::fs::create_dir_all(&icons_dir).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(&applications_dir).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(&desktop_dir).map_err(|e| e.to_string())?;
        std::fs::write(&icon_path, icon_svg).map_err(|e| e.to_string())?;
        std::fs::write(&desktop_path, &desktop_content).map_err(|e| e.to_string())?;
        std::fs::write(&desktop_shortcut_path, &desktop_content).map_err(|e| e.to_string())?;

        let _ = std::fs::remove_file(applications_dir.join("repack2linux-rs.desktop"));
        let _ = std::fs::remove_file(icons_dir.join("repack2linux-rs.svg"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for target in [&desktop_path, &desktop_shortcut_path] {
                if let Ok(meta) = std::fs::metadata(target) {
                    let mut perms = meta.permissions();
                    perms.set_mode(0o755);
                    let _ = std::fs::set_permissions(target, perms);
                }
            }
        }
        cfg.app_shortcut_installed = true;
        let _ = crate::config::ConfigManager::save(cfg);
        Ok(())
    }
}

const APP_VERSION: &str = "1.3.0";

impl Default for RepackApp {
    fn default() -> Self {
        let mut cfg = crate::config::ConfigManager::load();
        let _ = Self::ensure_app_shortcut(&mut cfg);
        let lang = if cfg.language.contains("Polish") {
            Language::Polish
        } else {
            Language::English
        };
        let ui_mode = cfg.ui_mode.clone();
        let mut protons = crate::proton::ProtonManager::list_ge_protons();
        protons.insert(0, "System Wine (Default)".to_string());
        let gpu = Self::detect_gpu();
        let db = Database::new();
        let preset_count = db.count_cloud_presets() as u32;
        let queue_snapshot = crate::community_sync::queue_snapshot();
        let show_welcome_overlay = cfg.welcome_screen_enabled;
        
        // Pomocnicze etykiety dla stanu początkowego
        let waiting_label = if lang == Language::Polish {
            "Oczekiwanie..."
        } else {
            "Waiting..."
        };
        let inspector_idle = if lang == Language::Polish {
            "Bezczynny"
        } else {
            "Idle"
        };
        let no_source = if lang == Language::Polish {
            "Brak wybranego źródła"
        } else {
            "No source selected"
        };

        Self {
            game_name: String::new(),
            repack_path: None,
            dest_path: PathBuf::from(&cfg.default_install_dir),
            game_exe_override: None,
            repack_type: RepackType::Unknown,
            logs: vec![format!("R2L Engine v{}: ONLINE. GPU: {}", APP_VERSION, gpu)],
            progress: 0.0,
            current_tab: Tab::Factory,
            cfg,
            ui_mode,
            opt_dxvk: true,
            opt_vcrun2022: true,
            opt_xaudio: true,
            opt_win32: false,
            opt_ultra_compat: true,
            opt_mangohud: false,
            opt_gamemode: true,
            opt_auto_launch: true,
            opt_no_dxvk: false,
            opt_legacy_mode: false,
            opt_windows_version: "win10".to_string(),
            opt_d3dx9: false,
            opt_vcrun2005: false,
            opt_vcrun2008: false,
            opt_xact: false,
            opt_physx: false,
            opt_csmt: true,
            mount_point: None,
            loop_dev: None,
            ge_protons: protons,
            selected_proton: Some("System Wine (Default)".into()),
            lang,
            factory_mode: FactoryMode::Live,
            engine_insight: waiting_label.into(),
            final_archive_path: None,
            gpu_vendor: gpu,
            is_path_dangerous: false,
            db,
            show_export_modal: false,
            opt_export_standalone: true,
            opt_export_archive: false,
            opt_export_installer: false,
            opt_include_deps: true,
            opt_skip_cleanup: false,
            export_dest_path: None,
            export_status: ExportStatus::Idle,
            export_scope: ExportScope::Full,
            export_dry_run: false,
            animation_tick: 0,
            is_producing: false,
            preset_count,
            proton_source: ProtonSource::Default,
            last_source_path_before_production: None,
            preset_inspector_source: inspector_idle.into(),
            preset_inspector_confidence: 0,
            preset_inspector_reason: no_source.into(),
            preset_inspector_match: "-".into(),
            community_queue_pending: queue_snapshot.pending,
            community_queue_attempts: queue_snapshot.attempts,
            community_last_retry_at: queue_snapshot.last_attempt_at,
            community_last_error: queue_snapshot.last_error,
            community_repo_root: queue_snapshot.repo_root,
            community_remote_enabled: queue_snapshot.remote_enabled,
            show_welcome_overlay,
            available_update: None,
            game_icon: None,
        }
    }
}
