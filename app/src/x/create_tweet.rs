use anyhow::{Context, Result};

pub(super) async fn create_tweet(access_token: &str, text: &str) -> Result<String> {
    let resp = reqwest::Client::new()
        .post("https://api.twitter.com/2/tweets")
        .bearer_auth(access_token)
        .json(&serde_json::json!({"text": text}))
        .send().await.context("failed to send create tweet request")?;
    let status = resp.status();
    let body = resp.text().await.context("failed to read tweet response")?;
    if !status.is_success() {
        return Err(anyhow::Error::new(super::HttpStatusError(status)).context(body));
    }
    let v: serde_json::Value = serde_json::from_str(&body).context("failed to parse tweet response")?;
    let id = v.get("data").and_then(|d| d.get("id")).and_then(|x| x.as_str())
        .context("missing tweet id")?;
    Ok(id.to_string())
}
