use anyhow::Result;
use chromiumoxide::page::Page;
use std::path::Path;
use std::time::Duration;
use tracing::warn;

#[allow(dead_code)]
pub async fn choose_board(page: &Page, board_url: &str) -> Result<()> {
    let board_name = board_url.trim_end_matches('/').split('/').last()
        .unwrap_or("random-thoughts").replace('-', " ");
    if !super::open_board_picker::open_board_picker(page).await? {
        warn!("could not find pinterest board picker; continuing");
        let _ = super::super::write_debug_snapshot(page, Path::new("../data/pinterest_choose_board_debug.json")).await;
        return Ok(());
    }
    tokio::time::sleep(Duration::from_millis(1200)).await;
    super::type_board_name::type_board_name(page, &board_name).await?;
    if !super::pick_board_item::pick_board_item(page, &board_name).await? {
        warn!(board = %board_name, "failed to pick board by name; continuing");
        let _ = super::super::write_debug_snapshot(page, Path::new("../data/pinterest_choose_board_debug.json")).await;
    }
    tokio::time::sleep(Duration::from_millis(800)).await;
    Ok(())
}
