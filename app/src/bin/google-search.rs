use anyhow::{Context, Result};
use chromiumoxide::browser::Browser;
use futures::StreamExt;
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

    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

    let results: serde_json::Value = page
        .evaluate(
            r#"(() => {
  const out = [];
  const blocks = Array.from(document.querySelectorAll('div.g')).slice(0, 8);
  for (const b of blocks) {
    const a = b.querySelector('a[href]');
    const h3 = b.querySelector('h3');
    const title = h3 ? (h3.innerText || '').trim() : '';
    const href = a ? (a.href || '') : '';
    if (title && href) out.push({ title, href });
  }
  return out;
})()"#,
        )
        .await?
        .into_value()?;

    if let Some(arr) = results.as_array() {
        if arr.is_empty() {
            warn!("no results found (possible consent/captcha page)");
        }
        for (idx, item) in arr.iter().enumerate() {
            let title = item.get("title").and_then(|v| v.as_str()).unwrap_or("");
            let href = item.get("href").and_then(|v| v.as_str()).unwrap_or("");
            println!("{}\t{}\t{}", idx + 1, title, href);
        }
    } else {
        warn!("unexpected results shape: {}", results);
    }

    Ok(())
}
