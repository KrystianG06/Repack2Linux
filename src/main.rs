use iced::widget::{button, column, container, row, stack, text, Space};
use iced::{font, window, Alignment, Background, Border, Color, Element, Length, Task, Theme};
use std::path::{Path, PathBuf};

mod command_runner;
mod community_sync;
mod config;
mod database;
mod dependencies;
mod detector;
mod engine;
mod export;
mod installer;
mod mounter;
mod presets;
mod proton;
mod shortcuts;
mod ui;

use command_runner::CommandRunner;
use export::{ExportArtifact, ExportAudit, ExportScope};
use ui::theme::{root_background, ACCENT_CYAN, DEEP_DARK, TEXT_DIM};

pub use config::UiMode;

pub fn main() -> iced::Result {
    iced::application("Repack2Linux v1.01", RepackApp::update, RepackApp::view)
        .window(app_window_settings())
        .theme(|_| Theme::Dark)
        .subscription(RepackApp::subscription)
        .run_with(|| {
            (
                RepackApp::default(),
                Task::batch(vec![
                    Task::done(Message::SyncCloudDatabase),
                    Task::done(Message::ProcessCommunityQueue),
                ]),
            )
        })
}

fn app_window_settings() -> window::Settings {
    window::Settings {
        icon: app_window_icon(),
        #[cfg(target_os = "linux")]
        platform_specific: window::settings::PlatformSpecific {
            application_id: "repack2linux".to_string(),
            ..Default::default()
        },
        ..window::Settings::default()
    }
}

fn app_window_icon() -> Option<window::Icon> {
    const SIZE: u32 = 64;
    let mut rgba = vec![0_u8; (SIZE * SIZE * 4) as usize];
    let cx = (SIZE as f32) / 2.0;
    let cy = (SIZE as f32) / 2.0;
    let radius = 19.5_f32;

    for y in 0..SIZE {
        for x in 0..SIZE {
            let i = ((y * SIZE + x) * 4) as usize;
            let mut r = 5_u8;
            let mut g = 6_u8;
            let mut b = 15_u8;
            let a = 255_u8;

            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let d = (dx * dx + dy * dy).sqrt();
            if d <= radius {
                let t = (x as f32 + y as f32) / ((SIZE - 1) as f32 * 2.0);
                r = (14.0 + (255.0 - 14.0) * t) as u8;
                g = (95.0 + (79.0 - 95.0) * t) as u8;
                b = (174.0 + (88.0 - 174.0) * t) as u8;
            }

            rgba[i] = r;
            rgba[i + 1] = g;
            rgba[i + 2] = b;
            rgba[i + 3] = a;
        }
    }

    window::icon::from_rgba(rgba, SIZE, SIZE).ok()
}

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

pub struct RepackApp {
    pub game_name: String,
    pub repack_path: Option<PathBuf>,
    pub dest_path: PathBuf,
    pub game_exe_override: Option<PathBuf>,
    pub repack_type: detector::RepackType,
    pub logs: Vec<String>,
    pub progress: f32,
    pub current_tab: Tab,
    pub cfg: config::AppConfig,
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
    pub db: database::Database,
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

impl Default for RepackApp {
    fn default() -> Self {
        let mut cfg = config::ConfigManager::load();
        Self::ensure_app_shortcut(&mut cfg);
        let lang = if cfg.language.contains("Polish") {
            Language::Polish
        } else {
            Language::English
        };
        let ui_mode = cfg.ui_mode.clone();
        let mut protons = proton::ProtonManager::list_ge_protons();
        protons.insert(0, "System Wine (Default)".to_string());
        let gpu = Self::detect_gpu();
        let db = database::Database::new();
        let preset_count = db.count_cloud_presets() as u32;
        let queue_snapshot = community_sync::queue_snapshot();
        let show_welcome_overlay = cfg.welcome_screen_enabled;
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
            repack_type: detector::RepackType::Unknown,
            logs: vec![format!("R2L Engine v1.01: ONLINE. GPU: {}", gpu)],
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
        }
    }
}

#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub enum Tab {
    #[default]
    Factory,
    Tools,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    English,
    Polish,
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Language::English => write!(f, "English"),
            Language::Polish => write!(f, "Polski"),
        }
    }
}

impl Language {
    pub const ALL: [Language; 2] = [Language::English, Language::Polish];
}

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
    DismissWelcomePressed,
    ToggleWelcomeAnimation(bool),
    ToggleWelcomeScreen(bool),
    KillWinePressed,
    Tick,
    ExportScopeChanged(ExportScope),
    ToggleDryRun(bool),
}

