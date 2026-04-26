use anyhow::Result;
use chromiumoxide::browser::Browser;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    let (browser, mut handler) = Browser::connect("http://127.0.0.1:9222").await?;

    tokio::spawn(async move {
        while let Some(_event) = handler.next().await {
            let _ = _event;
        }
    });

    let page = browser.new_page("https://amazon.com").await?;
    page.wait_for_navigation().await?;

    Ok(())
}
