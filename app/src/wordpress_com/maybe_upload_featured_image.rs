use anyhow::Result;
use reqwest::Client;
use std::path::Path;

pub(super) async fn maybe_upload_featured_image(
    client: &Client,
    site: &str,
    token: &str,
    title: &str,
    content: String,
    image_path: Option<&Path>,
) -> Result<(String, Option<String>)> {
    let Some(ip) = image_path.filter(|p| p.exists()) else {
        return Ok((content, None));
    };
    let Some(url) = super::upload_media::upload_media(client, site, token, ip).await? else {
        return Ok((content, None));
    };
    let content = format!("<p><img src=\"{}\" alt=\"{}\" /></p>\n{}", url, title, content);
    Ok((content, Some(url)))
}
