use crate::detector::GameRequirements;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::Duration;

fn normalize_like_id(value: &str) -> String {
    value
        .to_lowercase()
        .replace(':', "")
        .replace('-', " ")
        .replace('_', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
}

fn read_json(path: &Path) -> Result<Value, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    serde_json::from_str(&raw).map_err(|e| format!("Invalid JSON in {}: {}", path.display(), e))
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    let payload =
        serde_json::to_string_pretty(value).map_err(|e| format!("JSON serialize error: {}", e))?;
    std::fs::write(path, payload).map_err(|e| format!("Failed to write {}: {}", path.display(), e))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PendingRemoteSync {
    created_at: String,
    last_attempt_at: Option<String>,
    attempts: u32,
    last_error: String,
}

const MAX_RETRY_ATTEMPTS: u32 = 20;
const RETRY_TTL_HOURS: i64 = 24 * 7;

#[derive(Debug, Clone)]
pub struct QueueSnapshot {
    pub pending: bool,
    pub attempts: u32,
    pub last_attempt_at: Option<String>,
    pub last_error: Option<String>,
    pub repo_root: Option<String>,
    pub remote_enabled: bool,
}

fn queue_file() -> PathBuf {
    let mut dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    dir.push("repack2linux");
    let _ = std::fs::create_dir_all(&dir);
    dir.push("community-sync-queue.json");
    if !dir.exists() {
        let mut legacy = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        legacy.push("repack2proton");
        legacy.push("community-sync-queue.json");
        if legacy.exists() {
            let _ = std::fs::copy(&legacy, &dir);
        }
    }
    dir
}

fn load_queue() -> Option<PendingRemoteSync> {
    let path = queue_file();
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str::<PendingRemoteSync>(&raw).ok()
}

pub fn queue_snapshot() -> QueueSnapshot {
    let queue = load_queue();
    let repo_root = resolve_repo_root();
    let remote_enabled = std::env::var("R2L_GITHUB_TOKEN")
        .or_else(|_| std::env::var("R2P_GITHUB_TOKEN"))
        .ok()
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);

    QueueSnapshot {
        pending: queue.is_some(),
        attempts: queue.as_ref().map(|q| q.attempts).unwrap_or(0),
        last_attempt_at: queue.as_ref().and_then(|q| q.last_attempt_at.clone()),
        last_error: queue
            .as_ref()
            .map(|q| q.last_error.clone())
            .filter(|e| !e.is_empty()),
        repo_root: repo_root.map(|p| p.to_string_lossy().to_string()),
        remote_enabled,
    }
}

fn save_queue(entry: &PendingRemoteSync) -> Result<(), String> {
    let path = queue_file();
    let payload =
        serde_json::to_string_pretty(entry).map_err(|e| format!("Queue serialize error: {}", e))?;
    std::fs::write(&path, payload)
        .map_err(|e| format!("Queue write error {}: {}", path.display(), e))
}

fn clear_queue() -> Result<(), String> {
    let path = queue_file();
    if path.exists() {
        std::fs::remove_file(&path)
            .map_err(|e| format!("Queue remove error {}: {}", path.display(), e))?;
    }
    Ok(())
}

fn enqueue_remote_retry(last_error: String) -> Result<(), String> {
    let mut entry = load_queue().unwrap_or(PendingRemoteSync {
        created_at: chrono::Utc::now().to_rfc3339(),
        last_attempt_at: None,
        attempts: 0,
        last_error: String::new(),
    });
    entry.attempts = entry.attempts.saturating_add(1);
    entry.last_attempt_at = Some(chrono::Utc::now().to_rfc3339());
    entry.last_error = last_error;
    save_queue(&entry)
}

fn parse_rfc3339(value: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Utc))
}

fn retry_backoff_delay(attempts: u32) -> Duration {
    let exp = attempts.min(8);
    let minutes = 2_u64.pow(exp);
    Duration::from_secs((minutes * 60).min(60 * 60 * 24))
}

fn has_repo_layout(path: &Path) -> bool {
    path.join("presets.json").exists() && path.join("cloud").join("games.sample.json").exists()
}

