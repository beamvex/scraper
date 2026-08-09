use anyhow::Result;
use chromiumoxide::browser::Browser;
use crate::util::sanitize_path_component;
use tracing::warn;

pub async fn run_once(browser: &Browser) -> Result<()> {
    let Some((query, target_url, page)) = super::search_and_pick::search_and_pick(browser).await? else {
        return Ok(());
    };
    let (product_name, product_dir, html_path) = super::navigate_product::navigate_product(&page, &target_url).await?;
    let product_folder_name = sanitize_path_component(&product_name);
    let image_path = super::download_image::download_image(&page, &product_dir).await;
    if crate::DEBUG { page.close().await?; return Ok(()); }
    let openai_key = crate::openai::load_chatgpt_key().await?;
    let product_url = page.url().await.ok().flatten();
    let (review_html, review_path) = match super::generate_review::generate_review(
        &product_name, &product_folder_name, &html_path, &openai_key, product_url.as_deref(),
    ).await {
        Ok(r) => r,
        Err(e) => { warn!(error = %e, "failed to generate review article"); page.close().await?; return Ok(()); }
    };
    super::publish_all::publish_all(browser, query, &product_name, &review_html, &review_path, image_path.as_deref()).await?;
    page.close().await?;
    Ok(())
}
