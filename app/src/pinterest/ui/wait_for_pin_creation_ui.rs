use anyhow::{bail, Result};
use chromiumoxide::page::Page;

pub async fn wait_for_pin_creation_ui(page: &Page) -> Result<()> {
    if super::poll_pin_creation_ui::poll_pin_creation_ui(page).await? {
        return Ok(());
    }
    let url = page.url().await.ok().flatten().unwrap_or_default();
    super::write_wait_debug::write_wait_debug(page).await;
    bail!("pinterest pin creation UI did not render (url: {})", url)
}
