use chromiumoxide::browser::Browser;
use std::path::Path;
use tracing::warn;

pub(super) async fn post_pin(
    browser: &Browser,
    review_html: &str,
    product_name: &str,
    post_url: Option<&str>,
    image_path: Option<&Path>,
) {
    let pin_title = crate::util::extract_article_title(review_html)
        .unwrap_or_else(|| product_name.to_string());
    let pin_desc = crate::util::extract_first_paragraph_text(review_html);
    if let Err(e) = crate::pinterest::maybe_post_pin_to_board(
        browser, &pin_title, pin_desc.as_deref(), post_url, image_path,
    ).await {
        warn!(error = %e, "failed to post pinterest pin");
    }
}
