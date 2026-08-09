use anyhow::{bail, Result};
use chromiumoxide::page::Page;
use std::path::Path;
use tracing::info;

pub async fn publish_pin(page: &Page) -> Result<()> {
    if !super::try_click_publish::try_click_publish(page).await? {
        let _ = super::super::write_debug_snapshot(page, Path::new("../data/pinterest_publish_debug.json")).await;
        bail!("could not find pinterest publish/save button")
    }
    tokio::time::sleep(std::time::Duration::from_millis(3500)).await;
    let _ = super::super::write_debug_snapshot(page, Path::new("../data/pinterest_publish_after_debug.json")).await;
    info!("attempted to publish pinterest pin");
    Ok(())
}
