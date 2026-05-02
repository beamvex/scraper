mod openai;
mod computer_queries;
mod util;
mod wordpress_com;

use anyhow::Result;
use chromiumoxide::browser::Browser;
use futures::StreamExt;
use rand::seq::IndexedRandom;
use std::path::PathBuf;
use tracing::{info, warn};

use crate::openai::{generate_review_article_html, load_chatgpt_key};
use crate::computer_queries::COMPUTER_QUERIES;
use crate::util::sanitize_path_component;
use crate::wordpress_com::publish_review_html_to_wordpress_com;

#[tokio::main]
async fn main() -> Result<()> {
    if let Ok(home) = std::env::var("HOME") {
        let _ = dotenvy::from_filename(format!("{}/.env", home));
    }

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("connecting to existing chrome");
    let (browser, mut handler) = Browser::connect("http://127.0.0.1:9222").await?;
    info!(ws = %browser.websocket_address(), "connected");

    tokio::spawn(async move {
        info!("handler task started");
        while let Some(_event) = handler.next().await {
            let _ = _event;
        }
        info!("handler task finished");
    });

    let mut rng = rand::rng();
    let query = COMPUTER_QUERIES.choose(&mut rng).unwrap();
    info!(%query, "selected search query");
    let search_url = format!("https://www.amazon.com/s?k={}", query.replace(' ', "+"));
    info!(%search_url, "opening search url");

    let page = browser.new_page(search_url).await?;

    tokio::time::sleep(std::time::Duration::from_millis(750)).await;

    let mut results = page.find_elements("a.a-link-normal.s-no-outline").await?;

    if results.is_empty() {
        warn!("primary selector returned 0 results, trying fallback selector");
        results = page.find_elements("h2 a.a-link-normal").await?;
    }

    info!(count = results.len(), "found result link candidates");

    if let Some(product) = results.choose(&mut rng) {
        let product_url = product.attribute("href").await?;
        if let Some(url) = &product_url {
            if url.is_empty() {
                info!("clicking random result (empty url)");
            } else {
                info!("clicking random result {url}");
            }
        } else {
            info!("clicking random result (no url found)");
        }
        product.click().await?;
        page.wait_for_navigation().await?;

        let product_name: String = page
            .evaluate("document.title")
            .await?
            .into_value::<String>()?;
        let product_folder_name = sanitize_path_component(&product_name);
        let product_dir: PathBuf = ["/data", &product_folder_name].iter().collect();

        let openai_api_key = load_chatgpt_key().await?;

        tokio::fs::create_dir_all(&product_dir).await?;
        info!(dir = %product_dir.display(), name = %product_name, "created product directory");

        let html = page.content().await?;
        let html_path = product_dir.join("page.html");
        tokio::fs::write(&html_path, html).await?;
        info!(path = %html_path.display(), "saved page html");

        info!("calling openai to generate review article");
        let product_url = page.url().await.ok().flatten();
        match generate_review_article_html(
            &openai_api_key,
            &product_name,
            product_url.as_deref(),
            &tokio::fs::read_to_string(&html_path)
                .await
                .unwrap_or_default(),
        )
        .await
        {
            Ok(review_html) => {
                let reviews_dir: PathBuf = ["/data", "reviews"].iter().collect();
                tokio::fs::create_dir_all(&reviews_dir).await?;
                let review_path = reviews_dir.join(format!("{}.html", product_folder_name));
                tokio::fs::write(&review_path, review_html).await?;
                info!(path = %review_path.display(), "saved review article");

                match publish_review_html_to_wordpress_com(&review_path).await {
                    Ok(()) => {
                        info!("created WordPress.com post");
                    }
                    Err(err) => {
                        warn!(error = %err, "failed to create WordPress.com post");
                    }
                }
            }
            Err(err) => {
                warn!(error = %err, "failed to generate review article");
            }
        }

        page.close().await?;
        info!("closed product tab");
    } else {
        warn!("no results found to click");
    }

    Ok(())
}
