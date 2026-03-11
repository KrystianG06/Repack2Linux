use std::fs;
use std::io::Write;
use std::path::PathBuf;

pub struct ShortcutManager;

impl ShortcutManager {
    pub fn create_desktop_shortcut(
        game_name: &str,
        exe_path: &str,
        prefix_path: &str,
        icon_path: Option<&str>,
        mangohud: bool,
        gamemode: bool,
    ) -> Result<PathBuf, String> {
        let home = std::env::var("HOME").map_err(|_| "No HOME var".to_string())?;
        let applications_dir = PathBuf::from(&home).join(".local/share/applications");

        if !applications_dir.exists() {
            fs::create_dir_all(&applications_dir).map_err(|e| e.to_string())?;
        }

        let clean_name = game_name.replace(" ", "_").to_lowercase();
        let desktop_file_path = applications_dir.join(format!("r2p_{}.desktop", clean_name));

        // Budowanie komendy Exec przy użyciu 'env' (bardziej stabilne niż bash -c)
        let exe_path_quoted = if exe_path.contains(' ') { format!("\"{}\"", exe_path) } else { exe_path.to_string() };
        let prefix_path_quoted = if prefix_path.contains(' ') { format!("\"{}\"", prefix_path) } else { prefix_path.to_string() };
        
        let mut full_exec = format!("env WINEPREFIX={} ", prefix_path_quoted);
        if mangohud {
            full_exec.push_str("MANGOHUD=1 ");
        }

        let mut command = format!("wine {}", exe_path_quoted);
        if gamemode {
            command = format!("gamemoderun {}", command);
        }
        full_exec.push_str(&command);

        let icon = icon_path.unwrap_or("applications-games");

        let path_str = std::path::Path::new(exe_path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        let content = format!(
            "[Desktop Entry]\n\
            Type=Application\n\
            Name={}\n\
            Comment=Zainstalowano przez Repack2Linux Factory\n\
            Exec={}\n\
            Path={}\n\
            Icon={}\n\
            Terminal=false\n\
            Categories=Game;\n\
            StartupNotify=true\n",
            game_name, full_exec, path_str, icon
        );

        let mut file = fs::File::create(&desktop_file_path).map_err(|e| e.to_string())?;
        file.write_all(content.as_bytes())
            .map_err(|e| e.to_string())?;

        Ok(desktop_file_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn create_desktop_shortcut_exports_icon_line() {
        let temp_home = tempdir().unwrap();
        let previous = env::var_os("HOME");
        env::set_var("HOME", temp_home.path());

        let result = ShortcutManager::create_desktop_shortcut(
            "Test Game",
            "/tmp/game.exe",
            "/tmp/pfx",
            Some("custom-icon"),
            false,
            false,
        );

        if let Some(prev) = previous {
            env::set_var("HOME", prev);
        } else {
            env::remove_var("HOME");
        }

        assert!(result.is_ok());
        let path = result.unwrap();
        let contents = fs::read_to_string(&path).unwrap();
        assert!(contents.contains("Icon=custom-icon"));
        assert!(contents.contains("Name=Test Game"));
    }
}
