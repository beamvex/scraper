use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::json;
use std::env;

pub async fn trigger_new_post(title: &str, url: Option<&str>) -> Result<()> {
    let key = match env::var("IFTTT_WEBHOOK_KEY") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => return Ok(()),
    };
    let event = env::var("IFTTT_WEBHOOK_EVENT").unwrap_or_else(|_| "new_post".to_string());

    let endpoint = format!(
        "https://maker.ifttt.com/trigger/{}/json/with/key/{}",
        event, key
    );

    let body = json!({
        "value1": title,
        "value2": url.unwrap_or(""),
    });

    let client = Client::new();
    client
        .post(endpoint)
        .json(&body)
        .send()
        .await
        .context("failed to send IFTTT webhook")?
        .error_for_status()
        .context("IFTTT webhook returned error status")?;

    Ok(())
}
