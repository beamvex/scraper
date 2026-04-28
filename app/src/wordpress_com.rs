use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde_json::json;
use std::env;
use std::path::Path;

pub async fn publish_review_html_to_wordpress_com(review_html_path: &Path) -> Result<()> {
    let site = env::var("WPCOM_SITE").context("WPCOM_SITE is not set")?;
    let token = env::var("WPCOM_TOKEN").context("WPCOM_TOKEN is not set")?;
    let status = env::var("WPCOM_POST_STATUS").unwrap_or_else(|_| "draft".to_string());

    let html = std::fs::read_to_string(review_html_path)
        .with_context(|| format!("failed to read {}", review_html_path.display()))?;

    let title = extract_title(&html).unwrap_or_else(|| "New post".to_string());
    let content = extract_body_html(&html).unwrap_or_else(|| html.clone());

    let url = format!(
        "https://public-api.wordpress.com/rest/v1.1/sites/{}/posts/new",
        site
    );

    let client = Client::new();
    let resp = client
        .post(url)
        .bearer_auth(token)
        .json(&json!({
            "title": title,
            "content": content,
            "status": status,
        }))
        .send()
        .await
        .context("failed to send WordPress.com create post request")?;

    let status_code = resp.status();
    let body_text = resp
        .text()
        .await
        .context("failed to read WordPress.com response body")?;

    if !status_code.is_success() {
        bail!(
            "WordPress.com post creation failed: HTTP {}: {}",
            status_code,
            body_text
        );
    }

    Ok(())
}

fn extract_title(html: &str) -> Option<String> {
    // Prefer first <h1>..</h1>, fallback to <title>..</title>.
    if let Some(h1) = extract_between_case_insensitive(html, "<h1", "</h1>") {
        // strip opening tag
        if let Some(gt) = h1.find('>') {
            let inner = h1[(gt + 1)..].trim();
            if !inner.is_empty() {
                return Some(html_decode_minimal(inner));
            }
        }
    }

    if let Some(t) = extract_between_case_insensitive(html, "<title", "</title>") {
        if let Some(gt) = t.find('>') {
            let inner = t[(gt + 1)..].trim();
            if !inner.is_empty() {
                return Some(html_decode_minimal(inner));
            }
        }
    }

    None
}

fn extract_body_html(html: &str) -> Option<String> {
    let body = extract_between_case_insensitive(html, "<body", "</body>")?;
    let start = body.find('>')?;
    let inner = body[(start + 1)..].trim();
    if inner.is_empty() {
        None
    } else {
        Some(inner.to_string())
    }
}

fn extract_between_case_insensitive(haystack: &str, start_tag: &str, end_tag: &str) -> Option<String> {
    let h_lower = haystack.to_ascii_lowercase();
    let s_lower = start_tag.to_ascii_lowercase();
    let e_lower = end_tag.to_ascii_lowercase();

    let start_idx = h_lower.find(&s_lower)?;
    let after_start = &haystack[start_idx..];

    let end_lower_idx = h_lower[start_idx..].find(&e_lower)?;
    let end_idx = start_idx + end_lower_idx;

    Some(after_start[..(end_idx - start_idx)].to_string())
}

fn html_decode_minimal(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}
