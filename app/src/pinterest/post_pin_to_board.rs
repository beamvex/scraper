use anyhow::{bail, Context, Result};
use chromiumoxide::browser::Browser;
use std::path::Path;
use std::time::Duration;
use tracing::info;

pub(super) async fn post_pin_to_board(
    browser: &Browser,
    board_url: &str,
    title: &str,
    description: Option<&str>,
    link: &str,
    image_path: &Path,
) -> Result<()> {
    info!(%board_url, "opening pinterest board");
    let _board_page = browser.new_page(board_url).await.context("failed to open pinterest board")?;
    tokio::time::sleep(Duration::from_millis(1200)).await;
    let page = browser.new_page("https://www.pinterest.com/pin-builder/").await.context("failed to open pinterest pin builder")?;
    super::spawn_browser_logger::spawn_browser_logger(&page).await?;
    tokio::time::sleep(Duration::from_millis(3000)).await;
    super::ui::wait_for_pin_creation_ui(&page).await?;
    if super::ui::is_login_interstitial(&page).await.unwrap_or(false) {
        bail!("pinterest appears to require login in the current browser session (redirected to login)");
    }
    super::upload::upload_image(&page, image_path).await?;
    super::fields::fill_text_fields(&page, title, description, link).await?;
    super::board::choose_board(&page, board_url).await?;
    super::publish::publish_pin(&page).await
}
