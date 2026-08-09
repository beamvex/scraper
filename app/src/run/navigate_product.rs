use anyhow::Result;
use chromiumoxide::page::Page;
use std::path::PathBuf;
use tracing::info;

use crate::util::sanitize_path_component;

pub(super) async fn navigate_product(page: &Page, target_url: &str) -> Result<(String, PathBuf, PathBuf)> {
    info!(%target_url, "navigating to product url");
    tokio::time::timeout(std::time::Duration::from_secs(60), page.goto(target_url))
        .await.map_err(|_| anyhow::anyhow!("navigation timeout"))??;
    let product_name: String = page.evaluate("document.title").await?.into_value::<String>()?;
    let product_folder_name = sanitize_path_component(&product_name);
    let product_dir: PathBuf = ["../data", &product_folder_name].iter().collect();
    tokio::fs::create_dir_all(&product_dir).await?;
    info!(dir = %product_dir.display(), name = %product_name, "created product directory");
    let html = page.content().await?;
    let html_path = product_dir.join("page.html");
    tokio::fs::write(&html_path, html).await?;
    info!(path = %html_path.display(), "saved page html");
    Ok((product_name, product_dir, html_path))
}
