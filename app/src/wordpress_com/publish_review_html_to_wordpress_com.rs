use anyhow::{Context, Result};
use reqwest::Client;
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
    let client = Client::new();
    let (title, raw_content) = super::read_review_html::read_review_html(review_html_path)?;
    let content = raw_content.replace(&title, "");
    let (content, featured_image) = super::maybe_upload_featured_image::maybe_upload_featured_image(
        &client, &site, &token, &title, content, main_image_path,
    ).await?;
    super::send_create_post::send_create_post(
        &client, &site, &token, &title, &content, &status, category, featured_image,
    ).await
}
