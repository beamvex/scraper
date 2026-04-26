use anyhow::Result;
use chromiumoxide::browser::Browser;
use futures::StreamExt;
use rand::seq::IndexedRandom;

#[tokio::main]
async fn main() -> Result<()> {
    let (browser, mut handler) = Browser::connect("http://127.0.0.1:9222").await?;

    tokio::spawn(async move {
        while let Some(_event) = handler.next().await {
            let _ = _event;
        }
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
    ];

    let mut rng = rand::rng();
    let query = computer_queries.choose(&mut rng).unwrap();
    let search_url = format!(
        "https://www.amazon.com/s?k={}",
        query.replace(' ', "+")
    );

    let page = browser.new_page(search_url).await?;

    tokio::time::sleep(std::time::Duration::from_millis(750)).await;

    let mut results = page
        .find_elements("a.a-link-normal.s-no-outline")
        .await?;

    if results.is_empty() {
        results = page.find_elements("h2 a.a-link-normal").await?;
    }

    if let Some(product) = results.choose(&mut rng) {
        product.click().await?;
        page.wait_for_navigation().await?;
    }

    Ok(())
}
