use anyhow::{Context, Result};
use std::path::Path;

pub fn load_token(path: &Path) -> Result<super::XToken> {
    let s = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let t: super::XToken = serde_json::from_str(&s).context("failed to parse x token json")?;
    Ok(t)
}