fn candidate_roots() -> Vec<PathBuf> {
    let mut out = Vec::new();

    if let Ok(env_root) =
        std::env::var("R2L_COMMUNITY_DB_ROOT").or_else(|_| std::env::var("R2P_COMMUNITY_DB_ROOT"))
    {
        let p = PathBuf::from(env_root);
        out.push(p.clone());
        if let Some(parent) = p.parent() {
            out.push(parent.to_path_buf());
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        out.push(cwd.clone());
        for ancestor in cwd.ancestors().take(8) {
            out.push(ancestor.to_path_buf());
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            out.push(parent.to_path_buf());
            for ancestor in parent.ancestors().take(8) {
                out.push(ancestor.to_path_buf());
            }
        }
    }

    out
}

pub fn resolve_repo_root() -> Option<PathBuf> {
    let mut seen = std::collections::HashSet::new();
    for candidate in candidate_roots() {
        let key = candidate.to_string_lossy().to_string();
        if !seen.insert(key) {
            continue;
        }
        if has_repo_layout(&candidate) {
            return Some(candidate);
        }
    }
    None
}

fn upsert_presets_file(
    path: &Path,
    game_id: &str,
    game_name: &str,
    preferred_exe: Option<&str>,
    selected_proton: Option<&str>,
    reqs: &GameRequirements,
) -> Result<(), String> {
    let mut root = read_json(path)?;
    upsert_presets_value(
        &mut root,
        game_id,
        game_name,
        preferred_exe,
        selected_proton,
        reqs,
    )?;
    write_json(path, &root)
}

fn upsert_presets_value(
    root: &mut Value,
    game_id: &str,
    game_name: &str,
    preferred_exe: Option<&str>,
    selected_proton: Option<&str>,
    reqs: &GameRequirements,
) -> Result<(), String> {
    let arr = root
        .as_array_mut()
        .ok_or_else(|| "presets.json must be a JSON array".to_string())?;

    let mut entry = json!({
        "id": game_id,
        "name": game_name,
        "dxvk": reqs.needs_dxvk,
        "xaudio": reqs.needs_xaudio,
        "vcrun": reqs.needs_vcrun,
        "is_64bit": reqs.is_64bit,
        "proton": selected_proton.unwrap_or("System Wine (Default)"),
        "preferred_exe": preferred_exe.unwrap_or(""),
        "d3dx9": reqs.needs_d3dx9,
        "vcrun2005": reqs.needs_vcrun2005,
        "vcrun2008": reqs.needs_vcrun2008,
        "physx": reqs.needs_physx,
        "xact": reqs.needs_xact,
        "source": "community_learned",
        "updated_at": chrono::Utc::now().to_rfc3339(),
    });

    let idx = arr.iter().position(|item| {
        let id_match = item["id"].as_str().map(|s| s == game_id).unwrap_or(false);
        let name_match = item["name"]
            .as_str()
            .map(|s| normalize_like_id(s) == normalize_like_id(game_name))
            .unwrap_or(false);
        id_match || name_match
    });

    if let Some(i) = idx {
        let prev = arr.get(i).cloned().unwrap_or_else(|| json!({}));
        if entry["preferred_exe"].as_str().unwrap_or("").is_empty() {
            entry["preferred_exe"] = prev["preferred_exe"].clone();
        }
        arr[i] = entry;
    } else {
        arr.push(entry);
    }

    Ok(())
}

fn upsert_cloud_sample_file(
    path: &Path,
    game_name: &str,
    preferred_exe: Option<&str>,
    selected_proton: Option<&str>,
    reqs: &GameRequirements,
) -> Result<(), String> {
    let mut root = read_json(path)?;
    upsert_cloud_sample_value(&mut root, game_name, preferred_exe, selected_proton, reqs)?;
    write_json(path, &root)
}

fn upsert_cloud_sample_value(
    root: &mut Value,
    game_name: &str,
    preferred_exe: Option<&str>,
    selected_proton: Option<&str>,
    reqs: &GameRequirements,
) -> Result<(), String> {
    let games = root["games"]
        .as_array_mut()
        .ok_or_else(|| "games.sample.json must contain 'games' array".to_string())?;

    let mut entry = json!({
        "game_name": game_name,
        "preferred_exe": preferred_exe.unwrap_or(""),
        "needs_dxvk": reqs.needs_dxvk,
        "needs_xaudio": reqs.needs_xaudio,
        "needs_vcrun": reqs.needs_vcrun,
        "is_64bit": reqs.is_64bit,
        "recommended_proton": selected_proton.unwrap_or("System Wine (Default)"),
        "needs_d3dx9": reqs.needs_d3dx9,
        "needs_vcrun2005": reqs.needs_vcrun2005,
        "needs_vcrun2008": reqs.needs_vcrun2008,
        "needs_physx": reqs.needs_physx,
        "needs_xact": reqs.needs_xact,
        "source": "community_learned",
        "updated_at": chrono::Utc::now().to_rfc3339(),
    });

    let idx = games.iter().position(|item| {
        item["game_name"]
            .as_str()
            .map(|s| normalize_like_id(s) == normalize_like_id(game_name))
            .unwrap_or(false)
    });

    if let Some(i) = idx {
        let prev = games.get(i).cloned().unwrap_or_else(|| json!({}));
        if entry["preferred_exe"].as_str().unwrap_or("").is_empty() {
            entry["preferred_exe"] = prev["preferred_exe"].clone();
        }
        games[i] = entry;
    } else {
        games.push(entry);
    }

    Ok(())
}

async fn github_get_sha(
    client: &reqwest::Client,
    token: &str,
    repo: &str,
    branch: &str,
    path: &str,
) -> Result<Option<String>, String> {
    let url = format!(
        "https://api.github.com/repos/{}/contents/{}?ref={}",
        repo, path, branch
    );
    let response = client
        .get(url)
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "repack2linux-rs-community-sync")
        .send()
        .await
        .map_err(|e| format!("GitHub SHA request failed for {}: {}", path, e))?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!(
            "GitHub SHA request failed for {}: HTTP {} {}",
            path, status, body
        ));
    }

    let payload: Value = response
        .json()
        .await
        .map_err(|e| format!("GitHub SHA JSON parse failed for {}: {}", path, e))?;
    Ok(payload["sha"].as_str().map(|s| s.to_string()))
}

