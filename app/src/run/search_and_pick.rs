use anyhow::Result;
use chromiumoxide::browser::Browser;
use chromiumoxide::page::Page;
use rand::seq::IndexedRandom;
use tracing::{info, warn};

use crate::computer_queries::COMPUTER_QUERIES;
use crate::util::resolve_amazon_url;

pub(super) async fn search_and_pick(browser: &Browser) -> Result<Option<(&'static str, String, Page)>> {
    let mut rng = rand::rng();
    let query: &'static str = COMPUTER_QUERIES.choose(&mut rng).unwrap();
    info!(%query, "selected search query");
    let page = browser.new_page(format!("https://www.amazon.com/s?k={}", query.replace(' ', "+"))).await?;
    tokio::time::sleep(std::time::Duration::from_millis(750)).await;
    let mut results = page.find_elements("a.a-link-normal.s-no-outline").await?;
    if results.is_empty() {
        warn!("primary selector returned 0 results, trying fallback selector");
        results = page.find_elements("h2 a.a-link-normal").await?;
    }
    info!(count = results.len(), "found result link candidates");
    let Some(product) = results.choose(&mut rng) else {
        warn!("no results found to click"); return Ok(None);
    };
    let product_url = product.attribute("href").await?;
    let Some(url) = product_url.filter(|u| !u.is_empty()) else {
        warn!("no href found on selected result"); page.close().await?; return Ok(None);
    };
    Ok(Some((query, resolve_amazon_url(&url), page)))
}
