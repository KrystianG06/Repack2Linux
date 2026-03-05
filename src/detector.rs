use pelite::pe32::PeFile as PeFile32;
use pelite::pe64::PeFile;
use regex::Regex;
use std::io::Read;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, PartialEq)]
pub enum RepackType {
    FitGirl,
    DODI,
    GOG,
    Classic,
    AlreadyWrapped,
    Unknown,
}

#[derive(Debug, Clone, Default)]
pub struct GameRequirements {
    pub needs_dxvk: bool,
    pub needs_xaudio: bool,
    pub needs_vcrun: bool,
    pub is_64bit: bool,
    pub engine_type: String,
    pub engine_version: Option<String>,
    // ROZSZERZONE PARAMETRY
    pub needs_d3dx9: bool,
    pub needs_vcrun2005: bool,
    pub needs_vcrun2008: bool,
    pub needs_physx: bool,
    pub needs_xact: bool,
    pub has_anticheat: bool,
}

#[derive(Debug, Clone)]
pub struct GameInfo {
    pub clean_name: String,
    pub repack_type: RepackType,
    pub is_64bit: bool,
    pub suggested_dx: u8,
    pub requirements: GameRequirements,
    pub is_path_dangerous: bool,
}

pub struct Detector;

impl Detector {
    pub fn detect(path: &Path) -> GameInfo {
        let best_exe = Self::find_game_exe(path);
        let raw_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let is_path_dangerous =
            raw_name.contains(' ') || raw_name.contains('/') || raw_name.contains('\'');

        let clean_name = Self::clean_name(&raw_name);
        let repack_type = Self::identify_repack(path);

        // GŁĘBOKA ANALIZA FOLDERU
        let (engine, version, extra_reqs) = Self::deep_scan_folder(path);

        let mut reqs = extra_reqs;
        reqs.engine_type = engine;
        reqs.engine_version = version;

        if let Some(exe) = &best_exe {
            if let Ok(r) = Self::analyze_binary(exe) {
                reqs.is_64bit = r.is_64bit;
                reqs.needs_dxvk = reqs.needs_dxvk || r.needs_dxvk;
                reqs.needs_xaudio = reqs.needs_xaudio || r.needs_xaudio;
                reqs.needs_vcrun = reqs.needs_vcrun || r.needs_vcrun;
                if !r.is_64bit {
                    reqs.needs_d3dx9 = true;
                }
            }
        }

        GameInfo {
            clean_name,
            repack_type,
            is_64bit: reqs.is_64bit,
            suggested_dx: if reqs.needs_dxvk { 11 } else { 9 },
            requirements: reqs,
            is_path_dangerous,
        }
    }

    fn deep_scan_folder(path: &Path) -> (String, Option<String>, GameRequirements) {
        let mut engine = "Generic".to_string();
        let mut version = None;
        let mut reqs = GameRequirements::default();

        let walker = WalkDir::new(path).max_depth(4).into_iter();
        for entry in walker.filter_map(|e| e.ok()) {
            let fname = entry.file_name().to_string_lossy().to_lowercase();

            // 1. SILNIKI (Precyzyjnie)
            if fname == "unityplayer.dll" || fname == "unity main" {
                engine = "Unity".into();
                // Spróbuj odczytać wersję z globalgamemanagers lub pliku exe
            }
            if fname.contains("unreal") || fname.ends_with(".uasset") {
                engine = "Unreal".into();
                if entry.path().to_string_lossy().contains("UE4") {
                    version = Some("4".into());
                }
                if entry.path().to_string_lossy().contains("UE5") {
                    version = Some("5".into());
                }
            }
            if fname == "engine.dll" && entry.path().to_string_lossy().contains("bin") {
                engine = "Source".into();
            }

            // 2. ANTI-CHEAT
            if fname.contains("easyanticheat") || fname.contains("eac") {
                reqs.has_anticheat = true;
            }
            if fname.contains("battleye") {
                reqs.has_anticheat = true;
            }

            // 3. ANALIZA PLIKÓW KONFIGUURACYJNYCH (.ini / .xml)
            if fname.ends_with(".ini") || fname.ends_with(".cfg") {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    let content_lower = content.to_lowercase();
                    if content_lower.contains("directx 9") || content_lower.contains("dx9") {
                        reqs.needs_d3dx9 = true;
                    }
                    if content_lower.contains("physx") {
                        reqs.needs_physx = true;
                    }
                    if content_lower.contains("xaudio") {
                        reqs.needs_xaudio = true;
                    }
                }
            }

            // 4. DETEKCJA DLL (W folderze gry)
            if fname == "physxloader.dll" || fname.contains("physx") {
                reqs.needs_physx = true;
            }
            if fname == "d3dx9_43.dll" {
                reqs.needs_d3dx9 = true;
            }
            if fname.contains("msvcp140") {
                reqs.needs_vcrun = true;
            }
            if fname.contains("msvcp80") {
                reqs.needs_vcrun2005 = true;
            }
            if fname.contains("msvcp90") {
                reqs.needs_vcrun2008 = true;
            }
        }

