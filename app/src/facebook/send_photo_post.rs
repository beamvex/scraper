use anyhow::{Context, Result};
use reqwest::{multipart, Client};

pub(super) async fn send_photo_post(
    page_id: &str,
    caption: String,
    access_token: String,
    part: multipart::Part,
) -> Result<Option<String>> {
    let endpoint = format!("https://graph.facebook.com/v20.0/{}/photos", page_id);
    let form = multipart::Form::new()
        .text("caption", caption)
        .text("access_token", access_token)
        .part("source", part);
    let resp = Client::new().post(endpoint).multipart(form).send().await
        .context("failed to send Facebook page photo post request")?;
    let status = resp.status();
    let body = resp.text().await.context("failed to read Facebook page photo post response")?;
    if !status.is_success() {
        return Err(anyhow::anyhow!("Facebook page photo post failed: HTTP {}: {}", status, body));
    }
    let v: serde_json::Value = serde_json::from_str(&body).context("failed to parse Facebook response")?;
    Ok(v.get("post_id").or_else(|| v.get("id")).and_then(|x| x.as_str()).map(String::from))
}
