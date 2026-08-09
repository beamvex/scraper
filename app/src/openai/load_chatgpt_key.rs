use anyhow::{Context, Result, bail};
use std::path::PathBuf;

pub async fn load_chatgpt_key() -> Result<String> {
    let home = std::env::var("HOME").context("HOME env var not set")?;
    let key_path: PathBuf = [home.as_str(), ".chatgptkey"].iter().collect();
    let key = tokio::fs::read_to_string(&key_path)
        .await
        .with_context(|| format!("failed to read key file at {}", key_path.display()))?;
    let key = key.trim().to_string();
    if key.is_empty() {
        bail!("chatgpt key file is empty: {}", key_path.display());
    }
    Ok(key)
}
