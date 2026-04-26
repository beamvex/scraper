use anyhow::Result;
use chromiumoxide::browser::Browser;
use futures::StreamExt;
use rand::seq::IndexedRandom;
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
        if let Ok(Some(url)) = page.url().await {
            info!(%url, "navigated to product page");
        } else {
            info!("navigated to product page (url unavailable)");
        }
    } else {
        warn!("no results found to click");
    }

    Ok(())
}
