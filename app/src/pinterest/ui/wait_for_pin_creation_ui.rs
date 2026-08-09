use anyhow::{bail, Result};
use chromiumoxide::page::Page;
use std::path::Path;
use std::time::Duration;
use tracing::warn;

pub async fn wait_for_pin_creation_ui(page: &Page) -> Result<()> {
    for _ in 0..360 {
        let counts: (u64, u64, u64, u64, u64) = page
            .evaluate(
                r#"(() => {
  const inputs = document.querySelectorAll('input,textarea').length;
  const editables = document.querySelectorAll('[contenteditable="true"]').length;
  const fileInputs = document.querySelectorAll('input[type=file]').length;
  const buttons = document.querySelectorAll('button,div[role=button],a[role=button]').length;
  const iframes = document.querySelectorAll('iframe').length;
  return [inputs, editables, fileInputs, buttons, iframes];
})()"#,
            )
            .await?
            .into_value::<(u64, u64, u64, u64, u64)>()
            .unwrap_or((0, 0, 0, 0, 0));

        if counts.2 >= 1 {
            return Ok(());
        }

        let formish = counts.0.saturating_add(counts.1);
        if counts.3 >= 6 || (counts.3 >= 4 && formish >= 3) {
            return Ok(());
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    let url = page.url().await.ok().flatten().unwrap_or_default();

    if let Ok(html) = page.content().await {
        let debug_path = Path::new("../data/pinterest_upload_debug.html");
        if let Err(err) = tokio::fs::write(debug_path, html).await {
            warn!(error = %err, path = %debug_path.display(), "failed to write pinterest debug html");
        } else {
            warn!(path = %debug_path.display(), "wrote pinterest debug html");
        }
    }

    let debug_js = r#"(() => {
  const text = (document.documentElement && document.documentElement.innerText) ? document.documentElement.innerText : '';
  const sample = (sel) => Array.from(document.querySelectorAll(sel)).slice(0, 40).map(el => ({
    tag: (el.tagName||'').toLowerCase(),
    type: el.getAttribute('type') || '',
    role: el.getAttribute('role') || '',
    aria: el.getAttribute('aria-label') || '',
    name: el.getAttribute('name') || '',
    id: el.getAttribute('id') || '',
    placeholder: el.getAttribute('placeholder') || '',
    contenteditable: el.getAttribute('contenteditable') || '',
    text: (el.innerText || '').trim().slice(0, 120),
  }));
  return {
    href: location.href,
    title: document.title,
    readyState: document.readyState,
    inputCount: document.querySelectorAll('input,textarea').length,
    editableCount: document.querySelectorAll('[contenteditable="true"]').length,
    fileInputCount: document.querySelectorAll('input[type=file]').length,
    buttonCount: document.querySelectorAll('button').length,
    roleButtonCount: document.querySelectorAll('button,div[role=button],a[role=button]').length,
    iframeCount: document.querySelectorAll('iframe').length,
    innerTextSnippet: text.trim().slice(0, 2000),
    inputsSample: sample('input,textarea,[contenteditable="true"]'),
    buttonsSample: sample('button,div[role=button],a[role=button]'),
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

    bail!("pinterest pin creation UI did not render (url: {})", url)
}
