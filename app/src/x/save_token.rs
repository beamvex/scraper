use anyhow::{Context, Result};
use std::path::Path;

pub fn save_token(path: &Path, token: &super::XToken) -> Result<()> {
    let s = serde_json::to_string_pretty(token).context("failed to serialize token json")?;
    std::fs::write(path, s).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}
