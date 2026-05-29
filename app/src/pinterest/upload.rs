use anyhow::{Result, bail};
use chromiumoxide::cdp::browser_protocol::dom::SetFileInputFilesParams;
use chromiumoxide::page::Page;
use std::path::Path;
use std::time::Duration;
use tracing::{info, warn};

pub(super) async fn upload_image(page: &Page, image_path: &Path) -> Result<()> {
    info!(path = %image_path.display(), "uploading pinterest pin image");

    // Try common file input selectors.
    let selectors = ["input[type='file']", "input[type=\"file\"]"];

    // Pinterest often loads the input asynchronously; poll for a while.
    for _ in 0..40 {
        for sel in selectors {
            if let Ok(el) = page.find_element(sel).await {
                let abs = image_path
                    .canonicalize()
                    .unwrap_or_else(|_| image_path.to_path_buf());
                let files = vec![abs.display().to_string()];
                let cmd = SetFileInputFilesParams::builder()
                    .files(files)
                    .backend_node_id(el.backend_node_id)
                    .build()
                    .map_err(|e| anyhow::anyhow!(e))?;

                if let Err(err) = page.execute(cmd).await {
                    warn!(selector = sel, error = %err, "failed to set pinterest file input files");
                    continue;
                }
                tokio::time::sleep(Duration::from_millis(4500)).await;
                return Ok(());
            }
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    let url = page.url().await.ok().flatten().unwrap_or_default();
    warn!(%url, "could not find pinterest image upload input");

    if let Ok(html) = page.content().await {
        let debug_path = Path::new("../data/pinterest_upload_debug.html");
        if let Err(err) = tokio::fs::write(debug_path, html).await {
            warn!(error = %err, path = %debug_path.display(), "failed to write pinterest debug html");
        } else {
            warn!(path = %debug_path.display(), "wrote pinterest debug html");
        }
    }

    let debug_js = r#"(() => {
  const inputs = Array.from(document.querySelectorAll('input')).slice(0, 200).map(el => ({
    type: el.getAttribute('type') || '',
    name: el.getAttribute('name') || '',
    id: el.id || '',
    aria: el.getAttribute('aria-label') || '',
    accept: el.getAttribute('accept') || '',
    class: el.className || ''
  }));
  const buttons = Array.from(document.querySelectorAll('button')).slice(0, 200).map(el => ({
    text: (el.innerText || '').trim().slice(0, 120),
    aria: el.getAttribute('aria-label') || '',
    id: el.id || '',
    class: el.className || ''
  }));
  const iframes = Array.from(document.querySelectorAll('iframe')).slice(0, 50).map(el => ({
    src: el.getAttribute('src') || '',
    id: el.id || '',
    title: el.getAttribute('title') || ''
  }));
  const text = (document.documentElement && document.documentElement.innerText) ? document.documentElement.innerText : '';
  return {
    href: location.href,
    title: document.title,
    readyState: document.readyState,
    inputCount: document.querySelectorAll('input').length,
    fileInputCount: document.querySelectorAll('input[type=file]').length,
    iframeCount: document.querySelectorAll('iframe').length,
    innerTextSnippet: text.trim().slice(0, 1200),
    inputs,
    buttons,
    iframes,
  };
})()"#;

    if let Ok(v) = page
        .evaluate(debug_js)
        .await?
        .into_value::<serde_json::Value>()
    {
        let debug_path = Path::new("../data/pinterest_upload_debug.json");
        if let Ok(s) = serde_json::to_string_pretty(&v) {
            if let Err(err) = tokio::fs::write(debug_path, s).await {
                warn!(error = %err, path = %debug_path.display(), "failed to write pinterest debug json");
            } else {
                warn!(path = %debug_path.display(), "wrote pinterest debug json");
            }
        }
    }

    bail!("could not find pinterest image upload input")
}
