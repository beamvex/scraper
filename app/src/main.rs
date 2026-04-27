use anyhow::{Context, Result, bail};
use chromiumoxide::browser::Browser;
use futures::StreamExt;
use rand::seq::IndexedRandom;
use serde_json::json;
use std::collections::HashSet;
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
        info!("clicking random result");
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
            }
            Err(err) => {
                warn!(error = %err, "failed to generate review article");
            }
        }

        let image_urls: Vec<String> = page
            .evaluate(
                r#"(() => {
  const urls = new Set();
  const normalize = (u) => {
    if (!u) return u;
    if (typeof u !== 'string') return u;
    // Try to strip Amazon sizing segments like: _AC_SX679_ or _SS40_
    // Keep extension.
    return u.replace(/\._[A-Z0-9,]+_\./, '.');
  };
  const add = (u) => {
    if (!u) return;
    if (typeof u !== 'string') return;
    if (!u.startsWith('http')) return;
    urls.add(normalize(u));
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

  // Amazon product gallery thumbnails
  document
    .querySelectorAll('#altImages img')
    .forEach((img) => {
      add(img.getAttribute('data-old-hires'));
      add(img.getAttribute('data-src'));
      add(img.currentSrc);
      add(img.src);
    });

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

        let mut seen = HashSet::new();
        let image_urls: Vec<String> = image_urls
            .into_iter()
            .filter(|u| seen.insert(u.clone()))
            .collect();

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

async fn load_chatgpt_key() -> Result<String> {
    let home = std::env::var("HOME").context("HOME env var not set")?;
    let key_path: PathBuf = [home.as_str(), ".chatgptkey"].iter().collect();
    let key = tokio::fs::read_to_string(&key_path)
        .await
        .with_context(|| format!("failed to read key file at {}", key_path.display()))?;
    let key = key.trim().to_string();
    if key.is_empty() {
        bail!("chatgpt key file is empty: {}", key_path.display());
    }
    Ok(key)
}

async fn generate_review_article_html(
    api_key: &str,
    product_title: &str,
    product_url: Option<&str>,
    product_page_html: &str,
) -> Result<String> {
    let mut html = product_page_html;
    const MAX_CHARS: usize = 120_000;
    if html.len() > MAX_CHARS {
        html = &html[..MAX_CHARS];
    }

    let url_line = product_url
        .map(|u| format!("Product URL: {}\n", u))
        .unwrap_or_default();

    let prompt = format!(
        "You are an expert consumer tech reviewer. Write a review article that is about a 4-minute read.\n\n\
Output requirements:\n\
- Output valid HTML only (no Markdown).\n\
- Use <article> with a single <h1>, then sections with <h2>.\n\
- Include: overview, key features, who it's for, pros/cons lists, pricing/value discussion, and verdict.\n\
- Do not mention that you were given raw HTML; infer details from the provided page.\n\
- If specs are unclear, state assumptions cautiously.\n\n\
Title: {}\n\
{}\n\
Product detail page HTML (truncated if needed):\n{}",
        product_title, url_line, html
    );

    let body = json!({
        "model": "gpt-4o-mini",
        "messages": [
            {"role": "user", "content": prompt}
        ],
        "temperature": 0.7
    });

    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .context("failed to call OpenAI chat completions")?;

    let status = resp.status();
    let v: serde_json::Value = resp
        .json()
        .await
        .context("failed to parse OpenAI response JSON")?;

    if !status.is_success() {
        bail!("OpenAI request failed ({}): {}", status, v);
    }

    let content = v
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c0| c0.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .map(|s| s.trim().to_string());

    match content {
        Some(c) if !c.is_empty() => Ok(c),
        _ => bail!("OpenAI response did not contain choices[0].message.content"),
    }
}
