use anyhow::{bail, Context, Result};
use reqwest::multipart;
use reqwest::Client;
use std::path::Path;

pub(super) async fn upload_media(
    client: &Client,
    site: &str,
    token: &str,
    image_path: &Path,
) -> Result<Option<String>> {
    let url = format!(
        "https://public-api.wordpress.com/rest/v1.1/sites/{}/media/new",
        site
    );

    let filename = image_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("main.jpg")
        .to_string();

    let bytes = std::fs::read(image_path)
        .with_context(|| format!("failed to read image {}", image_path.display()))?;

    let part = multipart::Part::bytes(bytes)
        .file_name(filename)
        .mime_str("image/jpeg")
        .unwrap();

    let form = multipart::Form::new().part("media[]", part);

    let resp = client
        .post(url)
        .bearer_auth(token)
        .multipart(form)
        .send()
        .await
        .context("failed to upload media to WordPress.com")?;

    let status_code = resp.status();
    let body_text = resp
        .text()
        .await
        .context("failed to read WordPress.com media response body")?;

    if !status_code.is_success() {
        bail!(
            "WordPress.com media upload failed: HTTP {}: {}",
            status_code,
            body_text
        );
    }

    let media_url = serde_json::from_str::<serde_json::Value>(&body_text)
        .ok()
        .and_then(|v| {
            v.get("media")
                .and_then(|m| m.get(0))
                .and_then(|m0| m0.get("URL").or_else(|| m0.get("url")))
                .and_then(|u| u.as_str())
                .map(|s| s.to_string())
        });

    Ok(media_url)
}