async fn github_get_file(
    client: &reqwest::Client,
    token: &str,
    repo: &str,
    branch: &str,
    path: &str,
) -> Result<Option<(String, String)>, String> {
    let url = format!(
        "https://api.github.com/repos/{}/contents/{}?ref={}",
        repo, path, branch
    );
    let response = client
        .get(url)
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "repack2linux-rs-community-sync")
        .send()
        .await
        .map_err(|e| format!("GitHub GET file failed for {}: {}", path, e))?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!(
            "GitHub GET file failed for {}: HTTP {} {}",
            path, status, body
        ));
    }

    let payload: Value = response
        .json()
        .await
        .map_err(|e| format!("GitHub GET file JSON parse failed for {}: {}", path, e))?;
    let sha = payload["sha"]
        .as_str()
        .ok_or_else(|| format!("Missing sha in GitHub response for {}", path))?
        .to_string();
    let content_b64 = payload["content"]
        .as_str()
        .ok_or_else(|| format!("Missing content in GitHub response for {}", path))?
        .replace('\n', "");
    let decoded = STANDARD
        .decode(content_b64.as_bytes())
        .map_err(|e| format!("Base64 decode failed for {}: {}", path, e))?;
    let content = String::from_utf8(decoded)
        .map_err(|e| format!("UTF-8 decode failed for {}: {}", path, e))?;
    Ok(Some((sha, content)))
}

async fn github_put_file(
    client: &reqwest::Client,
    token: &str,
    repo: &str,
    branch: &str,
    path: &str,
    message: &str,
    content: &str,
    sha_override: Option<String>,
) -> Result<(), String> {
    let sha = if let Some(override_sha) = sha_override {
        Some(override_sha)
    } else {
        github_get_sha(client, token, repo, branch, path).await?
    };
    let mut body = json!({
        "message": message,
        "content": STANDARD.encode(content.as_bytes()),
        "branch": branch,
    });
    if let Some(value) = sha {
        body["sha"] = Value::String(value);
    }

    let url = format!("https://api.github.com/repos/{}/contents/{}", repo, path);
    let response = client
        .put(url)
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "repack2linux-rs-community-sync")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("GitHub PUT failed for {}: {}", path, e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!(
            "GitHub PUT failed for {}: HTTP {} {}",
            path, status, body
        ));
    }
    Ok(())
}

fn is_conflict_error(err: &str) -> bool {
    err.contains("HTTP 409")
        || err.contains("sha")
        || err.contains("does not match")
        || err.contains("is out of date")
}

