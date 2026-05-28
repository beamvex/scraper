use anyhow::{Context, Result};
use chromiumoxide::browser::Browser;
use futures::StreamExt;
use std::path::PathBuf;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    if let Ok(home) = std::env::var("HOME") {
        let _ = dotenvy::from_filename(format!("{}/.env", home));
    }

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let query = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    if query.trim().is_empty() {
        anyhow::bail!("usage: cargo run --bin google-search -- \"search terms\"");
    }

    info!(%query, "connecting to existing chrome");
    let (browser, mut handler) = Browser::connect("http://127.0.0.1:9222").await?;

    tokio::spawn(async move {
        while let Some(_event) = handler.next().await {
            let _ = _event;
        }
    });

    let url = format!(
        "https://www.google.com/search?q={}",
        urlencoding::encode(&query)
    );
    info!(%url, "opening google search");

    let page = browser
        .new_page(url)
        .await
        .context("failed to open google search page")?;

    let out_dir: PathBuf = ["../data", "google-results"].iter().collect();
    tokio::fs::create_dir_all(&out_dir)
        .await
        .with_context(|| format!("failed to create {}", out_dir.display()))?;

    for page_idx in 0..50usize {
        let start = page_idx * 10;
        let page_url = format!(
            "https://www.google.com/search?q={}&num=10&start={}",
            urlencoding::encode(&query),
            start
        );

        info!(idx = page_idx + 1, %page_url, "loading google results page");
        page.goto(page_url)
            .await
            .context("failed to navigate google results page")?;

        tokio::time::sleep(std::time::Duration::from_millis(1600)).await;

        let html: String = page
            .evaluate("document.documentElement.outerHTML")
            .await?
            .into_value()
            .context("failed to read page html")?;

        if html.len() < 2000 {
            warn!(
                idx = page_idx + 1,
                len = html.len(),
                "captured html is unexpectedly small (possible interstitial/captcha)"
            );
        }

        let path = out_dir.join(format!("page_{:03}.html", page_idx + 1));
        tokio::fs::write(&path, html)
            .await
            .with_context(|| format!("failed to write {}", path.display()))?;
    }

    Ok(())
}
