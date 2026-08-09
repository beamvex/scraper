use anyhow::Result;

pub async fn post_link_to_page(title: &str, url: Option<&str>) -> Result<Option<String>> {
    let Some((page_id, access_token)) = super::get_fb_credentials::get_fb_credentials() else {
        return Ok(None);
    };
    let message = super::build_fb_message::build_fb_message(title, url);
    let form = super::build_fb_feed_form::build_fb_feed_form(message, &access_token, url);
    super::send_fb_feed_post::send_fb_feed_post(&page_id, form).await
}
