use anyhow::{Context, Result};
use reqwest::Client;

pub(super) async fn send_fb_feed_post(
    page_id: &str,
    form: Vec<(String, String)>,
) -> Result<Option<String>> {
    let endpoint = format!("https://graph.facebook.com/v20.0/{}/feed", page_id);
    let body = serde_urlencoded::to_string(&form).context("failed to url-encode facebook form")?;
    let resp = Client::new().post(endpoint)
        .header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(body).send().await.context("failed to send Facebook page feed post request")?;
    let status = resp.status();
    let body = resp.text().await.context("failed to read Facebook page feed response")?;
    if !status.is_success() {
        return Err(anyhow::anyhow!("Facebook page post failed: HTTP {}: {}", status, body));
    }
    Ok(serde_json::from_str::<serde_json::Value>(&body).ok()
        .and_then(|v| v.get("id").and_then(|x| x.as_str()).map(String::from)))
}
