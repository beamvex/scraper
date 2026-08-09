use anyhow::{bail, Context, Result};
use serde_json::json;

pub(super) async fn call_openai(api_key: &str, prompt: String) -> Result<serde_json::Value> {
    let body = json!({
        "model": "gpt-4o-mini",
        "messages": [{"role": "user", "content": prompt}],
        "temperature": 0.7
    });
    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .context("failed to call OpenAI chat completions")?;
    let status = resp.status();
    let v: serde_json::Value = resp.json().await.context("failed to parse OpenAI response JSON")?;
    if !status.is_success() { bail!("OpenAI request failed ({}): {}", status, v); }
    Ok(v)
}
