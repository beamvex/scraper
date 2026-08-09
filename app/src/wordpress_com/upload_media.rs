use anyhow::{bail, Context, Result};
use reqwest::Client;
use std::path::Path;

pub(super) async fn upload_media(
    client: &Client,
    site: &str,
    token: &str,
    image_path: &Path,
) -> Result<Option<String>> {
    let url = format!("https://public-api.wordpress.com/rest/v1.1/sites/{}/media/new", site);
    let part = super::build_media_part::build_media_part(image_path)?;
    let form = reqwest::multipart::Form::new().part("media[]", part);
    let resp = client.post(url).bearer_auth(token).multipart(form).send().await
        .context("failed to upload media to WordPress.com")?;
    let sc = resp.status();
    let body = resp.text().await.context("failed to read WordPress.com media response body")?;
    if !sc.is_success() { bail!("WordPress.com media upload failed: HTTP {}: {}", sc, body); }
    Ok(super::parse_media_url::parse_media_url(&body))
}
