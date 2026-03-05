use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub enum UiMode {
    #[serde(rename = "simple")]
    Simple,
    #[serde(rename = "advanced")]
    Advanced,
}

impl Default for UiMode {
    fn default() -> Self {
        UiMode::Simple
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    pub default_install_dir: String,
    pub preferred_proton: Option<String>,
    #[serde(default = "default_lang")]
    pub language: String,
    #[serde(default)]
    pub ui_mode: UiMode,
    #[serde(default)]
    pub first_launch_completed: bool,
    #[serde(default = "default_true")]
    pub welcome_animation_enabled: bool,
    #[serde(default = "default_true")]
    pub welcome_screen_enabled: bool,
    #[serde(default)]
    pub app_shortcut_installed: bool,
}

fn default_lang() -> String {
    "English".to_string()
}

fn default_true() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
        Self {
            default_install_dir: format!("{}/Games/R2L", home),
            preferred_proton: None,
            language: "English".to_string(),
            ui_mode: UiMode::Simple,
            first_launch_completed: false,
            welcome_animation_enabled: true,
            welcome_screen_enabled: true,
            app_shortcut_installed: false,
        }
    }
}

pub struct ConfigManager;

impl ConfigManager {
    fn new_config_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
        PathBuf::from(home).join(".config/repack2linux/config.json")
    }

    fn legacy_config_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
        PathBuf::from(home).join(".config/repack2proton/config.json")
    }

    pub fn get_config_path() -> PathBuf {
        Self::new_config_path()
    }

    pub fn load() -> AppConfig {
        let path = Self::new_config_path();
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(config) = serde_json::from_str(&content) {
                return config;
            }
        }

        let legacy = Self::legacy_config_path();
        if let Ok(content) = fs::read_to_string(&legacy) {
            if let Ok(config) = serde_json::from_str::<AppConfig>(&content) {
                let _ = Self::save(&config);
                return config;
            }
        }

        let default = AppConfig::default();
        let _ = Self::save(&default);
        default
    }

    pub fn save(config: &AppConfig) -> std::io::Result<()> {
        let path = Self::get_config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(config)?;
        fs::write(path, content)?;
        Ok(())
    }
}
