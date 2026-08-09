use anyhow::{bail, Result};
use chromiumoxide::page::Page;
use std::path::Path;
use tracing::info;

pub async fn upload_image(page: &Page, image_path: &Path) -> Result<()> {
    info!(path = %image_path.display(), "uploading pinterest pin image");
    if super::try_set_file_input::try_set_file_input(page, image_path).await? {
        return Ok(());
    }
    let url = page.url().await.ok().flatten().unwrap_or_default();
    super::write_upload_debug::write_upload_debug(page, &url).await;
    bail!("could not find pinterest image upload input")
}