impl RepackApp {
    fn ensure_app_shortcut(cfg: &mut config::AppConfig) {
        let home = std::env::var("HOME").unwrap_or_default();
        if home.is_empty() {
            return;
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
        let icon_path = icons_dir.join("repack2linux.svg");
        let desktop_path = applications_dir.join("repack2linux.desktop");

        let exe = std::env::current_exe()
            .ok()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "repack2linux".to_string());

        let desktop_content = format!(
            "[Desktop Entry]\nVersion=1.0\nType=Application\nName=Repack2Linux\nExec=\"{}\"\nIcon={}\nTerminal=false\nCategories=Game;Utility;\nStartupNotify=true\n",
            exe,
            icon_path.display()
        );

        if std::fs::create_dir_all(&icons_dir).is_ok()
            && std::fs::create_dir_all(&applications_dir).is_ok()
            && std::fs::write(&icon_path, icon_svg).is_ok()
            && std::fs::write(&desktop_path, desktop_content).is_ok()
        {
            let _ = std::fs::remove_file(applications_dir.join("repack2linux-rs.desktop"));
            let _ = std::fs::remove_file(icons_dir.join("repack2linux-rs.svg"));
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(meta) = std::fs::metadata(&desktop_path) {
                    let mut perms = meta.permissions();
                    perms.set_mode(0o755);
                    let _ = std::fs::set_permissions(&desktop_path, perms);
                }
            }
            cfg.app_shortcut_installed = true;
            let _ = config::ConfigManager::save(cfg);
        }
    }

    fn detect_gpu() -> String {
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

    pub fn tr<'a>(&self, key: &'a str) -> &'a str {
        match (self.lang, key) {
            (Language::Polish, "factory") => "PRODUKCJA",
            (Language::Polish, "tools") => "LOGI / DEBUG",
            (Language::Polish, "settings") => "KONFIGURACJA",
            (Language::Polish, "deploy_unit") => "CENTRUM PRODUKCYJNE",
            (Language::Polish, "deploy_desc") => "PRODUKCJA SAMOWYSTARCZALNYCH PACZEK",
            (Language::Polish, "engine_modules") => "MODUŁY SILNIKA",
            (Language::Polish, "runtime_env") => "ŚRODOWISKO BAZOWE",
            (Language::Polish, "browse_src") => "WYBIERZ ŹRÓDŁO",
            (Language::Polish, "iso_proto") => "OBRAZ ISO / EXE",
            (Language::Polish, "init_proto") => "START PRODUCTION",
            (Language::Polish, "lang_select") => "Wybór Języka",
            (Language::Polish, "dxvk_trans") => "DirectX / DXVK Armor",
            (Language::Polish, "vc_run") => "Biblioteki VC++",
            (Language::Polish, "xaudio_fix") => "XAudio / Sound Fix",
            (Language::Polish, "win32_mode") => "Legacy 32-bit Prefix",
            (Language::Polish, "ultra_compat") => "Ultra Compatibility (Full Set)",
            (Language::Polish, "no_dxvk") => "Disable DXVK (Safe OpenGL Mode)",
            (Language::Polish, "mangohud_over") => "Nakładka MangoHUD",
            (Language::Polish, "feral_gamemode") => "Feral GameMode",
            (Language::Polish, "auto_launch") => "Uruchom po zakończeniu",
            (Language::Polish, "factory_mode") => "TRYB PRACY",
            (Language::Polish, "insight") => "ANALIZA HEURYSTYCZNA",
            (Language::Polish, "change_exe") => "Zmień EXE",
            (Language::Polish, "copy_logs") => "Kopiuj Logi",
            (Language::Polish, "unmount_iso") => "ODMONTUJ ISO",
            (Language::Polish, "download_ge") => "POBIERZ GE-PROTON",
            (Language::Polish, "fix_path") => "NAPRAW ŚCIEŻKĘ (PEŁNY ŁAŃCUCH)",
            (Language::Polish, "open_debug") => "OTWÓRZ TERMINAL DEBUGOWANIA",
            (Language::Polish, "legacy_compat") => "KOMPATYBILNOŚĆ LEGACY",
            (Language::Polish, "legacy_mode") => "Tryb legacy (XP/32-bit)",
            (Language::Polish, "directx9") => "DirectX 9 (d3dx9)",
            (Language::Polish, "vcrun2005") => "VC++ 2005 Runtime",
            (Language::Polish, "vcrun2008") => "VC++ 2008 Runtime",
            (Language::Polish, "xact_audio") => "XACT (Audio)",
            (Language::Polish, "physx") => "PhysX",
            (Language::Polish, "enable_csmt") => "Włącz CSMT (Staging)",
            (Language::Polish, "system_language") => "SYSTEM I JĘZYK",
            (Language::Polish, "cloud_knowledge") => "BAZA WIEDZY (CLOUD)",
            (Language::Polish, "sync_now") => "SYNCHRONIZUJ TERAZ",
            (Language::Polish, "gpu_vendor") => "Dostawca GPU:",
            (Language::Polish, "export_options") => "OPCJE EKSPORTU",
            (Language::Polish, "export_standalone") => "Samodzielny folder portable",
            (Language::Polish, "export_installer") => "Instalator Unified SFX (.sh)",
            (Language::Polish, "include_deps") => "Dołącz zależności",
            (Language::Polish, "auto_launch_export") => "Auto start po eksporcie",
            (Language::Polish, "community_sync") => "SYNCHRONIZACJA SPOŁECZNOŚCIOWA",
            (Language::Polish, "community_queue_pending") => "OCZEKUJE NA RETRY",
            (Language::Polish, "community_queue_empty") => "KOLEJKA PUSTA",
            (Language::Polish, "community_never") => "nigdy",
            (Language::Polish, "community_none") => "brak",
            (Language::Polish, "community_not_found") => "nie znaleziono",
            (Language::Polish, "queue_attempts") => "Liczba prób",
            (Language::Polish, "remote_token") => "Token zdalny",
            (Language::Polish, "enabled") => "WŁĄCZONY",
            (Language::Polish, "disabled") => "WYŁĄCZONY",
            (Language::Polish, "last_retry") => "Ostatnia próba",
            (Language::Polish, "last_error") => "Ostatni błąd",
            (Language::Polish, "repo_root") => "Repo root",
            (Language::Polish, "retry_queue_now") => "URUCHOM RETRY TERAZ",
            (Language::Polish, "factory_title") => "REPACK2LINUX FACTORY",
            (Language::Polish, "factory_preset_applied") => "PRESET Z BAZY",
            (Language::Polish, "factory_ready") => "GOTOWE",
            (Language::Polish, "factory_select_game") => "Wybierz grę",
            (Language::Polish, "factory_auto_detect") => "AUTO-DETEKCJA",
            (Language::Polish, "factory_hardware") => "SPRZĘT",
            (Language::Polish, "factory_environment") => "ŚRODOWISKO",
            (Language::Polish, "factory_mode_label") => "TRYB",
            (Language::Polish, "factory_open_source") => "WYBIERZ ŹRÓDŁO",
            (Language::Polish, "factory_processing") => "PRZETWARZANIE...",
            (Language::Polish, "factory_start_production") => "START PRODUKCJI",
            (Language::Polish, "factory_change_source") => "Zmień źródło",
            (Language::Polish, "factory_activity_feed") => "AKTYWNOŚĆ",
            (Language::Polish, "factory_preset_inspector") => "INSPEKTOR PRESETÓW",
            (Language::Polish, "factory_source") => "ŹRÓDŁO",
            (Language::Polish, "factory_confidence") => "PEWNOŚĆ",
            (Language::Polish, "factory_match") => "DOPASOWANIE",
            (Language::Polish, "factory_reason") => "UZASADNIENIE",
            (Language::Polish, "factory_rollback_learned") => "COFNIJ UCZENIE",
            (Language::Polish, "factory_idle") => "Bezczynny",
            (Language::Polish, "factory_heuristic_analysis") => "ANALIZA HEURYSTYCZNA",
            (Language::Polish, "factory_advanced_pipeline") => "PIPELINE ZAAWANSOWANY",
            (Language::Polish, "factory_simple_mode") => "TRYB PROSTY",
            (Language::Polish, "factory_base_environment") => "BAZOWE ŚRODOWISKO",
            (Language::Polish, "factory_gpu") => "GPU",
            (Language::Polish, "factory_change_exe") => "Zmień EXE",
            (Language::Polish, "factory_copy_logs") => "Kopiuj logi",
            (Language::Polish, "factory_engine_parameters") => "PARAMETRY SILNIKA",
            (Language::Polish, "factory_dxvk") => "DXVK",
            (Language::Polish, "factory_xaudio") => "XAudio",
            (Language::Polish, "factory_vcrun2022") => "VC++ 2022",
            (Language::Polish, "factory_d3dx9") => "DirectX 9 (d3dx9)",
            (Language::Polish, "factory_vcrun2005") => "VC++ 2005",
            (Language::Polish, "factory_vcrun2008") => "VC++ 2008",
            (Language::Polish, "factory_physx_support") => "PhysX",
            (Language::Polish, "factory_xact_legacy") => "XACT (legacy)",
            (Language::Polish, "factory_prefix_32") => "Prefix 32-bit",
            (Language::Polish, "factory_browse_source") => "WYBIERZ ŹRÓDŁO",
            (Language::Polish, "factory_mount_iso") => "ZAMONTUJ ISO",
            (Language::Polish, "factory_producing") => "PRODUKCJA...",
            (Language::Polish, "factory_production") => "PRODUKCJA",
            (Language::Polish, "factory_live_feed") => "LIVE LOG",
            (Language::Polish, "factory_waiting") => "Oczekiwanie...",
            (Language::Polish, "factory_no_source_selected") => "Brak wybranego źródła",
            (Language::Polish, "tools_drivers_ok") => "Wszystkie zależności systemowe są dostępne.",
            (Language::Polish, "tools_system_health") => "ZDROWIE SYSTEMU",
            (Language::Polish, "tools_wine_proton") => "Wine/Proton",
            (Language::Polish, "tools_winetricks") => "Winetricks",
            (Language::Polish, "tools_ntlm") => "NTLM auth",
            (Language::Polish, "tools_drivers32") => "Biblioteki 32-bit",
            (Language::Polish, "tools_vulkan_runtime") => "Vulkan Runtime",
            (Language::Polish, "tools_gamemode") => "GameMode",
            (Language::Polish, "tools_mangohud") => "MangoHUD",
            (Language::Polish, "tools_utility_actions") => "AKCJE NARZĘDZIOWE",
            (Language::Polish, "tools_fix_system") => "NAPRAW SYSTEM",
            (Language::Polish, "tools_kill_wine") => "ZABIJ WSZYSTKIE PROCESY WINE",
            (Language::Polish, "tools_copy_logs") => "Kopiuj logi",
            (Language::Polish, "tools_save_logs") => "Zapisz logi",
            (Language::Polish, "tools_analyze") => "Analizuj",
            (Language::Polish, "tools_live_logs") => "LIVE LOGI",
            (Language::Polish, "tools_dependencies") => "ZALEŻNOŚCI",
            (Language::Polish, "tools_missing_packages") => "Brakujące pakiety",
            (Language::Polish, "tools_ok") => "OK",
            (Language::Polish, "tools_fail") => "BŁĄD",
            (Language::Polish, "tools_ready") => "GOTOWE",
            (Language::Polish, "tools_missing") => "BRAKUJE",
            (Language::Polish, "sidebar_tagline") => "GAME FACTORY",
            (Language::Polish, "cloud_database") => "BAZA CLOUD",
            (Language::Polish, "presets_loaded") => "presetów wczytano",
            (Language::Polish, "welcome_title") => "WITAJ W R2L",
            (Language::Polish, "welcome_subtitle") => "Repack2Linux",
            (Language::Polish, "welcome_points") => "• Ikona aplikacji została dodana do menu pulpitu.\n• Synchronizacja społeczności i uczenie profili są aktywne.\n• Gotowe środowisko pracy jest aktywne od razu po starcie.",
            (Language::Polish, "welcome_animation") => "Włącz animację powitania",
            (Language::Polish, "welcome_screen_setting") => "Pokazuj ekran powitalny przy starcie",
            (Language::Polish, "welcome_start") => "ZACZNIJ KORZYSTAĆ Z R2L",
            (Language::Polish, "export_config_title") => "KONFIGURACJA EKSPORTU",
            (Language::Polish, "export_config_desc") => "Zbuduj samowystarczalną paczkę i sprawdź prefix przed wysyłką",
            (Language::Polish, "export_pack_options") => "OPCJE PAKOWANIA",
            (Language::Polish, "export_keep_workdir") => "Zachowaj folder roboczy i dane tymczasowe",
            (Language::Polish, "export_scope_title") => "ZAKRES EKSPORTU",
            (Language::Polish, "export_dry_run_label") => "Dry run - weryfikacja prefixu bez zapisu",
            (Language::Polish, "export_target") => "CEL EKSPORTU",
            (Language::Polish, "export_change_folder") => "Zmień folder...",
            (Language::Polish, "export_no_folder") => "Brak wybranego folderu",
            (Language::Polish, "export_start") => "ROZPOCZNIJ EKSPORT",
            (Language::Polish, "export_cancel") => "ANULUJ",
            (Language::Polish, "export_running") => "EKSPORT W TRAKCIE",
            (Language::Polish, "export_progress") => "Postęp operacji:",
            (Language::Polish, "export_scope_label") => "Zakres eksportu",
            (Language::Polish, "export_running_dry") => "Dry run: weryfikacja prefixu przez skan",
            (Language::Polish, "export_running_full") => "Tryb produkcyjny: paczka zostanie utworzona",
            (Language::Polish, "export_running_note") => "Buduję stabilny pakiet offline i weryfikuję prefix.",
            (Language::Polish, "export_done_title") => "EKSPORT ZAKOŃCZONY",
            (Language::Polish, "export_done_desc") => "Gotowa paczka czeka w żądanej lokalizacji.",
            (Language::Polish, "export_scope_short") => "Zakres",
            (Language::Polish, "export_done_dry") => "Dry run: sprawdzono tylko integralność prefixu",
            (Language::Polish, "export_done_full") => "Tryb produkcyjny: wszystkie komponenty zapisane",
            (Language::Polish, "export_no_integrity") => "Brak danych integralności - spróbuj ponownie lub wykonaj dry run.",
            (Language::Polish, "export_integrity") => "Weryfikacja integralności:",
            (Language::Polish, "export_open_folder") => "OTWÓRZ FOLDER",
            (Language::Polish, "export_close") => "ZAMKNIJ",
            (Language::Polish, "export_error") => "BŁĄD EKSPORTU",
            (Language::Polish, "export_back_to_config") => "WRÓĆ DO KONFIGURACJI",

            (_, "factory") => "FACTORY",
            (_, "tools") => "LOGS / DEBUG",
            (_, "settings") => "CONFIGURATION",
            (_, "deploy_unit") => "PRODUCTION CENTER",
            (_, "deploy_desc") => "PACKAGE PRODUCTION",
            (_, "engine_modules") => "ENGINE MODULES",
            (_, "runtime_env") => "BASE RUNTIME",
            (_, "browse_src") => "BROWSE SOURCE",
            (_, "iso_proto") => "ISO / EXE FILE",
            (_, "init_proto") => "START PRODUCTION",
            (_, "lang_select") => "Language Selection",
            (_, "dxvk_trans") => "DirectX / DXVK Armor",
            (_, "vc_run") => "VC++ Runtimes",
            (_, "xaudio_fix") => "XAudio / Sound Fix",
            (_, "win32_mode") => "Legacy 32-bit Prefix",
            (_, "ultra_compat") => "Ultra Compatibility (Full Set)",
            (_, "no_dxvk") => "Disable DXVK (Safe OpenGL Mode)",
            (_, "mangohud_over") => "MangoHUD Overlay",
            (_, "feral_gamemode") => "Feral GameMode",
            (_, "auto_launch") => "Launch after production",
            (_, "unmount_iso") => "UNMOUNT ISO",
            (_, "download_ge") => "DOWNLOAD GE-PROTON",
            (_, "factory_mode") => "WORK MODE",
            (_, "insight") => "HEURISTIC ANALYSIS",
            (_, "change_exe") => "Change EXE",
            (_, "copy_logs") => "Copy Logs",
            (_, "fix_path") => "FIX FULL PATH CHAIN",
            (_, "open_debug") => "OPEN DEBUG SHELL",
            (_, "legacy_compat") => "LEGACY COMPATIBILITY",
            (_, "legacy_mode") => "Legacy Mode (XP/32bit)",
            (_, "directx9") => "DirectX 9 (d3dx9)",
            (_, "vcrun2005") => "VC++ 2005 Runtime",
            (_, "vcrun2008") => "VC++ 2008 Runtime",
            (_, "xact_audio") => "XACT (Audio)",
            (_, "physx") => "PhysX",
            (_, "enable_csmt") => "Enable CSMT (Staging)",
            (_, "system_language") => "SYSTEM & LANGUAGE",
            (_, "cloud_knowledge") => "CLOUD KNOWLEDGE",
            (_, "sync_now") => "SYNC NOW",
            (_, "gpu_vendor") => "GPU Vendor:",
            (_, "export_options") => "EXPORT OPTIONS",
            (_, "export_standalone") => "Standalone Portable Folder",
            (_, "export_installer") => "Unified SFX Installer (.sh)",
            (_, "include_deps") => "Include Dependencies",
            (_, "auto_launch_export") => "Auto Launch after export",
            (_, "community_sync") => "COMMUNITY SYNC",
            (_, "community_queue_pending") => "PENDING RETRY",
            (_, "community_queue_empty") => "QUEUE EMPTY",
            (_, "community_never") => "never",
            (_, "community_none") => "none",
            (_, "community_not_found") => "not found",
            (_, "queue_attempts") => "Queue attempts",
            (_, "remote_token") => "Remote token",
            (_, "enabled") => "ENABLED",
            (_, "disabled") => "DISABLED",
            (_, "last_retry") => "Last retry",
            (_, "last_error") => "Last error",
            (_, "repo_root") => "Repo root",
            (_, "retry_queue_now") => "RETRY QUEUE NOW",
            (_, "factory_title") => "REPACK2LINUX FACTORY",
            (_, "factory_preset_applied") => "PRESET APPLIED",
            (_, "factory_ready") => "READY",
            (_, "factory_select_game") => "Select game",
            (_, "factory_auto_detect") => "AUTO-DETECT",
            (_, "factory_hardware") => "HARDWARE",
            (_, "factory_environment") => "ENVIRONMENT",
            (_, "factory_mode_label") => "MODE",
            (_, "factory_open_source") => "OPEN SOURCE",
            (_, "factory_processing") => "PROCESSING...",
            (_, "factory_start_production") => "START PRODUCTION",
            (_, "factory_change_source") => "Change source",
            (_, "factory_activity_feed") => "ACTIVITY FEED",
            (_, "factory_preset_inspector") => "PRESET INSPECTOR",
            (_, "factory_source") => "SOURCE",
            (_, "factory_confidence") => "CONFIDENCE",
            (_, "factory_match") => "MATCH",
            (_, "factory_reason") => "REASON",
            (_, "factory_rollback_learned") => "ROLLBACK LEARNED",
            (_, "factory_idle") => "Idle",
            (_, "factory_heuristic_analysis") => "HEURISTIC ANALYSIS",
            (_, "factory_advanced_pipeline") => "ADVANCED PIPELINE",
            (_, "factory_simple_mode") => "SIMPLE MODE",
            (_, "factory_base_environment") => "BASE ENVIRONMENT",
            (_, "factory_gpu") => "GPU",
            (_, "factory_change_exe") => "Change EXE",
            (_, "factory_copy_logs") => "Copy logs",
            (_, "factory_engine_parameters") => "ENGINE PARAMETERS",
            (_, "factory_dxvk") => "DXVK",
            (_, "factory_xaudio") => "XAudio",
            (_, "factory_vcrun2022") => "VC++ 2022",
            (_, "factory_d3dx9") => "DirectX 9 (d3dx9)",
            (_, "factory_vcrun2005") => "VC++ 2005",
            (_, "factory_vcrun2008") => "VC++ 2008",
            (_, "factory_physx_support") => "PhysX",
            (_, "factory_xact_legacy") => "XACT (legacy)",
            (_, "factory_prefix_32") => "32-bit Prefix",
            (_, "factory_browse_source") => "BROWSE SOURCE",
            (_, "factory_mount_iso") => "MOUNT ISO",
            (_, "factory_producing") => "PRODUCING...",
            (_, "factory_production") => "PRODUCTION",
            (_, "factory_live_feed") => "LIVE FEED",
            (_, "factory_waiting") => "Waiting...",
            (_, "factory_no_source_selected") => "No source selected",
            (_, "tools_drivers_ok") => "All required system dependencies are available.",
            (_, "tools_system_health") => "SYSTEM HEALTH",
            (_, "tools_wine_proton") => "Wine/Proton",
            (_, "tools_winetricks") => "Winetricks",
            (_, "tools_ntlm") => "NTLM auth",
            (_, "tools_drivers32") => "32-bit libraries",
            (_, "tools_vulkan_runtime") => "Vulkan Runtime",
            (_, "tools_gamemode") => "GameMode",
            (_, "tools_mangohud") => "MangoHUD",
            (_, "tools_utility_actions") => "UTILITY ACTIONS",
            (_, "tools_fix_system") => "FIX SYSTEM",
            (_, "tools_kill_wine") => "KILL ALL WINE",
            (_, "tools_copy_logs") => "Copy logs",
            (_, "tools_save_logs") => "Save logs",
            (_, "tools_analyze") => "Analyze",
            (_, "tools_live_logs") => "LIVE LOGS",
            (_, "tools_dependencies") => "DEPENDENCIES",
            (_, "tools_missing_packages") => "Missing packages",
            (_, "tools_ok") => "OK",
            (_, "tools_fail") => "FAIL",
            (_, "tools_ready") => "READY",
            (_, "tools_missing") => "MISSING",
            (_, "sidebar_tagline") => "GAME FACTORY",
            (_, "cloud_database") => "CLOUD DATABASE",
            (_, "presets_loaded") => "presets loaded",
            (_, "welcome_title") => "WELCOME TO R2L",
            (_, "welcome_subtitle") => "Repack2Linux",
            (_, "welcome_points") => "• Your app icon has been installed to the desktop menu.\n• Community sync and profile learning are active.\n• The production workflow is ready right after startup.",
            (_, "welcome_animation") => "Enable welcome animation",
            (_, "welcome_screen_setting") => "Show welcome screen on startup",
            (_, "welcome_start") => "START USING R2L",
            (_, "export_config_title") => "EXPORT CONFIGURATION",
            (_, "export_config_desc") => "Build a self-contained package and verify the prefix before sharing.",
            (_, "export_pack_options") => "PACKAGING OPTIONS",
            (_, "export_keep_workdir") => "Keep working directory and temporary files",
            (_, "export_scope_title") => "EXPORT SCOPE",
            (_, "export_dry_run_label") => "Dry run - verify prefix without writing",
            (_, "export_target") => "EXPORT TARGET",
            (_, "export_change_folder") => "Change folder...",
            (_, "export_no_folder") => "No folder selected",
            (_, "export_start") => "START EXPORT",
            (_, "export_cancel") => "CANCEL",
            (_, "export_running") => "EXPORT IN PROGRESS",
            (_, "export_progress") => "Operation progress:",
            (_, "export_scope_label") => "Export scope",
            (_, "export_running_dry") => "Dry run: verifying prefix by scan",
            (_, "export_running_full") => "Production mode: package will be created",
            (_, "export_running_note") => "Building a stable offline package and validating prefix.",
            (_, "export_done_title") => "EXPORT COMPLETED",
            (_, "export_done_desc") => "Your package is ready at the selected location.",
            (_, "export_scope_short") => "Scope",
            (_, "export_done_dry") => "Dry run: only prefix integrity was checked",
            (_, "export_done_full") => "Production mode: all components have been saved",
            (_, "export_no_integrity") => "No integrity data available - retry or run dry run.",
            (_, "export_integrity") => "Integrity verification:",
            (_, "export_open_folder") => "OPEN FOLDER",
            (_, "export_close") => "CLOSE",
            (_, "export_error") => "EXPORT ERROR",
            (_, "export_back_to_config") => "BACK TO CONFIGURATION",
            _ => key,
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
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

    // --- SEKCJA HANDLERÓW (MODULARYZACJA) ---

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

            // PANCERNY OPENER TERMINALA
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

    fn current_requirements_from_options(&self) -> detector::GameRequirements {
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

    fn resolve_saved_exe(base_path: &Path, exe_hint: &str) -> Option<PathBuf> {
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

    fn preferred_exe_hint_for_source(&self, source_path: &Path) -> Option<String> {
        let exe = self.game_exe_override.as_ref()?;
        if let Ok(rel) = exe.strip_prefix(source_path) {
            return Some(rel.to_string_lossy().to_string());
        }
        exe.file_name().map(|f| f.to_string_lossy().to_string())
    }

    fn remember_learned_profile(&mut self, source_path: &Path, prefix_path: Option<&Path>) {
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

    fn refresh_community_status(&mut self) {
        let snapshot = community_sync::queue_snapshot();
        self.community_queue_pending = snapshot.pending;
        self.community_queue_attempts = snapshot.attempts;
        self.community_last_retry_at = snapshot.last_attempt_at;
        self.community_last_error = snapshot.last_error;
        self.community_repo_root = snapshot.repo_root;
        self.community_remote_enabled = snapshot.remote_enabled;
    }

    fn set_preset_inspector(
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
                p.file_name().unwrap().to_string_lossy().to_string()
            };
            self.repack_type = info.repack_type.clone();
            self.repack_path = Some(p.clone());
            self.game_exe_override = detector::Detector::find_game_exe(&p);
            self.is_path_dangerous = info.is_path_dangerous;

            let mut reqs = info.requirements.clone();
            let mut db_source = "Detector";
            let mut p_source = ProtonSource::Default;

            // 1. Learned JSON (źródło lub game_id)
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
            // 2. SQLite learned preset (legacy / path-based)
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
                // 3. Check Cloud DB
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

            // AUTO-KONFIGURACJA UI
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

            // AUTO PROTON SELECTION (Jeśli nie ustawiono z chmury)
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
            self.logs.push(format!(
                "[ENGINE] Proton selected from: {:?}",
                self.proton_source
            ));
            println!("[ENGINE] Proton selected from: {:?}", self.proton_source);
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
                self.game_exe_override = Some(f);
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
                self.repack_path = Some(path.parent().unwrap().to_path_buf());
                self.game_name =
                    detector::Detector::clean_name(path.file_stem().unwrap().to_str().unwrap());
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

            // AUTO-KONFIGURACJA UI
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
            self.logs.push(format!(
                "[ENGINE] Proton selected from: {:?}",
                self.proton_source
            ));
            println!("[ENGINE] Proton selected from: {:?}", self.proton_source);
        }
        Task::none()
    }

    fn handle_start_production(&mut self) -> Task<Message> {
        if self.repack_path.is_none() {
            return Task::none();
        }
        self.is_producing = true;
        let source_path = self.repack_path.clone().unwrap();
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
                self.repack_path = Some(PathBuf::from(&path)); // NOW PROJECT IS THE SOURCE
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
        } // FIX DLA NTLM

        let sys_libs = dependencies::DependencyManager::check_system_libs();
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

            // AUTO PROTON SELECTION
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
                // Najpierw sprawdzamy lokalną bazę w projekcie (games.sample.json)
                if let Ok(local_data) = tokio::fs::read_to_string("cloud/games.sample.json").await {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&local_data) {
                        return Ok(json);
                    }
                }

                // Jeśli brak lokalnej, próbujemy z chmury (nowa nazwa + fallback legacy)
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

        // FIX: Obliczamy ścieżkę relatywną EXE względem projektu
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

        Task::stream(async_stream::stream! {
            let final_portable_folder = export_dir.join(format!("{}_Portable", game_name.replace(" ", "_")));
            let final_installer_sh = export_dir.join(format!("{}.sh", game_name.replace(" ", "_")));

            // FIX: Zamykamy Wine przed operacjami na plikach
            let _ = tokio::process::Command::new("wineserver").arg("-k").status().await;
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            yield Message::ProgressUpdated(0.05);

            // 1. Zawsze przygotowujemy folder tymczasowy/docelowy
            let _ = tokio::fs::create_dir_all(&final_portable_folder).await;

            yield Message::LogAppended("[EXPORT] Kopiowanie plików gry...".into());
            let _ = tokio::process::Command::new("cp")
                .arg("-a").arg("--reflink=auto")
                .arg(format!("{}/.", source_dir.to_string_lossy()))
                .arg(&final_portable_folder).status().await;

            yield Message::ProgressUpdated(0.20);
            let inst = installer::Installer::new(&game_name, final_portable_folder.clone());

            let proton_path = if selected_proton.as_deref() == Some("System Wine (Default)") {
                None
            } else {
                crate::proton::ProtonManager::get_path(&selected_proton.clone().unwrap_or_default())
            };

            // Generujemy skrypty w folderze docelowym
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

            let mut export_artifact = ExportArtifact {
                installer_path: final_portable_folder.clone(),
                audits: Vec::new(),
                scope,
                dry_run,
                source_path: source_dir.clone(),
                prefix_path: final_portable_folder.join("pfx"),
            };

            // 2. Jeśli użytkownik chciał instalator .sh
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

                // AUTOMATYCZNA CZYSTKA PROJEKTU (Workdir)
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
                        // Uruchamiamy usuwanie w tle, aby nie blokować UI
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

    fn subscription(&self) -> iced::Subscription<Message> {
        if matches!(self.export_status, ExportStatus::Running(_))
            || (self.show_welcome_overlay && self.cfg.welcome_animation_enabled)
        {
            iced::time::every(std::time::Duration::from_millis(300)).map(|_| Message::Tick)
        } else {
            iced::Subscription::none()
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let sidebar = container(
            column![
                text("R2L").size(38).font(font_bold()).color(ACCENT_CYAN),
                text(self.tr("sidebar_tagline"))
                    .size(10)
                    .color(TEXT_DIM)
                    .font(font_bold()),
                Space::with_height(60),
                sidebar_btn(
                    self,
                    format!("  {}", self.tr("factory")),
                    Tab::Factory,
                    self.current_tab == Tab::Factory
                ),
                sidebar_btn(
                    self,
                    format!("  {}", self.tr("tools")),
                    Tab::Tools,
                    self.current_tab == Tab::Tools
                ),
                sidebar_btn(
                    self,
                    format!("  {}", self.tr("settings")),
                    Tab::Settings,
                    self.current_tab == Tab::Settings
                ),
                Space::with_height(Length::Fill),
                container(column![
                    row![
                        container(Space::with_width(8))
                            .style(|_: &Theme| container::Style {
                                background: Some(Background::Color(Color::from_rgb(0.0, 1.0, 0.4))),
                                border: Border {
                                    radius: 4.0.into(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            })
                            .width(8)
                            .height(8),
                        Space::with_width(8),
                        text(self.tr("cloud_database"))
                            .size(10)
                            .font(font_bold())
                            .color(ACCENT_CYAN),
                    ]
                    .align_y(Alignment::Center),
                    text(format!(
                        "{} {}",
                        self.preset_count,
                        self.tr("presets_loaded")
                    ))
                    .size(9)
                    .color(TEXT_DIM),
                ])
                .padding(10),
                container(text("v1.01").size(9).color(TEXT_DIM)).padding(10)
            ]
            .spacing(16)
            .align_x(Alignment::Start),
        )
        .width(220)
        .height(Length::Fill)
        .padding(24)
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(DEEP_DARK)),
            ..Default::default()
        });

        let main_content = container(match self.current_tab {
            Tab::Factory => ui::factory::view_factory(self),
            Tab::Tools => ui::tools::view_tools(self),
            Tab::Settings => ui::settings::view_settings(self),
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(32);

        let content = row![sidebar, main_content].spacing(0);

        let mut root: Element<'_, Message> = root_background(
            container(content)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(move |_: &Theme| container::Style {
                    background: Some(Background::Color(Color::from_rgb(0.14, 0.12, 0.20))),
                    ..Default::default()
                }),
        )
        .into();

        if self.show_export_modal {
            root = stack![
                root,
                button(
                    container(Space::with_width(Length::Fill))
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .style(|_| container::Style {
                            background: Some(Background::Color(Color::from_rgba(
                                0.0, 0.0, 0.0, 0.7
                            ))),
                            ..Default::default()
                        }),
                )
                .on_press(Message::ModalBackdropClicked)
                .style(|_, _| button::Style {
                    background: Some(Background::Color(Color::TRANSPARENT)),
                    ..Default::default()
                }),
                container(ui::factory::view_export_dialog(self))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
            ]
            .into();
        }

        if self.show_welcome_overlay {
            root = stack![
                root,
                button(
                    container(Space::with_width(Length::Fill))
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .style(|_| container::Style {
                            background: Some(Background::Color(Color::from_rgba(
                                0.0, 0.0, 0.0, 0.78
                            ))),
                            ..Default::default()
                        }),
                )
                .on_press(Message::ModalBackdropClicked)
                .style(|_, _| button::Style {
                    background: Some(Background::Color(Color::TRANSPARENT)),
                    ..Default::default()
                }),
                container(view_welcome_overlay(self))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
            ]
            .into();
        }

        root
    }
}

fn view_welcome_overlay(app: &RepackApp) -> Element<'_, Message> {
    let glow = if app.cfg.welcome_animation_enabled {
        (app.animation_tick % 4) as f32 * 0.04
    } else {
        0.0
    };
    let accent = Color::from_rgb(0.36 + glow, 0.72, 0.98);

    container(
        column![
            text(app.tr("welcome_title"))
                .size(30)
                .font(font_bold())
                .color(accent),
            text(app.tr("welcome_subtitle")).size(13).color(TEXT_DIM),
            Space::with_height(16),
            text(app.tr("welcome_points")).size(12).color(Color::WHITE),
            Space::with_height(18),
            button(
                container(text(app.tr("welcome_start")).size(14).font(font_bold()))
                    .padding(14)
                    .center_x(Length::Fill),
            )
            .on_press(Message::DismissWelcomePressed)
            .style(|t, s| ui::theme::brand_button_style(t, s, true))
            .width(Length::Fill),
        ]
        .spacing(8),
    )
    .padding(28)
    .width(560)
    .style(|_| container::Style {
        background: Some(Background::Color(Color::from_rgba(0.08, 0.09, 0.13, 0.96))),
        border: Border {
            radius: 14.0.into(),
            width: 1.0,
            color: Color::from_rgba(0.36, 0.72, 0.98, 0.45),
        },
        shadow: iced::Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.45),
            offset: iced::Vector::new(0.0, 10.0),
            blur_radius: 28.0,
        },
        ..Default::default()
    })
    .into()
}

fn sidebar_btn(
    _app: &RepackApp,
    label: String,
    tab: Tab,
    active: bool,
) -> Element<'static, Message> {
    let indicator: Element<'static, Message> = if active {
        container(Space::with_width(4))
            .height(20)
            .style(|_: &Theme| container::Style {
                background: Some(Background::Color(ACCENT_CYAN)),
                border: Border {
                    radius: 2.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
    } else {
        Space::with_width(0).into()
    };

    button(
        container(
            row![indicator, text(label).size(13).font(font_bold())]
                .spacing(12)
                .align_y(Alignment::Center),
        )
        .padding(16)
        .center_y(Length::Fill),
    )
    .width(Length::Fill)
    .on_press(Message::TabChanged(tab))
    .style(move |_t, s| {
        let mut style = button::Style::default();
        match s {
            button::Status::Hovered => {
                style.background = Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.05)));
                style.text_color = Color::WHITE;
            }
            _ => {
                style.text_color = if active { Color::WHITE } else { TEXT_DIM };
            }
        }
        style
    })
    .into()
}

fn font_bold() -> font::Font {
    font::Font {
        weight: font::Weight::Bold,
        ..font::Font::DEFAULT
    }
}
