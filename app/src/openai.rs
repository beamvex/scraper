use anyhow::{Context, Result, bail};
use serde_json::json;
use std::path::PathBuf;

pub async fn load_chatgpt_key() -> Result<String> {
    let home = std::env::var("HOME").context("HOME env var not set")?;
    let key_path: PathBuf = [home.as_str(), ".chatgptkey"].iter().collect();
    let key = tokio::fs::read_to_string(&key_path)
        .await
        .with_context(|| format!("failed to read key file at {}", key_path.display()))?;
    let key = key.trim().to_string();
    if key.is_empty() {
        bail!("chatgpt key file is empty: {}", key_path.display());
    }
    Ok(key)
}

pub async fn generate_review_article_html(
    api_key: &str,
    product_title: &str,
    product_url: Option<&str>,
    product_page_html: &str,
) -> Result<String> {
    let mut html = product_page_html;
    const MAX_CHARS: usize = 120_000;
    if html.len() > MAX_CHARS {
        html = &html[..MAX_CHARS];
    }

    let url_line = product_url
        .map(|u| format!("Product URL: {}\n", u))
        .unwrap_or_default();

    let prompt = format!(
        "You are an expert consumer tech reviewer. Write a review article that is about a 4-minute read.\n\n\
Output requirements:\n\
- Output valid HTML only (no Markdown).\n\
- Use <article> with a single <h1>, then sections with <h2>.\n\
- Include: overview, key features, who it's for, pros/cons lists, pricing/value discussion, and verdict.\n\
- Do not mention that you were given raw HTML; infer details from the provided page.\n\
- Write at least 5 sections with h2 headers\n\
- each section should have at least 3 paragraphs \n\
- each paragraph should be 3-5 sentences\n\
- If specs are unclear, state assumptions cautiously.\n\n\
Title: {}\n\
{}\n\
Product detail page HTML (truncated if needed):\n{}",
        product_title, url_line, html
    );

    let body = json!({
        "model": "gpt-4o-mini",
        "messages": [
            {"role": "user", "content": prompt}
        ],
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
    let v: serde_json::Value = resp
        .json()
        .await
        .context("failed to parse OpenAI response JSON")?;

    if !status.is_success() {
        bail!("OpenAI request failed ({}): {}", status, v);
    }

    let content = v
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c0| c0.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .map(|s| s.trim().to_string());

    match content {
        Some(c) if !c.is_empty() => Ok(c),
        _ => bail!("OpenAI response did not contain choices[0].message.content"),
    }
}
