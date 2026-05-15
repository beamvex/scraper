use anyhow::{Context, Result};
use reqwest::Client;
use reqwest::multipart;
use serde_json::Value;
use std::env;
use std::path::Path;

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

pub async fn post_photo_to_page(
    title: &str,
    url: Option<&str>,
    image_path: Option<&Path>,
) -> Result<Option<String>> {
    let page_id = match env::var("FB_PAGE_ID") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => return Ok(None),
    };

    let access_token = match env::var("FB_PAGE_ACCESS_TOKEN") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => return Ok(None),
    };

    let Some(image_path) = image_path.filter(|p| p.exists()) else {
        return post_link_to_page(title, url).await;
    };

    let caption = match env::var("FB_POST_TEMPLATE") {
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

    let endpoint = format!("https://graph.facebook.com/v20.0/{}/photos", page_id);

    let filename = image_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("image.jpg")
        .to_string();
    let bytes = std::fs::read(image_path)
        .with_context(|| format!("failed to read {}", image_path.display()))?;

    let part = multipart::Part::bytes(bytes)
        .file_name(filename)
        .mime_str("image/jpeg")
        .unwrap();

    let form = multipart::Form::new()
        .text("caption", caption)
        .text("access_token", access_token)
        .part("source", part);

    let client = Client::new();
    let resp = client
        .post(endpoint)
        .multipart(form)
        .send()
        .await
        .context("failed to send Facebook page photo post request")?;

    let status = resp.status();
    let body = resp
        .text()
        .await
        .context("failed to read Facebook page photo post response")?;

    if !status.is_success() {
        return Err(anyhow::anyhow!(
            "Facebook page photo post failed: HTTP {}: {}",
            status,
            body
        ));
    }

    let v: Value = serde_json::from_str(&body).context("failed to parse Facebook response")?;

    let post_id = v
        .get("post_id")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string());
    let id = v.get("id").and_then(|x| x.as_str()).map(|s| s.to_string());

    Ok(post_id.or(id))
}
