use anyhow::{Context, Result};
use std::path::Path;

pub(super) fn read_review_html(path: &Path) -> Result<(String, String)> {
    let html = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let title = super::extract_title::extract_title(&html)
        .unwrap_or_else(|| "New post".to_string());
    let content = super::extract_body_html::extract_body_html(&html)
        .unwrap_or_else(|| html.clone());
    Ok((title, content))
}
