use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::Value;
use std::env;

pub async fn post_link_to_page(title: &str, url: Option<&str>) -> Result<Option<String>> {
    let page_id = match env::var("FB_PAGE_ID") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => return Ok(None),
    };

    let access_token = match env::var("FB_PAGE_ACCESS_TOKEN") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => return Ok(None),
    };

    let message = match env::var("FB_POST_TEMPLATE") {
        Ok(t) if !t.trim().is_empty() => t
            .replace("{title}", title)
            .replace("{url}", url.unwrap_or("")),
        _ => {
            let u = url.unwrap_or("");
            if u.is_empty() {
                title.to_string()
            } else {
                format!("{}\n{}", title, u)
            }
        }
    };

    let endpoint = format!("https://graph.facebook.com/v20.0/{}/feed", page_id);

    let mut form: Vec<(String, String)> = vec![
        ("message".to_string(), message),
        ("access_token".to_string(), access_token),
    ];
    if let Some(u) = url {
        if !u.trim().is_empty() {
            form.push(("link".to_string(), u.to_string()));
        }
    }

    let body = serde_urlencoded::to_string(&form).context("failed to url-encode facebook form")?;

    let client = Client::new();
    let resp = client
        .post(endpoint)
        .header(
            reqwest::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded",
        )
        .body(body)
        .send()
        .await
        .context("failed to send Facebook page feed post request")?;

    let status = resp.status();
    let body = resp
        .text()
        .await
        .context("failed to read Facebook page feed response")?;

    if !status.is_success() {
        return Err(anyhow::anyhow!(
            "Facebook page post failed: HTTP {}: {}",
            status,
            body
        ));
    }

    let v: Value = serde_json::from_str(&body).context("failed to parse Facebook response")?;
    let id = v.get("id").and_then(|x| x.as_str()).map(|s| s.to_string());

    Ok(id)
}
