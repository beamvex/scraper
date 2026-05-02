mod openai;
mod computer_queries;
mod ifttt;
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
use crate::ifttt::trigger_new_post;
use crate::util::sanitize_path_component;
use crate::wordpress_com::publish_review_html_to_wordpress_com;

const DEBUG: bool = false;
const AMAZON_BASE: &str = "https://www.amazon.com";

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
        let Some(url) = product_url else {
            warn!("no href found on selected result");
            page.close().await?;
            return Ok(());
        };

        if url.is_empty() {
            warn!("selected result href is empty");
            page.close().await?;
            return Ok(());
        }

        let target_url = resolve_amazon_url(&url);
        info!(%target_url, "navigating to product url");

        tokio::time::timeout(std::time::Duration::from_secs(60), page.goto(target_url))
            .await
            .map_err(|_| anyhow::anyhow!("navigation timeout"))??;

        let product_name: String = page
            .evaluate("document.title")
            .await?
            .into_value::<String>()?;
        let product_folder_name = sanitize_path_component(&product_name);
        let product_dir: PathBuf = ["/data", &product_folder_name].iter().collect();

        tokio::fs::create_dir_all(&product_dir).await?;
        info!(dir = %product_dir.display(), name = %product_name, "created product directory");

        let html = page.content().await?;
        let html_path = product_dir.join("page.html");
        tokio::fs::write(&html_path, html).await?;
        info!(path = %html_path.display(), "saved page html");

        let main_image_url: Option<String> = page
            .evaluate(
                r#"(() => {
  const landing = document.querySelector('#landingImage');
  if (!landing) return null;

  const dyn = landing.getAttribute('data-a-dynamic-image');
  if (!dyn) return landing.src || null;

  try {
    const obj = JSON.parse(dyn);
    let bestUrl = null;
    let bestScore = -1;
    for (const [url, dims] of Object.entries(obj)) {
      if (!url || typeof url !== 'string') continue;
      if (!Array.isArray(dims) || dims.length < 2) continue;
      const w = Number(dims[0]) || 0;
      const h = Number(dims[1]) || 0;
      const score = w * h;
      if (score > bestScore) {
        bestScore = score;
        bestUrl = url;
      }
    }
    return bestUrl || landing.src || null;
  } catch (e) {
    return landing.src || null;
  }
})()"#,
            )
            .await?
            .into_value::<Option<String>>()?;

        let client = reqwest::Client::new();

        if let Some(url) = main_image_url.as_deref() {
            info!(%url, "downloading main product image");
            let path = product_dir.join("main.jpg");
            match client.get(url).send().await {
                Ok(resp) => match resp.bytes().await {
                    Ok(bytes) => {
                        if let Err(err) = tokio::fs::write(&path, bytes).await {
                            warn!(%url, path = %path.display(), error = %err, "failed to write main image");
                        } else {
                            info!(%url, path = %path.display(), "saved main image");
                        }
                    }
                    Err(err) => {
                        warn!(%url, error = %err, "failed to read main image body");
                    }
                },
                Err(err) => {
                    warn!(%url, error = %err, "failed to download main image");
                }
            }
        } else {
            warn!("no main product image url found");
        }

        if DEBUG {
            page.close().await?;
            info!("closed product tab");
            return Ok(());
        }

        let openai_api_key = load_chatgpt_key().await?;


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

                let main_image_path = product_dir.join("main.jpg");
                match publish_review_html_to_wordpress_com(
                    &review_path,
                    query,
                    main_image_path.exists().then_some(main_image_path.as_path()),
                )
                .await
                {
                    Ok(post_url) => {
                        info!("created WordPress.com post");
                        if let Err(err) = trigger_new_post(&product_name, post_url.as_deref()).await {
                            warn!(error = %err, "failed to trigger IFTTT webhook");
                        }
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

fn resolve_amazon_url(href: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") {
        href.to_string()
    } else if href.starts_with('/') {
        format!("{}{}", AMAZON_BASE, href)
    } else {
        format!("{}/{}", AMAZON_BASE, href)
    }
}
