use flate2::read::GzDecoder;
use futures_util::StreamExt;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use tar::Archive;
use tokio::io::AsyncWriteExt;
use tokio::task;
use tokio::time::{sleep, Duration};

pub struct ProtonManager;

impl ProtonManager {
    const MAX_RETRIES: usize = 3;
    const BACKOFF_MS: u64 = 600;

    pub fn get_home_dir() -> PathBuf {
        PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/".to_string()))
    }

    pub fn get_compatibility_tools_dir() -> PathBuf {
        let path = Self::get_home_dir().join(".steam/root/compatibilitytools.d");
        if !path.exists() {
            let _ = fs::create_dir_all(&path);
        }
        path
    }

    pub fn list_ge_protons() -> Vec<String> {
        let mut protons = Vec::new();
        let home = Self::get_home_dir();

        let paths = vec![
            Self::get_compatibility_tools_dir(),
            home.join(".local/share/Steam/compatibilitytools.d"),
        ];

        for path in paths {
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    if let Ok(name) = entry.file_name().into_string() {
                        if name.starts_with("GE-Proton") {
                            protons.push(name);
                        }
                    }
                }
            }
        }
        protons.sort();
        protons.reverse();
        protons.dedup();
        protons
    }

    pub async fn download_latest_ge() -> Result<String, String> {
        let client = reqwest::Client::builder()
            .user_agent("Repack2Linux")
            .build()
            .map_err(|e| e.to_string())?;

        let mut last_error = None;
        for attempt in 0..Self::MAX_RETRIES {
            match Self::download_with_client(&client).await {
                Ok(tag) => return Ok(tag),
                Err(err) => {
                    last_error = Some(err);
                    if attempt + 1 < Self::MAX_RETRIES {
                        let delay = Duration::from_millis(Self::BACKOFF_MS * (attempt as u64 + 1));
                        sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| "Failed to download GE-Proton".into()))
    }

    async fn download_with_client(client: &reqwest::Client) -> Result<String, String> {
        let res = client
            .get("https://api.github.com/repos/GloriousEggroll/proton-ge-custom/releases/latest")
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json::<Value>()
            .await
            .map_err(|e| e.to_string())?;

        let tag = res["tag_name"].as_str().ok_or("No tag found")?.to_string();
        let asset = res["assets"]
            .as_array()
            .ok_or("No assets")?
            .iter()
            .find(|a| {
                a["name"]
                    .as_str()
                    .map(|n| n.ends_with(".tar.gz"))
                    .unwrap_or(false)
            })
            .ok_or("No tarball found")?;
        let asset_url = asset["browser_download_url"]
            .as_str()
            .ok_or("Missing download url")?;

        let tar_name = format!("{}.tar.gz", tag);
        let tar_path = std::env::temp_dir().join(&tar_name);

        Self::download_asset(client, asset_url, &tar_path).await?;
        let dest_dir = Self::get_compatibility_tools_dir();
        Self::extract_archive(&tar_path, &dest_dir).await?;
        let _ = tokio::fs::remove_file(&tar_path).await;
        Ok(tag)
    }

    async fn download_asset(
        client: &reqwest::Client,
        url: &str,
        path: &Path,
    ) -> Result<(), String> {
        let response = client.get(url).send().await.map_err(|e| e.to_string())?;

        if !response.status().is_success() {
            return Err(format!("Download failed: HTTP {}", response.status()));
        }

        let expected = response.content_length();
        let mut file = tokio::fs::File::create(path)
            .await
            .map_err(|e| e.to_string())?;
        let mut downloaded = 0u64;

        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| e.to_string())?;
            downloaded += chunk.len() as u64;
            file.write_all(&chunk).await.map_err(|e| e.to_string())?;
        }

        if let Some(expected_bytes) = expected {
            if expected_bytes != downloaded {
                return Err(format!(
                    "Downloaded {} bytes but response expected {}",
                    downloaded, expected_bytes,
                ));
            }
        }

        Ok(())
    }

    async fn extract_archive(archive_path: &Path, dest_dir: &Path) -> Result<(), String> {
        let archive_path = archive_path.to_owned();
        let dest_dir = dest_dir.to_owned();

        task::spawn_blocking(move || -> Result<(), String> {
            let file = std::fs::File::open(&archive_path).map_err(|e| e.to_string())?;
            let decoder = GzDecoder::new(file);
            let mut archive = Archive::new(decoder);
            archive.unpack(dest_dir).map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())??;

        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_path(version: &str) -> Option<PathBuf> {
        let home = Self::get_home_dir();
        let paths = vec![
            Self::get_compatibility_tools_dir(),
            home.join(".local/share/Steam/compatibilitytools.d"),
        ];

        for path in paths {
            let p = path.join(version);
            if p.exists() {
                return Some(p);
            }
        }
        None
    }
}
