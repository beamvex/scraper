use chromiumoxide::page::Page;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

pub(super) async fn download_image(page: &Page, product_dir: &Path) -> Option<PathBuf> {
    let url = match super::get_image_url::get_image_url(page).await.ok().flatten() {
        Some(u) => u,
        None => { warn!("no main product image url found"); return None; }
    };
    info!(%url, "downloading main product image");
    let path = product_dir.join("main.jpg");
    match reqwest::Client::new().get(&url).send().await {
        Ok(resp) => match resp.bytes().await {
            Ok(bytes) => match tokio::fs::write(&path, bytes).await {
                Ok(_) => { info!(%url, path = %path.display(), "saved main image"); Some(path) }
                Err(e) => { warn!(%url, path = %path.display(), error = %e, "failed to write main image"); None }
            },
            Err(e) => { warn!(%url, error = %e, "failed to read main image body"); None }
        },
        Err(e) => { warn!(%url, error = %e, "failed to download main image"); None }
    }
}