fn merge_presets_json(local: &str, remote: &str) -> Result<String, String> {
    let local_v: Value =
        serde_json::from_str(local).map_err(|e| format!("local presets parse: {}", e))?;
    let remote_v: Value =
        serde_json::from_str(remote).map_err(|e| format!("remote presets parse: {}", e))?;
    let local_arr = local_v
        .as_array()
        .ok_or_else(|| "local presets must be array".to_string())?;
    let remote_arr = remote_v
        .as_array()
        .ok_or_else(|| "remote presets must be array".to_string())?;

    let mut map = std::collections::BTreeMap::<String, Value>::new();
    for item in remote_arr {
        let key = item["id"]
            .as_str()
            .map(|s| s.to_string())
            .or_else(|| item["name"].as_str().map(normalize_like_id))
            .unwrap_or_default();
        if !key.is_empty() {
            map.insert(key, item.clone());
        }
    }
    for item in local_arr {
        let key = item["id"]
            .as_str()
            .map(|s| s.to_string())
            .or_else(|| item["name"].as_str().map(normalize_like_id))
            .unwrap_or_default();
        if !key.is_empty() {
            map.insert(key, item.clone());
        }
    }
    let merged = Value::Array(map.into_values().collect());
    serde_json::to_string_pretty(&merged).map_err(|e| format!("serialize merged presets: {}", e))
}

fn merge_cloud_json(local: &str, remote: &str) -> Result<String, String> {
    let local_v: Value =
        serde_json::from_str(local).map_err(|e| format!("local cloud parse: {}", e))?;
    let remote_v: Value =
        serde_json::from_str(remote).map_err(|e| format!("remote cloud parse: {}", e))?;
    let local_arr = local_v["games"]
        .as_array()
        .ok_or_else(|| "local cloud must contain games array".to_string())?;
    let remote_arr = remote_v["games"]
        .as_array()
        .ok_or_else(|| "remote cloud must contain games array".to_string())?;

    let mut map = std::collections::BTreeMap::<String, Value>::new();
    for item in remote_arr {
        let key = item["game_name"]
            .as_str()
            .map(normalize_like_id)
            .unwrap_or_default();
        if !key.is_empty() {
            map.insert(key, item.clone());
        }
    }
    for item in local_arr {
        let key = item["game_name"]
            .as_str()
            .map(normalize_like_id)
            .unwrap_or_default();
        if !key.is_empty() {
            map.insert(key, item.clone());
        }
    }
    let merged = json!({ "games": map.into_values().collect::<Vec<Value>>() });
    serde_json::to_string_pretty(&merged).map_err(|e| format!("serialize merged cloud: {}", e))
}

async fn push_repo_files_to_github(repo_root: &Path) -> Result<String, String> {
    let token =
        match std::env::var("R2L_GITHUB_TOKEN").or_else(|_| std::env::var("R2P_GITHUB_TOKEN")) {
            Ok(value) if !value.trim().is_empty() => value,
            _ => return Ok("remote sync skipped (set R2L_GITHUB_TOKEN)".to_string()),
        };

    let repo = std::env::var("R2L_GITHUB_REPO")
        .or_else(|_| std::env::var("R2P_GITHUB_REPO"))
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "KrystianG06/Repack2Proton".to_string());
    let branch = std::env::var("R2L_GITHUB_BRANCH")
        .or_else(|_| std::env::var("R2P_GITHUB_BRANCH"))
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "master".to_string());
    let client = reqwest::Client::new();

    let presets_path = repo_root.join("presets.json");
    let cloud_path = repo_root.join("cloud").join("games.sample.json");
    let presets_content = tokio::fs::read_to_string(&presets_path)
        .await
        .map_err(|e| format!("Failed to re-read {}: {}", presets_path.display(), e))?;
    let cloud_content = tokio::fs::read_to_string(&cloud_path)
        .await
        .map_err(|e| format!("Failed to re-read {}: {}", cloud_path.display(), e))?;

    let message = format!(
        "community-sync: automatic learned update ({})",
        chrono::Utc::now().to_rfc3339()
    );
    let presets_push = github_put_file(
        &client,
        &token,
        &repo,
        &branch,
        "presets.json",
        &message,
        &presets_content,
        None,
    )
    .await;
    if let Err(e) = presets_push {
        if !is_conflict_error(&e) {
            return Err(e);
        }
        let Some((remote_sha, remote_content)) =
            github_get_file(&client, &token, &repo, &branch, "presets.json").await?
        else {
            return Err("Conflict but remote presets.json not found".to_string());
        };
        let merged = merge_presets_json(&presets_content, &remote_content)?;
        let _ = tokio::fs::write(&presets_path, &merged).await;
        github_put_file(
            &client,
            &token,
            &repo,
            &branch,
            "presets.json",
            &message,
            &merged,
            Some(remote_sha),
        )
        .await?;
    }

    let cloud_push = github_put_file(
        &client,
        &token,
        &repo,
        &branch,
        "cloud/games.sample.json",
        &message,
        &cloud_content,
        None,
    )
    .await;
    if let Err(e) = cloud_push {
        if !is_conflict_error(&e) {
            return Err(e);
        }
        let Some((remote_sha, remote_content)) =
            github_get_file(&client, &token, &repo, &branch, "cloud/games.sample.json").await?
        else {
            return Err("Conflict but remote cloud/games.sample.json not found".to_string());
        };
        let merged = merge_cloud_json(&cloud_content, &remote_content)?;
        let _ = tokio::fs::write(&cloud_path, &merged).await;
        github_put_file(
            &client,
            &token,
            &repo,
            &branch,
            "cloud/games.sample.json",
            &message,
            &merged,
            Some(remote_sha),
        )
        .await?;
    }

    Ok(format!("GitHub synced to {}/{}", repo, branch))
}

