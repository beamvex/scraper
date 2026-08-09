use chromiumoxide::cdp::browser_protocol::log::EventEntryAdded;
use chromiumoxide::cdp::js_protocol::runtime::EventConsoleApiCalled;
use chromiumoxide::page::Page;
use futures::StreamExt;

pub(super) async fn spawn_browser_logger(page: &Page) -> anyhow::Result<()> {
    let mut log_events = page.event_listener::<EventEntryAdded>().await?;
    let mut console_events = page.event_listener::<EventConsoleApiCalled>().await?;
    tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(ev) = log_events.next() => {
                    tracing::info!(level = ?ev.entry.level, source = ?ev.entry.source, url = ?ev.entry.url, "[browser log] {}", ev.entry.text);
                }
                Some(ev) = console_events.next() => {
                    let args: Vec<String> = ev.args.iter()
                        .map(|a| a.value.as_ref().map(|v| v.to_string()).or_else(|| a.description.clone()).unwrap_or_default())
                        .collect();
                    tracing::info!(r#type = ?ev.r#type, "[console] {}", args.join(" "));
                }
                else => break,
            }
        }
    });
    Ok(())
}
