use anyhow::Result;
use chromiumoxide::page::Page;

pub async fn is_login_interstitial(page: &Page) -> Result<bool> {
    let url = page.url().await.ok().flatten().unwrap_or_default();
    if url.contains("/login") {
        return Ok(true);
    }

    let html: String = page
        .evaluate("document.documentElement.innerText")
        .await?
        .into_value::<String>()
        .unwrap_or_default();

    Ok(html.to_lowercase().contains("log in") && html.to_lowercase().contains("password"))
}
