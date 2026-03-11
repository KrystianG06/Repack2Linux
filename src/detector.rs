use pelite::pe32::PeFile as PeFile32;
use pelite::pe64::PeFile;
use regex::Regex;
use std::io::Read;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use pelite::resources::Name;

#[allow(dead_code)]
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

    pub fn extract_icon(exe_path: &Path, output_png: &Path) -> bool {
        // 1. Zawsze czyścimy przed startem, żeby nie było starych śmieci
        if output_png.exists() {
            let _ = std::fs::remove_file(output_png);
        }

        // 2. Próbujemy wyciągnąć ikonę bezpośrednio z EXE za pomocą pelite
        if let Ok(bytes) = std::fs::read(exe_path) {
            let icon_data: Option<Vec<u8>> = match PeFile::from_bytes(&bytes) {
                Ok(pe) => {
                    use pelite::pe64::Pe;
                    pe.resources().ok().and_then(|res| Self::get_best_icon_from_pe(res))
                }
                Err(_) => match PeFile32::from_bytes(&bytes) {
                    Ok(pe) => {
                        use pelite::pe32::Pe;
                        pe.resources().ok().and_then(|res| Self::get_best_icon_from_pe(res))
                    }
                    Err(_) => None,
                },
            };

            if let Some(data) = icon_data.as_ref() {
                // Konwersja na PNG za pomocą biblioteki image, aby uniknąć "śniegu" i błędnych formatów
                if let Ok(img) = image::load_from_memory(&data) {
                    if img.save(output_png).is_ok() {
                        return true;
                    }
                }
            }
        }

        // --- FALLBACK: Jeśli ekstrakcja z EXE zawiodła, szukamy gotowych plików w folderze ---
        let game_dir = match exe_path.parent() {
            Some(p) => p,
            None => return false,
        };
        let common_names = ["icon.png", "icon.ico", "folder.jpg", "app.ico", "UnityPlayer.ico", "UnityPlayer.png"];
        let mut search_paths = vec![game_dir.to_path_buf()];
        
        if let Ok(entries) = std::fs::read_dir(game_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    let name = entry.file_name().to_string_lossy().to_lowercase();
                    if name.ends_with("_data") || name == "resources" || name == "assets" {
                        search_paths.push(entry.path());
                    }
                }
            }
        }

        for path in search_paths {
            for name in common_names {
                let p = path.join(name);
                if p.exists() {
                    if name.ends_with(".png") {
                        // Nawet jeśli to PNG, przepuszczamy przez bibliotekę image dla pewności poprawności
                        if let Ok(img) = image::open(&p) {
                            if img.save(output_png).is_ok() {
                                return true;
                            }
                        }
                    } else if name.ends_with(".ico") {
                        if let Ok(bytes) = std::fs::read(&p) {
                            if let Ok(img) = image::load_from_memory(&bytes) {
                                if img.save(output_png).is_ok() {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }
        false
    }

    fn get_best_icon_from_pe<'a>(resources: pelite::resources::Resources<'a>) -> Option<Vec<u8>> {
        let root = resources.root().ok()?;

        // 1. Znajdujemy katalog grup ikon (ID 14)
        let mut group_type_dir = None;
        for entry in root.entries() {
            if let Ok(name) = entry.name() {
                if name == Name::Id(14) {
                    if let Ok(pelite::resources::Entry::Directory(d)) = entry.entry() {
                        group_type_dir = Some(d);
                        break;
                    }
                }
            }
        }
        let group_type_dir = group_type_dir?;

        let mut best_entry: Option<(u8, u8, u16, u16)> = None; // (w, h, bpp, id)

        // 2. Szukamy najlepszej ikony w grupach
        for entry in group_type_dir.entries() {
            if let Ok(pelite::resources::Entry::Directory(name_dir)) = entry.entry() {
                // Pozycja w katalogu języków (poziom 3)
                if let Some(lang_entry) = name_dir.entries().next() {
                    if let Ok(pelite::resources::Entry::DataEntry(data_entry)) = lang_entry.entry() {
                        if let Ok(data) = data_entry.bytes() {
                            if data.len() < 6 {
                                continue;
                            }
                            let count = u16::from_le_bytes([data[4], data[5]]);
                            let mut offset = 6;
                            for _ in 0..count {
                                if data.len() < offset + 14 {
                                    break;
                                }
                                let w    = data[offset];
                                let h    = data[offset + 1];
                                let bpp  = u16::from_le_bytes([data[offset + 6], data[offset + 7]]);
                                let id   = u16::from_le_bytes([data[offset + 12], data[offset + 13]]);

                                let eff_w = if w == 0 { 256u32 } else { w as u32 };
                                let eff_h = if h == 0 { 256u32 } else { h as u32 };
                                let eff_bpp = if bpp == 0 { 32u32 } else { bpp as u32 };
                                let score = eff_w * eff_h * eff_bpp;

                                if let Some((bw, bh, bbpp, _)) = best_entry {
                                    let beff_w = if bw == 0 { 256u32 } else { bw as u32 };
                                    let beff_h = if bh == 0 { 256u32 } else { bh as u32 };
                                    let beff_bpp = if bbpp == 0 { 32u32 } else { bbpp as u32 };
                                    if score > beff_w * beff_h * beff_bpp {
                                        best_entry = Some((w, h, bpp, id));
                                    }
                                } else {
                                    best_entry = Some((w, h, bpp, id));
                                }
                                offset += 14;
                            }
                        }
                    }
                }
            }
        }

        if let Some((w, h, bpp, id)) = best_entry {
            // 3. Pobieramy dane konkretnej ikony (Typ 3) o ID wyciągniętym z grupy
            let mut icon_type_dir = None;
            for entry in root.entries() {
                if let Ok(name) = entry.name() {
                    if name == Name::Id(3) {
                        if let Ok(pelite::resources::Entry::Directory(d)) = entry.entry() {
                            icon_type_dir = Some(d);
                            break;
                        }
                    }
                }
            }
            let icon_type_dir = icon_type_dir?;

            let mut icon_name_dir = None;
            for entry in icon_type_dir.entries() {
                if let Ok(name) = entry.name() {
                    if name == Name::Id(id as u32) {
                        if let Ok(pelite::resources::Entry::Directory(d)) = entry.entry() {
                            icon_name_dir = Some(d);
                            break;
                        }
                    }
                }
            }
            let icon_name_dir = icon_name_dir?;

            if let Some(lang_entry) = icon_name_dir.entries().next() {
                if let Ok(pelite::resources::Entry::DataEntry(data_entry)) = lang_entry.entry() {
                    if let Ok(icon_data) = data_entry.bytes() {
                        if icon_data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
                            return Some(icon_data.to_vec());
                        }

                        let mut ico = Vec::with_capacity(22 + icon_data.len());
                        ico.extend_from_slice(&[0, 0]); // Reserved
                        ico.extend_from_slice(&[1, 0]); // Type (1 for icon)
                        ico.extend_from_slice(&[1, 0]); // Count (1 icon)
                        ico.push(w);
                        ico.push(h);
                        ico.push(0); // Color count
                        ico.push(0); // Reserved
                        ico.extend_from_slice(&1u16.to_le_bytes()); // Planes
                        ico.extend_from_slice(&bpp.to_le_bytes()); // BPP
                        ico.extend_from_slice(&(icon_data.len() as u32).to_le_bytes());
                        ico.extend_from_slice(&22u32.to_le_bytes()); // Offset
                        ico.extend_from_slice(icon_data);
                        return Some(ico);
                    }
                }
            }
        }
        None
    }
}
