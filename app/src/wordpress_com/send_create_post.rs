use anyhow::{bail, Context, Result};
use reqwest::Client;

pub(super) async fn send_create_post(
    client: &Client,
    site: &str,
    token: &str,
    title: &str,
    content: &str,
    status: &str,
    category: &str,
    featured_image: Option<String>,
) -> Result<Option<String>> {
    let url = format!("https://public-api.wordpress.com/rest/v1.1/sites/{}/posts/new", site);
    let payload = super::build_post_payload::build_post_payload(title, content, status, category, featured_image);
    let resp = client.post(url).bearer_auth(token).json(&payload).send().await
        .context("failed to send WordPress.com create post request")?;
    let sc = resp.status();
    let body = resp.text().await.context("failed to read WordPress.com response body")?;
    if !sc.is_success() { bail!("WordPress.com post creation failed: HTTP {}: {}", sc, body); }
    Ok(serde_json::from_str::<serde_json::Value>(&body).ok()
        .and_then(|v| v.get("URL").and_then(|u| u.as_str()).map(String::from)))
}
