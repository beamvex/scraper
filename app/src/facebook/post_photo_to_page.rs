use anyhow::Result;
use std::path::Path;

pub async fn post_photo_to_page(
    title: &str,
    url: Option<&str>,
    image_path: Option<&Path>,
) -> Result<Option<String>> {
    let Some((page_id, access_token)) = super::get_fb_credentials::get_fb_credentials() else {
        return Ok(None);
    };
    let Some(ip) = image_path.filter(|p| p.exists()) else {
        return super::post_link_to_page::post_link_to_page(title, url).await;
    };
    let caption = super::build_fb_message::build_fb_message(title, url);
    let part = super::build_photo_part::build_photo_part(ip)?;
    super::send_photo_post::send_photo_post(&page_id, caption, access_token, part).await
}
