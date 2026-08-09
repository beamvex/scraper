use anyhow::{bail, Context, Result};
use chromiumoxide::browser::Browser;
use chromiumoxide::cdp::browser_protocol::log::EventEntryAdded;
use chromiumoxide::cdp::js_protocol::runtime::EventConsoleApiCalled;
use futures::StreamExt;
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
    let _board_page = browser
        .new_page(board_url)
        .await
        .context("failed to open pinterest board")?;

    tokio::time::sleep(Duration::from_millis(1200)).await;

    let pin_builder_url = "https://www.pinterest.com/pin-builder/";
    info!(%pin_builder_url, "opening pinterest pin builder");
    let page = browser
        .new_page(pin_builder_url)
        .await
        .context("failed to open pinterest pin builder")?;

    let mut log_events = page.event_listener::<EventEntryAdded>().await?;
    let mut console_events = page.event_listener::<EventConsoleApiCalled>().await?;
    tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(ev) = log_events.next() => {
                    tracing::info!(
                        level = ?ev.entry.level,
                        source = ?ev.entry.source,
                        url = ?ev.entry.url,
                        "[browser log] {}", ev.entry.text
                    );
                }
                Some(ev) = console_events.next() => {
                    let args: Vec<String> = ev.args.iter()
                        .map(|a| a.value.as_ref()
                            .map(|v| v.to_string())
                            .or_else(|| a.description.clone())
                            .unwrap_or_default())
                        .collect();
                    tracing::info!(r#type = ?ev.r#type, "[console] {}", args.join(" "));
                }
                else => break,
            }
        }
    });

    tokio::time::sleep(Duration::from_millis(3000)).await;

    super::ui::wait_for_pin_creation_ui(&page).await?;

    if super::ui::is_login_interstitial(&page).await.unwrap_or(false) {
        bail!(
            "pinterest appears to require login in the current browser session (redirected to login)"
        );
    }

    super::upload::upload_image(&page, image_path).await?;
    super::fields::fill_text_fields(&page, title, description, link).await?;
    super::board::choose_board(&page, board_url).await?;
    super::publish::publish_pin(&page).await?;

    Ok(())
}
