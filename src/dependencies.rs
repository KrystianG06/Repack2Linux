use std::path::PathBuf;
use std::process::Command;

pub struct DependencyManager;

impl DependencyManager {
    pub fn is_available() -> bool {
        Command::new("which")
            .arg("winetricks")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    pub fn check_vulkan_functional() -> bool {
        let paths = ["/usr/share/vulkan/icd.d", "/etc/vulkan/icd.d"];
        for p in paths {
            if let Ok(entries) = std::fs::read_dir(p) {
                if entries.count() > 0 {
                    return true;
                }
            }
        }
        false
    }

    pub fn check_system_libs() -> Vec<String> {
        let mut missing = Vec::new();

        let libs = vec![
            ("libvulkan.so.1", "x86", "libvulkan1:i386"),
            ("libGL.so.1", "x86", "libgl1:i386"),
        ];

        for (lib, arch, pkg) in libs {
            let output = Command::new("ldconfig").arg("-p").output();

            if let Ok(out) = output {
                let s = String::from_utf8_lossy(&out.stdout);
                let found = s.lines().any(|line| {
                    line.contains(lib)
                        && (line.contains(arch)
                            || line.contains("x86,")
                            || line.contains("libc6,x86"))
                });

                if !found {
                    missing.push(pkg.to_string());
                }
            }
        }
        missing
    }

    pub async fn install(prefix: &PathBuf, packages: Vec<&str>) -> Result<(), String> {
        if packages.is_empty() {
            return Ok(());
        }
        if !Self::is_available() {
            return Err(
                "KRYTYCZNY BLAD: 'winetricks' nie jest zainstalowany w systemie!".to_string(),
            );
        }

        let mut child = tokio::process::Command::new("winetricks")
            .arg("-q")
            .arg("--force")
            .args(&packages)
            .env("WINEPREFIX", prefix.to_string_lossy().to_string())
            .env("WINE_LARGE_ADDRESS_AWARE", "1")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("Nie udalo sie uruchomic winetricks: {}", e))?;

        let status = child
            .wait()
            .await
            .map_err(|e| format!("Oczekiwanie na winetricks przerwane: {}", e))?;

        if status.success() {
            Ok(())
        } else if status.code() == Some(1) {
            // Kod 1 to często błędy sum kontrolnych lub ostrzeżenia, które nie psują wszystkiego
            eprintln!("[WARN] Winetricks zakonczyl sie z ostrzezeniami (kod 1). Kontynuujemy...");
            Ok(())
        } else {
            Err(format!(
                "Winetricks zakonczyl sie bledem (kod: {}). Sprobuj zainstalowac recznie: {:?}",
                status.code().unwrap_or(-1),
                packages
            ))
        }
    }
}
