use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde_json::json;
use std::env;
use std::path::Path;

pub async fn publish_review_html_to_wordpress_com(
    review_html_path: &Path,
    category: &str,
    main_image_path: Option<&Path>,
) -> Result<Option<String>> {
    let site = env::var("WPCOM_SITE").context("WPCOM_SITE is not set")?;
    let token = env::var("WPCOM_TOKEN").context("WPCOM_TOKEN is not set")?;
    let status = env::var("WPCOM_POST_STATUS").unwrap_or_else(|_| "draft".to_string());

    let html = std::fs::read_to_string(review_html_path)
        .with_context(|| format!("failed to read {}", review_html_path.display()))?;

    let title = super::extract_title::extract_title(&html).unwrap_or_else(|| "New post".to_string());
    let mut content = super::extract_body_html::extract_body_html(&html).unwrap_or_else(|| html.clone());

    content = content.replace(&title, "");

    let client = Client::new();

    let mut featured_image: Option<String> = None;

    if let Some(image_path) = main_image_path {
        if image_path.exists() {
            if let Some(media_url) = super::upload_media::upload_media(&client, &site, &token, image_path).await? {
                featured_image = Some(media_url.clone());
                content = format!("<p><img src=\"{}\" alt=\"{}\" /></p>\n{}", media_url, title, content);
            }
        }
    }

    let url = format!(
        "https://public-api.wordpress.com/rest/v1.1/sites/{}/posts/new",
        site
    );

    let mut payload = serde_json::Map::new();
    payload.insert("title".to_string(), json!(title));
    payload.insert("content".to_string(), json!(content));
    payload.insert("status".to_string(), json!(status));
    payload.insert("categories".to_string(), json!(category));

    if let Some(featured_image) = featured_image {
        payload.insert("featured_image".to_string(), json!(featured_image));
    }

    let resp = client
        .post(url)
        .bearer_auth(token)
        .json(&payload)
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

    let post_url = serde_json::from_str::<serde_json::Value>(&body_text)
        .ok()
        .and_then(|v| v.get("URL").and_then(|u| u.as_str()).map(|s| s.to_string()));

    Ok(post_url)
}
