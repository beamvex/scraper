use anyhow::Result;
use chromiumoxide::browser::Browser;
use std::path::Path;
use tracing::{info, warn};

pub(super) async fn publish_all(
    browser: &Browser,
    query: &str,
    product_name: &str,
    review_html: &str,
    review_path: &Path,
    image_path: Option<&Path>,
) -> Result<()> {
    let post_url = match crate::wordpress_com::publish_review_html_to_wordpress_com(review_path, query, image_path).await {
        Ok(u) => u,
        Err(e) => { warn!(error = %e, "failed to create WordPress.com post"); return Ok(()); }
    };
    info!("created WordPress.com post");
    if let Err(e) = crate::ifttt::trigger_new_post(product_name, post_url.as_deref()).await {
        warn!(error = %e, "failed to trigger IFTTT webhook");
    }
    if let Err(e) = crate::x::maybe_tweet_new_post(product_name, post_url.as_deref()).await {
        warn!(error = %e, "failed to post X tweet");
    }
    super::post_pin::post_pin(browser, review_html, product_name, post_url.as_deref(), image_path).await;
    Ok(())
}
