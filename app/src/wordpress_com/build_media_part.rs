use anyhow::{Context, Result};
use reqwest::multipart;
use std::path::Path;

pub(super) fn build_media_part(image_path: &Path) -> Result<multipart::Part> {
    let filename = image_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("main.jpg")
        .to_string();
    let bytes = std::fs::read(image_path)
        .with_context(|| format!("failed to read image {}", image_path.display()))?;
    multipart::Part::bytes(bytes)
        .file_name(filename)
        .mime_str("image/jpeg")
        .map_err(|e| anyhow::anyhow!(e))
}
