use anyhow::Result;
use chromiumoxide::browser::Browser;
use futures::StreamExt;
use rand::seq::IndexedRandom;
use std::path::PathBuf;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<()> {
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

    let computer_queries = [
        "mechanical keyboard",
        "gaming mouse",
        "usb c hub",
        "nvme ssd",
        "27 inch monitor",
        "laptop stand",
        "wifi 6 router",
        "webcam 1080p",
        "noise cancelling headset",
        "raspberry pi kit",
        "usb microphone",
        "stream deck",
        "ergonomic mouse",
        "keyboard wrist rest",
        "monitor arm",
        "laptop dock",
        "thunderbolt dock",
        "portable monitor",
        "external ssd",
        "ssd enclosure",
        "usb c cable",
        "displayport cable",
        "hdmi cable 2.1",
        "network switch",
        "ethernet cable cat6",
        "nas enclosure",
        "ups battery backup",
        "gaming chair",
        "desk mat",
        "standing desk",
        "soldering kit",
        "arduino starter kit",
        "thermal paste",
        "cpu cooler",
        "pc case fan",
        "graphics card support bracket",
        "m.2 heatsink",
        "bluetooth adapter",
        "wifi adapter",
        "usb c sd card reader",
        "sd card",
        "keycap set",
        "mechanical keyboard switch",
        "mouse pad",
        "webcam mount",
        "laptop privacy screen",
        "cable management",
        "desk lamp",
    ];

    let mut rng = rand::rng();
    let query = computer_queries.choose(&mut rng).unwrap();
    info!(%query, "selected search query");
    let search_url = format!(
        "https://www.amazon.com/s?k={}",
        query.replace(' ', "+")
    );
    info!(%search_url, "opening search url");

    let page = browser.new_page(search_url).await?;

    tokio::time::sleep(std::time::Duration::from_millis(750)).await;

    let mut results = page
        .find_elements("a.a-link-normal.s-no-outline")
        .await?;

    if results.is_empty() {
        warn!("primary selector returned 0 results, trying fallback selector");
        results = page.find_elements("h2 a.a-link-normal").await?;
    }

    info!(count = results.len(), "found result link candidates");

    if let Some(product) = results.choose(&mut rng) {
        info!("clicking random result");
        product.click().await?;
        page.wait_for_navigation().await?;

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

        let image_urls: Vec<String> = page
            .evaluate(
                r#"(() => {
  const urls = new Set();
  const add = (u) => {
    if (!u) return;
    if (typeof u !== 'string') return;
    if (!u.startsWith('http')) return;
    urls.add(u);
  };

  const landing = document.querySelector('#landingImage');
  if (landing) {
    try {
      const dyn = landing.getAttribute('data-a-dynamic-image');
      if (dyn) {
        const obj = JSON.parse(dyn);
        Object.keys(obj).forEach(add);
      }
    } catch (e) {}
    add(landing.src);
  }

  document
    .querySelectorAll('img')
    .forEach((img) => {
      add(img.currentSrc);
      add(img.src);
      add(img.getAttribute('data-old-hires'));
      add(img.getAttribute('data-src'));
    });

  return Array.from(urls).slice(0, 30);
})()"#,
            )
            .await?
            .into_value::<Vec<String>>()?;

        info!(count = image_urls.len(), "extracted image urls");

        let client = reqwest::Client::new();
        for (idx, url) in image_urls.iter().enumerate() {
            let filename = format!("image_{:02}.jpg", idx + 1);
            let path = product_dir.join(filename);

            match client.get(url).send().await {
                Ok(resp) => match resp.bytes().await {
                    Ok(bytes) => {
                        if let Err(err) = tokio::fs::write(&path, bytes).await {
                            warn!(%url, path = %path.display(), error = %err, "failed to write image");
                        } else {
                            info!(%url, path = %path.display(), "saved image");
                        }
                    }
                    Err(err) => {
                        warn!(%url, error = %err, "failed to read image body");
                    }
                },
                Err(err) => {
                    warn!(%url, error = %err, "failed to download image");
                }
            }
        }

        if let Ok(Some(url)) = page.url().await {
            info!(%url, "completed product page capture");
        } else {
            info!("completed product page capture");
        }

        page.close().await?;
        info!("closed product tab");
    } else {
        warn!("no results found to click");
    }

    Ok(())
}

fn sanitize_path_component(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        let keep = c.is_ascii_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.');
        if keep {
            out.push(c);
        } else {
            out.push('_');
        }
    }

    let out = out.trim().trim_matches('.').to_string();
    let out = out.split_whitespace().collect::<Vec<_>>().join(" ");

    if out.is_empty() {
        "unknown-product".to_string()
    } else {
        out.chars().take(120).collect()
    }
}