        (engine, version, reqs)
    }

    fn analyze_binary(path: &Path) -> Result<GameRequirements, String> {
        let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let mut buffer = vec![0; 1024 * 1024];
        let bytes_read = file.read(&mut buffer).map_err(|e| e.to_string())?;
        buffer.truncate(bytes_read);

        let mut reqs = GameRequirements::default();
        if PeFile::from_bytes(&buffer).is_ok() {
            reqs.is_64bit = true;
        } else if PeFile32::from_bytes(&buffer).is_ok() {
            reqs.is_64bit = false;
        }

        let s = String::from_utf8_lossy(&buffer).to_lowercase();
        reqs.needs_dxvk = s.contains("d3d11.dll")
            || s.contains("d3d10")
            || s.contains("dxgi.dll")
            || s.contains("d3d12.dll");
        reqs.needs_xaudio = s.contains("x3daudio") || s.contains("xaudio2");
        reqs.needs_vcrun = s.contains("msvcp") || s.contains("vcruntime");

        if !reqs.is_64bit {
            reqs.needs_d3dx9 = s.contains("d3d9.dll");
            reqs.needs_vcrun2005 = s.contains("msvcp80.dll");
            reqs.needs_vcrun2008 = s.contains("msvcp90.dll");
        }

        Ok(reqs)
    }

    fn identify_repack(path: &Path) -> RepackType {
        if path.join("play.sh").exists() && path.join("pfx").exists() {
            return RepackType::AlreadyWrapped;
        }
        let walker = WalkDir::new(path).max_depth(2).into_iter();
        for entry in walker.filter_map(|e| e.ok()) {
            let fname = entry.file_name().to_string_lossy().to_lowercase();
            if fname.starts_with("fg-0") {
                return RepackType::FitGirl;
            }
            if fname.contains("dodi") {
                return RepackType::DODI;
            }
        }
        RepackType::Unknown
    }

    pub fn clean_name(raw: &str) -> String {
        let mut name = raw.replace("_", " ").replace(".", " ");
        let re_brackets = Regex::new(r"\[.*?\]|\(.*?\)").unwrap();
        name = re_brackets.replace_all(&name, "").to_string();
        let re_tags =
            Regex::new(r"(?i)(fitgirl|repack|dodi|xatab|codex|flt|v\d+|setup|install|steamrip)")
                .unwrap();
        name = re_tags.replace_all(&name, "").to_string();
        name.split_whitespace().collect::<Vec<&str>>().join(" ")
    }

    pub fn find_game_exe(install_dir: &Path) -> Option<PathBuf> {
        let folder_name = install_dir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        let walker = WalkDir::new(install_dir).max_depth(4).into_iter();
        let mut candidates: Vec<(PathBuf, i32)> = Vec::new();

        for entry in walker.filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                let fname = entry.file_name().to_string_lossy().to_lowercase();
                if fname.ends_with(".exe") {
                    let mut score = 0;
                    if fname == format!("{}.exe", folder_name) {
                        score += 100;
                    }
                    if fname.contains("launch") || fname.contains("start") {
                        score += 50;
                    }
                    if fname.contains("shipping") || fname.contains("game") {
                        score += 30;
                    }
                    if fname.contains("setup")
                        || fname.contains("install")
                        || fname.contains("unins")
                    {
                        score -= 200;
                    }
                    candidates.push((entry.path().to_path_buf(), score));
                }
            }
        }
        candidates.sort_by(|a, b| b.1.cmp(&a.1));
        candidates.first().map(|c| c.0.clone())
    }

    pub fn extract_icon(game_dir: &Path, output_png: &Path) -> bool {
        if let Some(exe) = Self::find_game_exe(game_dir) {
            let _ = std::process::Command::new("wrestool")
                .arg("-x")
                .arg("-t")
                .arg("14")
                .arg("-o")
                .arg(output_png)
                .arg(&exe)
                .status();
            return output_png.exists();
        }
        false
    }
}