pub async fn process_retry_queue(repo_root: PathBuf) -> Result<String, String> {
    let Some(mut entry) = load_queue() else {
        return Ok("queue empty".to_string());
    };

    let now = chrono::Utc::now();
    let created_at = parse_rfc3339(&entry.created_at).unwrap_or(now);
    if now - created_at > chrono::Duration::hours(RETRY_TTL_HOURS) {
        clear_queue()?;
        return Ok("retry queue expired and was cleared".to_string());
    }

    if entry.attempts >= MAX_RETRY_ATTEMPTS {
        clear_queue()?;
        return Ok("retry queue reached max attempts and was cleared".to_string());
    }

    if let Some(last_attempt_raw) = &entry.last_attempt_at {
        if let Some(last_attempt) = parse_rfc3339(last_attempt_raw) {
            let wait_for = retry_backoff_delay(entry.attempts.max(1));
            if let Ok(elapsed) = (now - last_attempt).to_std() {
                if elapsed < wait_for {
                    let left = wait_for - elapsed;
                    let left_minutes = (left.as_secs() / 60).max(1);
                    return Ok(format!(
                        "retry backoff active: next attempt in ~{} min",
                        left_minutes
                    ));
                }
            }
        }
    }

    entry.attempts = entry.attempts.saturating_add(1);
    entry.last_attempt_at = Some(chrono::Utc::now().to_rfc3339());
    let _ = save_queue(&entry);

    match push_repo_files_to_github(&repo_root).await {
        Ok(msg) => {
            clear_queue()?;
            Ok(format!("retry queue flushed; {}", msg))
        }
        Err(err) => {
            let _ = enqueue_remote_retry(err.clone());
            Err(format!("retry queue pending: {}", err))
        }
    }
}

pub async fn sync_learned_preset(
    repo_root: PathBuf,
    game_id: String,
    game_name: String,
    preferred_exe: Option<String>,
    selected_proton: Option<String>,
    reqs: GameRequirements,
) -> Result<String, String> {
    let presets_path = repo_root.join("presets.json");
    let cloud_path = repo_root.join("cloud").join("games.sample.json");

    upsert_presets_file(
        &presets_path,
        &game_id,
        &game_name,
        preferred_exe.as_deref(),
        selected_proton.as_deref(),
        &reqs,
    )?;
    upsert_cloud_sample_file(
        &cloud_path,
        &game_name,
        preferred_exe.as_deref(),
        selected_proton.as_deref(),
        &reqs,
    )?;

    let local_summary = "Local community DB updated (presets.json + cloud/games.sample.json)";
    match push_repo_files_to_github(&repo_root).await {
        Ok(msg) => {
            let _ = clear_queue();
            Ok(format!("{}; {}", local_summary, msg))
        }
        Err(err) => {
            let queue_result = enqueue_remote_retry(err.clone());
            let queue_note = if queue_result.is_ok() {
                "remote sync queued for retry"
            } else {
                "remote sync failed and queue write failed"
            };
            Ok(format!("{}; {}: {}", local_summary, queue_note, err))
        }
    }
}
