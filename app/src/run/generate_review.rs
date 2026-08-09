use anyhow::Result;
use std::path::{Path, PathBuf};
use tracing::info;

pub(super) async fn generate_review(
    product_name: &str,
    product_folder_name: &str,
    html_path: &Path,
    openai_api_key: &str,
    product_url: Option<&str>,
) -> Result<(String, PathBuf)> {
    let review_html = crate::openai::generate_review_article_html(
        openai_api_key, product_name, product_url,
        &tokio::fs::read_to_string(html_path).await.unwrap_or_default(),
    ).await?;
    let reviews_dir: PathBuf = ["../data", "reviews"].iter().collect();
    tokio::fs::create_dir_all(&reviews_dir).await?;
    let review_path = reviews_dir.join(format!("{}.html", product_folder_name));
    tokio::fs::write(&review_path, &review_html).await?;
    info!(path = %review_path.display(), "saved review article");
    Ok((review_html, review_path))
}
