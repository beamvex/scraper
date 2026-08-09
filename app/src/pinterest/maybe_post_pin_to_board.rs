use anyhow::Result;
use chromiumoxide::browser::Browser;
use std::env;
use std::path::Path;
use tracing::info;

pub async fn maybe_post_pin_to_board(
    browser: &Browser,
    title: &str,
    description: Option<&str>,
    article_url: Option<&str>,
    image_path: Option<&Path>,
) -> Result<()> {
    info!("maybe posting pin to board");
    let board_url = env::var("PINTEREST_BOARD_URL").ok().filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "https://uk.pinterest.com/forster2474/random-thoughts/".to_string());
    let Some(url) = article_url.filter(|u| !u.trim().is_empty()) else {
        info!("no article url provided, skipping pinterest post"); return Ok(());
    };
    let Some(ip) = image_path.filter(|p| p.exists()) else {
        info!("no image path provided or image does not exist, skipping pinterest post"); return Ok(());
    };
    if env::var("PINTEREST_ENABLED").ok().as_deref() != Some("1") {
        info!("pinterest is disabled, skipping post"); return Ok(());
    }
    info!("posting pin to board");
    super::post_pin_to_board::post_pin_to_board(browser, &board_url, title, description, url, ip).await
}
