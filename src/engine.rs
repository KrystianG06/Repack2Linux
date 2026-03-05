use crate::{dependencies, detector, installer, presets::GamePresets, shortcuts, Message};
use async_stream::stream;
use futures_core::stream::Stream;
use std::path::PathBuf;
use std::process::Stdio;
use walkdir::WalkDir;

pub struct Engine;

#[derive(Debug, Clone, Default)]
pub struct ProductionOptions {
    pub dxvk: bool,
    pub vcrun: bool,
    pub win32: bool,
    pub ultra_compat: bool,
    pub mangohud: bool,
    pub gamemode: bool,
    pub no_dxvk: bool,
    pub legacy_mode: bool,
    pub d3dx9: bool,
    pub vcrun2005: bool,
    pub vcrun2008: bool,
    pub physx: bool,
    pub xact: bool,
}

impl Engine {
    pub fn run_production(
        game_name: String,
        source_path: PathBuf,
        project_dir: PathBuf,
        _config: crate::config::AppConfig,
        options: ProductionOptions,
        selected_proton: Option<String>,
        exe_override: Option<PathBuf>,
        gpu: String,
    ) -> impl Stream<Item = Message> {
        stream! {
            yield Message::LogAppended("[FACTORY] Initiating Project-Based Pipeline...".into());
            yield Message::ProgressUpdated(0.01);

            // 1. INTELIGENTNY I SZYBKI IMPORT DO PROJEKTU
            yield Message::LogAppended(format!("[IMPORT] Analysing game data: {}...", source_path.display()));

            let mut files_to_copy = Vec::new();
            for entry in WalkDir::new(&source_path).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() {
                    files_to_copy.push(entry.path().to_path_buf());
                }
            }
            let total_files = files_to_copy.len();
            yield Message::LogAppended(format!("[IMPORT] Found {} files to sync.", total_files));

            if let Err(e) = tokio::fs::create_dir_all(&project_dir).await {
                yield Message::LogAppended(format!("[ERROR] Could not create project directory: {}", e));
                yield Message::ProductionFinished(Err(e.to_string()));
                return;
            }

            let mut copied_count = 0;
            let mut last_reported_p = 0.0;

            for file in files_to_copy {
                if let Ok(rel_path) = file.strip_prefix(&source_path) {
                    let target_path = project_dir.join(rel_path);
                    if let Some(parent) = target_path.parent() {
                        if let Err(e) = tokio::fs::create_dir_all(parent).await {
                            yield Message::LogAppended(format!("[ERROR] FS Failure: {}", e));
                            yield Message::ProductionFinished(Err(e.to_string()));
                            return;
                        }
                    }

                    if let Err(e) = tokio::fs::copy(&file, &target_path).await {
                        yield Message::LogAppended(format!("[ERROR] Copy failed for {:?}: {}", file.file_name().unwrap_or_default(), e));
                        yield Message::ProductionFinished(Err(format!("Copy failed: {}", e)));
                        return;
                    }

                    copied_count += 1;
                    let current_p = (copied_count as f32 / total_files as f32) * 0.4;
                    if current_p - last_reported_p >= 0.01 || copied_count == total_files {
                        yield Message::ProgressUpdated(0.01 + current_p);
                        last_reported_p = current_p;
                        if copied_count % 1000 == 0 {
                            yield Message::LogAppended(format!("[IMPORT] Progress: {}/{} files...", copied_count, total_files));
                        }
                    }
                }
            }

            yield Message::LogAppended("[IMPORT] Project files are now self-sufficient.".into());
            yield Message::ProgressUpdated(0.45);

            // 2. KONFIGURACJA WINE
            let install_dir = project_dir.clone();
            let inst = installer::Installer::new(&game_name, install_dir.clone());

            if let Some(preset) = GamePresets::get_preset(&game_name) {
                yield Message::LogAppended(format!("[AUTO] Detected preset: {} applied.", preset.name));
            }

            yield Message::LogAppended(format!("[CORE] Building {:?} Sandbox...", if options.win32 { "Win32" } else { "Win64" }));
            yield Message::ProgressUpdated(0.5);

            // PANCERNA KONFIGURACJA PREFIXU
            match inst.prepare_prefix_ext(options.win32).await {
                Ok(Some(p)) => {
                    yield Message::LogAppended(format!("[PREFIX] Reused prefix from {}", p.display()));
                }
                Ok(None) => {}
                Err(e) => {
                    yield Message::LogAppended(format!("[ERROR] Prefix configuration failed: {}", e));
                    yield Message::ProductionFinished(Err(format!("Wine environment setup failed: {}", e)));
                    return;
                }
            }

            if let Err(e) = inst.isolate_saves().await {
                yield Message::LogAppended(format!("[WARN] Saves isolation failed (non-critical): {}", e));
            }
            yield Message::ProgressUpdated(0.6);

            let mut deps = vec![];
            if (options.dxvk || options.ultra_compat) && !options.no_dxvk { deps.extend(vec!["dxvk", "d3dx9", "d3dcompiler_43", "d3dcompiler_47"]); }
            if options.vcrun || options.ultra_compat { deps.push("vcrun2022"); }
            if options.legacy_mode || options.d3dx9 { deps.push("d3dx9"); }
            if options.legacy_mode || options.vcrun2005 { deps.push("vcrun2005"); }
            if options.legacy_mode || options.vcrun2008 { deps.push("vcrun2008"); }
            if options.physx { deps.push("physx"); }
            if options.xact { deps.push("xact"); }

            if !deps.is_empty() {
                yield Message::LogAppended(format!("[ENGINE] Injecting dependencies: {:?}", deps));
                yield Message::ProgressUpdated(0.7);
                if let Err(e) = dependencies::DependencyManager::install(&inst.prefix_path, deps).await {
                    yield Message::LogAppended(format!("[ERROR] Dependency injection failed: {}", e));
                    yield Message::ProductionFinished(Err(format!("Dependency error: {}", e)));
                    return;
                }
            }
            yield Message::ProgressUpdated(0.85);

            // 3. FINALIZACJA I URUCHOMIENIE
            let final_exe = if let Some(over) = exe_override {
                if let Ok(rel) = over.strip_prefix(&source_path) { install_dir.join(rel) } else { over }
            } else {
                detector::Detector::find_game_exe(&install_dir).unwrap_or_default()
            };

            if final_exe.exists() {
                let rel = final_exe.strip_prefix(&install_dir).unwrap_or(&final_exe);
                let rel_str = rel.to_string_lossy();
                let proton_path = if selected_proton.as_deref() == Some("System Wine (Default)") { None } else { crate::proton::ProtonManager::get_path(&selected_proton.unwrap_or_default()) };

                let (dll_overrides, env_vars) = if let Some(preset) = GamePresets::get_preset(&game_name) {
                    (Some(preset.dll_overrides.to_string()), Some(preset.env_vars))
                } else { (None, None) };

                if let Err(e) = inst.generate_portable_script_ext(&rel_str, options.mangohud, options.gamemode, options.no_dxvk, proton_path, &gpu, dll_overrides, env_vars).await {
                    yield Message::LogAppended(format!("[ERROR] Script generation failed: {}", e));
                    yield Message::ProductionFinished(Err(format!("Loader generation failed: {}", e)));
                    return;
                }

                if let Err(e) = inst.generate_readme().await {
                    yield Message::LogAppended(format!("[WARN] Readme generation failed: {}", e));
                }

                // EKSTRAKCJA IKONY
                yield Message::LogAppended("[ASSETS] Looking for game icon...".into());
                let icon_path = install_dir.join("icon.png");
                let _ = detector::Detector::extract_icon(&install_dir, &icon_path);

                yield Message::ProgressUpdated(0.95);

                if let Err(e) = shortcuts::ShortcutManager::create_desktop_shortcut(
                    &game_name, &final_exe.to_string_lossy(), &inst.prefix_path.to_string_lossy(), None, options.mangohud, options.gamemode
                ) {
                    yield Message::LogAppended(format!("[WARN] Shortcut creation failed: {}", e));
                }

                yield Message::LogAppended("[LIVE] Starting game from project workdir...".into());

                let debug_cmd = format!(
                    "cd '{}' && WINEDEBUG=-all,err+all WINEPREFIX='{}' wine '{}'",
                    install_dir.display(), inst.prefix_path.display(), final_exe.display()
                );

                let mut cmd = tokio::process::Command::new("bash");
                cmd.arg("-c")
                    .arg(&debug_cmd)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());

                match cmd.spawn() {
                    Ok(_child) => {
                        yield Message::LogAppended("[LIVE] Game started in background. UI unlocked.".into());
                    }
                    Err(e) => {
                        yield Message::LogAppended(format!("[ERROR] Failed to start: {}", e));
                    }
                }
                yield Message::ProductionFinished(Ok(install_dir.to_string_lossy().to_string()));
            } else {
                yield Message::ProductionFinished(Err("Binary not found in project!".into()));
            }
        }
    }
}
