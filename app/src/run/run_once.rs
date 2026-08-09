use anyhow::Result;
use chromiumoxide::browser::Browser;

pub async fn run_once(browser: &Browser) -> Result<()> {
    let Some((_query, target_url, page)) = super::search_and_pick::search_and_pick(browser).await? else {
        return Ok(());
    };
    let (_product_name, product_dir, _html_path) = super::navigate_product::navigate_product(&page, &target_url).await?;
    let _image_path = super::download_image::download_image(&page, &product_dir).await;
    if crate::DEBUG { page.close().await?; return Ok(()); }
    page.close().await?;
    Ok(())
}
