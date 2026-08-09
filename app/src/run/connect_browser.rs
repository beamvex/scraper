use anyhow::Result;
use chromiumoxide::browser::Browser;
use futures::StreamExt;
use tracing::info;

pub async fn connect_browser() -> Result<Browser> {
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
    Ok(browser)
}
