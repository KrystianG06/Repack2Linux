use crate::detector::GameRequirements;
use crate::export::ExportScope;
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub struct Database {
    conn: Connection,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct LearnedRequirements {
    needs_dxvk: bool,
    needs_xaudio: bool,
    needs_vcrun: bool,
    is_64bit: bool,
    needs_d3dx9: bool,
    needs_vcrun2005: bool,
    needs_vcrun2008: bool,
    needs_physx: bool,
    needs_xact: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct LearnedProfileEntry {
    game_id: String,
    game_name: String,
    source_paths: Vec<String>,
    preferred_exe: Option<String>,
    selected_proton: Option<String>,
    prefix_path: Option<String>,
    requirements: LearnedRequirements,
    success_count: u32,
    last_success_at: String,
    #[serde(default)]
    gpu_variants: HashMap<String, LearnedGpuVariant>,
    #[serde(default)]
    history: Vec<LearnedHistoryEntry>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct LearnedGpuVariant {
    preferred_exe: Option<String>,
    selected_proton: Option<String>,
    prefix_path: Option<String>,
    requirements: LearnedRequirements,
    success_count: u32,
    last_success_at: String,
    #[serde(default)]
    history: Vec<LearnedHistoryEntry>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct LearnedHistoryEntry {
    saved_at: String,
    preferred_exe: Option<String>,
    selected_proton: Option<String>,
    prefix_path: Option<String>,
    requirements: LearnedRequirements,
    success_count: u32,
}

#[derive(Debug)]
pub struct GameProfile {
    pub game_id: String,
    pub game_name: String,
    pub source_path: String,
    pub prefix_path: String,
    pub selected_proton: Option<String>,
    pub export_scope: ExportScope,
    pub skip_cleanup: bool,
    pub export_installer: bool,
    pub export_standalone: bool,
    pub dry_run: bool,
    pub audit_hash: String,
    pub last_exported: String,
}

impl Database {
    fn app_config_dir() -> PathBuf {
        let mut dir = dirs::config_dir().unwrap_or(PathBuf::from("."));
        dir.push("repack2linux");
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    fn legacy_config_dir() -> PathBuf {
        let mut dir = dirs::config_dir().unwrap_or(PathBuf::from("."));
        dir.push("repack2proton");
        dir
    }

    pub fn new() -> Self {
        let mut db_path = Self::app_config_dir();
        db_path.push("factory.db");
        if !db_path.exists() {
            let mut legacy = Self::legacy_config_dir();
            legacy.push("factory.db");
            if legacy.exists() {
                let _ = std::fs::copy(&legacy, &db_path);
            }
        }

        let conn = Connection::open(db_path).expect("Failed to open SQLite database");
        Self::ensure_schema(&conn).expect("Failed to ensure database schema");
        let db = Self { conn };
        db.seed_knowledge();
        db
    }

    fn ensure_schema(conn: &Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS game_presets (
                folder_path TEXT PRIMARY KEY,
                game_name TEXT,
                preferred_exe TEXT,
                needs_dxvk INTEGER,
                needs_xaudio INTEGER,
                needs_vcrun INTEGER,
                is_64bit INTEGER,
                needs_d3dx9 INTEGER,
                needs_vcrun2005 INTEGER,
                needs_vcrun2008 INTEGER,
                needs_physx INTEGER,
                needs_xact INTEGER
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS cloud_presets (
                game_id TEXT PRIMARY KEY,
                name TEXT,
                needs_dxvk INTEGER,
                needs_xaudio INTEGER,
                needs_vcrun INTEGER,
                is_64bit INTEGER,
                suggested_proton TEXT,
                preferred_exe TEXT,
                needs_d3dx9 INTEGER,
                needs_vcrun2005 INTEGER,
                needs_vcrun2008 INTEGER,
                needs_physx INTEGER,
                needs_xact INTEGER
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS game_profiles (
                source_path TEXT PRIMARY KEY,
                game_id TEXT,
                game_name TEXT,
                prefix_path TEXT,
                selected_proton TEXT,
                last_scope TEXT,
                skip_cleanup INTEGER,
                export_installer INTEGER,
                export_standalone INTEGER,
                dry_run INTEGER,
                audit_hash TEXT,
                last_exported TEXT
            )",
            [],
        )?;

        Self::apply_migrations(conn)?;
        Ok(())
    }

    fn apply_migrations(conn: &Connection) -> Result<()> {
        let version: i32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version < 1 {
            Self::ensure_columns(
                conn,
                "game_presets",
                &[
                    ("needs_physx", "needs_physx INTEGER DEFAULT 0"),
                    ("needs_xact", "needs_xact INTEGER DEFAULT 0"),
                    ("needs_vcrun2005", "needs_vcrun2005 INTEGER DEFAULT 0"),
                    ("needs_vcrun2008", "needs_vcrun2008 INTEGER DEFAULT 0"),
                ],
            )?;
            Self::ensure_columns(
                conn,
                "cloud_presets",
                &[
                    ("needs_physx", "needs_physx INTEGER DEFAULT 0"),
                    ("needs_xact", "needs_xact INTEGER DEFAULT 0"),
                    ("needs_vcrun2005", "needs_vcrun2005 INTEGER DEFAULT 0"),
                    ("needs_vcrun2008", "needs_vcrun2008 INTEGER DEFAULT 0"),
                ],
            )?;
            conn.pragma_update(None, "user_version", &1)?;
        }
        Ok(())
    }

    fn ensure_columns(conn: &Connection, table: &str, columns: &[(&str, &str)]) -> Result<()> {
        let existing_cols = Self::table_columns(conn, table)?;
        for (name, definition) in columns {
            if !existing_cols.contains(*name) {
                conn.execute(
                    &format!("ALTER TABLE {} ADD COLUMN {}", table, definition),
                    [],
                )?;
            }
        }
        Ok(())
    }

    fn table_columns(conn: &Connection, table: &str) -> Result<std::collections::HashSet<String>> {
        let mut cols = std::collections::HashSet::new();
        let mut stmt = conn.prepare(&format!("PRAGMA table_info('{}')", table))?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let name: String = row.get(1)?;
            cols.insert(name);
        }
        Ok(cols)
    }

    pub fn update_cloud_database(&self, full_data: serde_json::Value) -> usize {
        let mut count = 0;
        let games_array = if let Some(arr) = full_data.as_array() {
            Some(arr)
        } else {
            full_data["games"].as_array()
        };

        if let Some(games) = games_array {
            for item in games {
                let name = item["name"]
                    .as_str()
                    .or(item["game_name"].as_str())
                    .unwrap_or_default();
                if name.is_empty() {
                    continue;
                }

                let res = self.conn.execute(
                    "INSERT OR REPLACE INTO cloud_presets (
                        game_id, name, needs_dxvk, needs_xaudio, needs_vcrun, is_64bit, suggested_proton, preferred_exe,
                        needs_d3dx9, needs_vcrun2005, needs_vcrun2008, needs_physx, needs_xact
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    params![
                        item["id"].as_str().unwrap_or(name),
                        name,
                        item["dxvk"].as_bool().or(item["needs_dxvk"].as_bool()).unwrap_or(true) as i32,
                        item["xaudio"].as_bool().or(item["needs_xaudio"].as_bool()).unwrap_or(false) as i32,
                        item["vcrun"].as_bool().or(item["needs_vcrun"].as_bool()).unwrap_or(true) as i32,
                        item["is_64bit"].as_bool().unwrap_or(true) as i32,
                        item["proton"].as_str().or(item["recommended_proton"].as_str()).unwrap_or("GE-Proton"),
                        item["preferred_exe"].as_str().unwrap_or_default(),
                        item["d3dx9"].as_bool().or(item["needs_d3dx9"].as_bool()).unwrap_or(false) as i32,
                        item["vcrun2005"].as_bool().or(item["needs_vcrun2005"].as_bool()).unwrap_or(false) as i32,
                        item["vcrun2008"].as_bool().or(item["needs_vcrun2008"].as_bool()).unwrap_or(false) as i32,
                        item["physx"].as_bool().or(item["needs_physx"].as_bool()).unwrap_or(false) as i32,
                        item["xact"].as_bool().or(item["needs_xact"].as_bool()).unwrap_or(false) as i32,
                    ]
                );
                if res.is_ok() {
                    count += 1;
                }
            }
        }
        count
    }

    pub fn count_cloud_presets(&self) -> usize {
        self.conn
            .query_row("SELECT COUNT(*) FROM cloud_presets", [], |r| r.get(0))
            .unwrap_or(0)
    }

    fn normalize_name(name: &str) -> String {
        name.to_lowercase()
            .replace("gta", "grand theft auto")
            .replace("4", "iv")
            .replace("3", "iii")
            .replace("2", "ii")
            .replace("5", "v")
            .replace(":", "")
            .replace("-", " ")
            .replace("_", " ")
            .split_whitespace()
            .filter(|w| {
                ![
                    "repack",
                    "fitgirl",
                    "dodi",
                    "edition",
                    "complete",
                    "remastered",
                    "v1",
                    "setup",
                ]
                .contains(w)
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn find_cloud_preset(
        &self,
        search_name: &str,
    ) -> Option<(
        crate::detector::GameRequirements,
        Option<String>,
        Option<String>,
        String,
        u8,
    )> {
        let search_norm = Self::normalize_name(search_name);

        let mut stmt = self.conn.prepare(
            "SELECT needs_dxvk, needs_xaudio, needs_vcrun, is_64bit, preferred_exe, suggested_proton, name,
                    needs_d3dx9, needs_vcrun2005, needs_vcrun2008, needs_physx, needs_xact 
             FROM cloud_presets"
        ).ok()?;

        let mut rows = stmt.query([]).ok()?;
        let mut best_match: Option<(
            crate::detector::GameRequirements,
            Option<String>,
            Option<String>,
            String,
            u8,
        )> = None;
        let mut best_score = 0;

        while let Ok(Some(row)) = rows.next() {
            let db_name: String = row.get::<usize, String>(6).unwrap_or_default();
            let db_norm = Self::normalize_name(&db_name);

            let mut score = 0;
            if db_norm == search_norm {
                score = 100;
            } else if db_norm.contains(&search_norm) || search_norm.contains(&db_norm) {
                score = 50;
            }

            if score > best_score {
                best_score = score;
                let reqs = GameRequirements {
                    needs_dxvk: row.get::<usize, i32>(0).unwrap_or(1) != 0,
                    needs_xaudio: row.get::<usize, i32>(1).unwrap_or(0) != 0,
                    needs_vcrun: row.get::<usize, i32>(2).unwrap_or(1) != 0,
                    is_64bit: row.get::<usize, i32>(3).unwrap_or(1) != 0,
                    needs_d3dx9: row.get::<usize, i32>(7).unwrap_or(0) != 0,
                    needs_vcrun2005: row.get::<usize, i32>(8).unwrap_or(0) != 0,
                    needs_vcrun2008: row.get::<usize, i32>(9).unwrap_or(0) != 0,
                    needs_physx: row.get::<usize, i32>(10).unwrap_or(0) != 0,
                    needs_xact: row.get::<usize, i32>(11).unwrap_or(0) != 0,
                    engine_type: "Cloud_DB".into(),
                    engine_version: None,
                    has_anticheat: false,
                };
                let exe: Option<String> = row.get(4).ok();
                let proton: Option<String> = row.get(5).ok();
                best_match = Some((reqs, exe, proton, db_name, score as u8));
            }
        }
        best_match
    }

    fn seed_knowledge(&self) {
        let _ = self.conn.execute(
            "INSERT OR IGNORE INTO cloud_presets (
                game_id, name, needs_dxvk, needs_xaudio, needs_vcrun, is_64bit, suggested_proton, preferred_exe,
                needs_d3dx9, needs_vcrun2005, needs_vcrun2008, needs_physx, needs_xact
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params!["CCD_INTERNAL", "City Car Driving", 1, 1, 1, 0, "System Wine (Default)", "starter.exe", 1, 1, 1, 1, 1]
        );
    }

    pub fn get_preset(&self, folder_path: &str) -> Option<(GameRequirements, Option<String>)> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT needs_dxvk, needs_xaudio, needs_vcrun, is_64bit, preferred_exe,
                    needs_d3dx9, needs_vcrun2005, needs_vcrun2008, needs_physx, needs_xact 
             FROM game_presets WHERE folder_path = ?",
            )
            .ok()?;

        let res = stmt.query_row(params![folder_path], |row| {
            let reqs = GameRequirements {
                needs_dxvk: row.get::<usize, i32>(0)? != 0,
                needs_xaudio: row.get::<usize, i32>(1)? != 0,
                needs_vcrun: row.get::<usize, i32>(2)? != 0,
                is_64bit: row.get::<usize, i32>(3)? != 0,
                needs_d3dx9: row.get::<usize, i32>(5)? != 0,
                needs_vcrun2005: row.get::<usize, i32>(6)? != 0,
                needs_vcrun2008: row.get::<usize, i32>(7)? != 0,
                needs_physx: row.get::<usize, i32>(8)? != 0,
                needs_xact: row.get::<usize, i32>(9)? != 0,
                engine_type: "Learned".into(),
                engine_version: None,
                has_anticheat: false,
            };
            let exe: Option<String> = row.get(4).ok();
            Ok((reqs, exe))
        });
        res.ok()
    }

    pub fn save_preset(
        &self,
        folder_path: &str,
        name: &str,
        preferred_exe: Option<&str>,
        reqs: &GameRequirements,
    ) {
        let _ = self.conn.execute(
            "INSERT OR REPLACE INTO game_presets (
                folder_path, game_name, preferred_exe, needs_dxvk, needs_xaudio, needs_vcrun, is_64bit,
                needs_d3dx9, needs_vcrun2005, needs_vcrun2008, needs_physx, needs_xact
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                folder_path, name, preferred_exe,
                reqs.needs_dxvk as i32, reqs.needs_xaudio as i32, reqs.needs_vcrun as i32, reqs.is_64bit as i32,
                reqs.needs_d3dx9 as i32, reqs.needs_vcrun2005 as i32, reqs.needs_vcrun2008 as i32,
                reqs.needs_physx as i32, reqs.needs_xact as i32,
            ],
        );
    }

    pub fn save_game_profile(&self, profile: &GameProfile) {
        let _ = self.conn.execute(
            "INSERT OR REPLACE INTO game_profiles (
                source_path, game_id, game_name, prefix_path, selected_proton,
                last_scope, skip_cleanup, export_installer, export_standalone, dry_run,
                audit_hash, last_exported
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                profile.source_path,
                profile.game_id,
                profile.game_name,
                profile.prefix_path,
                profile.selected_proton.as_deref().unwrap_or_default(),
                Self::scope_to_text(profile.export_scope),
                profile.skip_cleanup as i32,
                profile.export_installer as i32,
                profile.export_standalone as i32,
                profile.dry_run as i32,
                profile.audit_hash,
                profile.last_exported,
            ],
        );
    }

    pub fn load_game_profile(&self, source_path: &str) -> Option<GameProfile> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT game_id, game_name, prefix_path, selected_proton, last_scope, skip_cleanup,
                        export_installer, export_standalone, dry_run, audit_hash, last_exported
                 FROM game_profiles WHERE source_path = ?",
            )
            .ok()?;

        let row = stmt.query_row(params![source_path], |row| {
            let scope_text: String = row.get(4)?;
            Ok(GameProfile {
                game_id: row.get(0)?,
                game_name: row.get(1)?,
                source_path: source_path.to_string(),
                prefix_path: row.get(2)?,
                selected_proton: row.get::<_, String>(3).ok().and_then(|s| {
                    if s.is_empty() {
                        None
                    } else {
                        Some(s)
                    }
                }),
                export_scope: Self::scope_from_text(&scope_text),
                skip_cleanup: row.get::<_, i32>(5)? != 0,
                export_installer: row.get::<_, i32>(6)? != 0,
                export_standalone: row.get::<_, i32>(7)? != 0,
                dry_run: row.get::<_, i32>(8)? != 0,
                audit_hash: row.get(9)?,
                last_exported: row.get(10)?,
            })
        });
        row.ok()
    }

    pub fn normalize_game_id(name: &str) -> String {
        Self::normalize_name(name)
    }

    fn learned_profiles_file() -> PathBuf {
        let mut db_path = Self::app_config_dir();
        db_path.push("learned-profiles.json");
        if !db_path.exists() {
            let mut legacy = Self::legacy_config_dir();
            legacy.push("learned-profiles.json");
            if legacy.exists() {
                let _ = fs::copy(&legacy, &db_path);
            }
        }
        db_path
    }

    fn load_learned_profiles_map() -> HashMap<String, LearnedProfileEntry> {
        let path = Self::learned_profiles_file();
        let Ok(payload) = fs::read_to_string(path) else {
            return HashMap::new();
        };
        serde_json::from_str::<HashMap<String, LearnedProfileEntry>>(&payload).unwrap_or_default()
    }

    fn save_learned_profiles_map(map: &HashMap<String, LearnedProfileEntry>) {
        let path = Self::learned_profiles_file();
        if let Ok(json) = serde_json::to_string_pretty(map) {
            let _ = fs::write(path, json);
        }
    }

    fn reqs_to_learned(reqs: &GameRequirements) -> LearnedRequirements {
        LearnedRequirements {
            needs_dxvk: reqs.needs_dxvk,
            needs_xaudio: reqs.needs_xaudio,
            needs_vcrun: reqs.needs_vcrun,
            is_64bit: reqs.is_64bit,
            needs_d3dx9: reqs.needs_d3dx9,
            needs_vcrun2005: reqs.needs_vcrun2005,
            needs_vcrun2008: reqs.needs_vcrun2008,
            needs_physx: reqs.needs_physx,
            needs_xact: reqs.needs_xact,
        }
    }

    fn learned_to_reqs(entry: &LearnedProfileEntry) -> GameRequirements {
        GameRequirements {
            needs_dxvk: entry.requirements.needs_dxvk,
            needs_xaudio: entry.requirements.needs_xaudio,
            needs_vcrun: entry.requirements.needs_vcrun,
            is_64bit: entry.requirements.is_64bit,
            needs_d3dx9: entry.requirements.needs_d3dx9,
            needs_vcrun2005: entry.requirements.needs_vcrun2005,
            needs_vcrun2008: entry.requirements.needs_vcrun2008,
            needs_physx: entry.requirements.needs_physx,
            needs_xact: entry.requirements.needs_xact,
            engine_type: "LearnedJSON".into(),
            engine_version: None,
            has_anticheat: false,
        }
    }

    pub fn save_learned_profile_json(
        &self,
        source_path: &str,
        game_name: &str,
        preferred_exe: Option<&str>,
        selected_proton: Option<&str>,
        prefix_path: Option<&str>,
        reqs: &GameRequirements,
        gpu_vendor: Option<&str>,
    ) {
        let game_id = Self::normalize_game_id(game_name);
        let mut map = Self::load_learned_profiles_map();
        let now = chrono::Local::now().to_rfc3339();

        let entry = map
            .entry(game_id.clone())
            .or_insert_with(|| LearnedProfileEntry {
                game_id: game_id.clone(),
                game_name: game_name.to_string(),
                source_paths: Vec::new(),
                preferred_exe: None,
                selected_proton: None,
                prefix_path: None,
                requirements: Self::reqs_to_learned(reqs),
                success_count: 0,
                last_success_at: now.clone(),
                gpu_variants: HashMap::new(),
                history: Vec::new(),
            });

        if entry.success_count > 0 {
            entry.history.push(LearnedHistoryEntry {
                saved_at: chrono::Local::now().to_rfc3339(),
                preferred_exe: entry.preferred_exe.clone(),
                selected_proton: entry.selected_proton.clone(),
                prefix_path: entry.prefix_path.clone(),
                requirements: entry.requirements.clone(),
                success_count: entry.success_count,
            });
            if entry.history.len() > 3 {
                entry.history.drain(0..entry.history.len() - 3);
            }
        }

        if !entry.source_paths.iter().any(|s| s == source_path) {
            entry.source_paths.push(source_path.to_string());
        }
        entry.game_name = game_name.to_string();
        entry.requirements = Self::reqs_to_learned(reqs);
        entry.preferred_exe = preferred_exe
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty());
        entry.selected_proton = selected_proton
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty());
        entry.prefix_path = prefix_path.map(|s| s.to_string()).filter(|s| !s.is_empty());
        entry.success_count = entry.success_count.saturating_add(1);
        entry.last_success_at = now;

        if let Some(gpu) = gpu_vendor {
            let key = gpu.trim().to_uppercase();
            if !key.is_empty() {
                let gpu_entry =
                    entry
                        .gpu_variants
                        .entry(key)
                        .or_insert_with(|| LearnedGpuVariant {
                            preferred_exe: None,
                            selected_proton: None,
                            prefix_path: None,
                            requirements: Self::reqs_to_learned(reqs),
                            success_count: 0,
                            last_success_at: chrono::Local::now().to_rfc3339(),
                            history: Vec::new(),
                        });

                if gpu_entry.success_count > 0 {
                    gpu_entry.history.push(LearnedHistoryEntry {
                        saved_at: chrono::Local::now().to_rfc3339(),
                        preferred_exe: gpu_entry.preferred_exe.clone(),
                        selected_proton: gpu_entry.selected_proton.clone(),
                        prefix_path: gpu_entry.prefix_path.clone(),
                        requirements: gpu_entry.requirements.clone(),
                        success_count: gpu_entry.success_count,
                    });
                    if gpu_entry.history.len() > 3 {
                        gpu_entry.history.drain(0..gpu_entry.history.len() - 3);
                    }
                }

                gpu_entry.preferred_exe = preferred_exe
                    .map(|s| s.to_string())
                    .filter(|s| !s.is_empty());
                gpu_entry.selected_proton = selected_proton
                    .map(|s| s.to_string())
                    .filter(|s| !s.is_empty());
                gpu_entry.prefix_path =
                    prefix_path.map(|s| s.to_string()).filter(|s| !s.is_empty());
                gpu_entry.requirements = Self::reqs_to_learned(reqs);
                gpu_entry.success_count = gpu_entry.success_count.saturating_add(1);
                gpu_entry.last_success_at = chrono::Local::now().to_rfc3339();
            }
        }

        Self::save_learned_profiles_map(&map);
    }

    pub fn load_learned_profile_json(
        &self,
        source_path: &str,
        game_name: &str,
        gpu_vendor: Option<&str>,
    ) -> Option<(
        GameRequirements,
        Option<String>,
        Option<String>,
        Option<String>,
        String,
        u8,
        String,
    )> {
        let map = Self::load_learned_profiles_map();
        let game_id = Self::normalize_game_id(game_name);
        let gpu_key = gpu_vendor.map(|g| g.trim().to_uppercase());

        let build_from_entry = |entry: &LearnedProfileEntry| {
            if let Some(key) = gpu_key.as_ref() {
                if let Some(gpu_variant) = entry.gpu_variants.get(key) {
                    let confidence =
                        (70 + (gpu_variant.success_count.saturating_mul(8)).min(28)).min(98) as u8;
                    return (
                        GameRequirements {
                            needs_dxvk: gpu_variant.requirements.needs_dxvk,
                            needs_xaudio: gpu_variant.requirements.needs_xaudio,
                            needs_vcrun: gpu_variant.requirements.needs_vcrun,
                            is_64bit: gpu_variant.requirements.is_64bit,
                            needs_d3dx9: gpu_variant.requirements.needs_d3dx9,
                            needs_vcrun2005: gpu_variant.requirements.needs_vcrun2005,
                            needs_vcrun2008: gpu_variant.requirements.needs_vcrun2008,
                            needs_physx: gpu_variant.requirements.needs_physx,
                            needs_xact: gpu_variant.requirements.needs_xact,
                            engine_type: "LearnedJSON-GPU".into(),
                            engine_version: None,
                            has_anticheat: false,
                        },
                        gpu_variant.preferred_exe.clone(),
                        gpu_variant.selected_proton.clone(),
                        gpu_variant.prefix_path.clone(),
                        entry.game_name.clone(),
                        confidence,
                        format!(
                            "GPU-variant matched ({}, successes: {})",
                            key, gpu_variant.success_count
                        ),
                    );
                }
            }

            let confidence = (62 + (entry.success_count.saturating_mul(6)).min(30)).min(95) as u8;
            (
                Self::learned_to_reqs(entry),
                entry.preferred_exe.clone(),
                entry.selected_proton.clone(),
                entry.prefix_path.clone(),
                entry.game_name.clone(),
                confidence,
                format!(
                    "Generic learned profile (successes: {})",
                    entry.success_count
                ),
            )
        };

        if let Some(entry) = map
            .values()
            .find(|e| e.source_paths.iter().any(|sp| sp == source_path))
        {
            return Some(build_from_entry(entry));
        }

        map.get(&game_id).map(build_from_entry)
    }

    pub fn rollback_learned_profile_json(
        &self,
        game_name: &str,
        gpu_vendor: Option<&str>,
    ) -> Result<String, String> {
        let game_id = Self::normalize_game_id(game_name);
        let mut map = Self::load_learned_profiles_map();
        let entry = map
            .get_mut(&game_id)
            .ok_or_else(|| "No learned profile found for this game".to_string())?;

        if let Some(gpu) = gpu_vendor {
            let key = gpu.trim().to_uppercase();
            if let Some(variant) = entry.gpu_variants.get_mut(&key) {
                if let Some(prev) = variant.history.pop() {
                    variant.preferred_exe = prev.preferred_exe;
                    variant.selected_proton = prev.selected_proton;
                    variant.prefix_path = prev.prefix_path;
                    variant.requirements = prev.requirements;
                    variant.success_count = prev.success_count.max(1);
                    variant.last_success_at = chrono::Local::now().to_rfc3339();
                    Self::save_learned_profiles_map(&map);
                    return Ok(format!("Rolled back GPU profile variant for {}", key));
                }
            }
        }

        if let Some(prev) = entry.history.pop() {
            entry.preferred_exe = prev.preferred_exe;
            entry.selected_proton = prev.selected_proton;
            entry.prefix_path = prev.prefix_path;
            entry.requirements = prev.requirements;
            entry.success_count = prev.success_count.max(1);
            entry.last_success_at = chrono::Local::now().to_rfc3339();
            Self::save_learned_profiles_map(&map);
            return Ok("Rolled back generic learned profile".to_string());
        }

        Err("No rollback snapshot available".to_string())
    }

    fn scope_to_text(scope: ExportScope) -> &'static str {
        match scope {
            ExportScope::Full => "Full",
            ExportScope::PrefixOnly => "PrefixOnly",
            ExportScope::GameOnly => "GameOnly",
            ExportScope::LibsOnly => "LibsOnly",
        }
    }

    fn scope_from_text(text: &str) -> ExportScope {
        match text {
            "PrefixOnly" => ExportScope::PrefixOnly,
            "GameOnly" => ExportScope::GameOnly,
            "LibsOnly" => ExportScope::LibsOnly,
            _ => ExportScope::Full,
        }
    }
}
