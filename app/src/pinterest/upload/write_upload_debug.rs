use chromiumoxide::page::Page;
use std::path::Path;
use tracing::warn;

const UPLOAD_DEBUG_JS: &str = r#"(() => {
  const inputs = Array.from(document.querySelectorAll('input')).slice(0, 200).map(el => ({
    type: el.getAttribute('type') || '', name: el.getAttribute('name') || '',
    id: el.id || '', aria: el.getAttribute('aria-label') || '',
    accept: el.getAttribute('accept') || '', class: el.className || ''
  }));
  const buttons = Array.from(document.querySelectorAll('button')).slice(0, 200).map(el => ({
    text: (el.innerText || '').trim().slice(0, 120), aria: el.getAttribute('aria-label') || '',
    id: el.id || '', class: el.className || ''
  }));
  const iframes = Array.from(document.querySelectorAll('iframe')).slice(0, 50).map(el => ({
    src: el.getAttribute('src') || '', id: el.id || '', title: el.getAttribute('title') || ''
  }));
  const text = (document.documentElement && document.documentElement.innerText) ? document.documentElement.innerText : '';
  return { href: location.href, title: document.title, readyState: document.readyState,
    inputCount: document.querySelectorAll('input').length,
    fileInputCount: document.querySelectorAll('input[type=file]').length,
    iframeCount: document.querySelectorAll('iframe').length,
    innerTextSnippet: text.trim().slice(0, 1200), inputs, buttons, iframes,
  };
})()"#;

pub(super) async fn write_upload_debug(page: &Page, url: &str) {
    warn!(%url, "could not find pinterest image upload input");
    if let Ok(html) = page.content().await {
        let p = Path::new("../data/pinterest_upload_debug.html");
        match tokio::fs::write(p, html).await {
            Err(e) => warn!(error = %e, path = %p.display(), "failed to write pinterest debug html"),
            Ok(_) => warn!(path = %p.display(), "wrote pinterest debug html"),
        }
    }
    if let Some(v) = page.evaluate(UPLOAD_DEBUG_JS).await.ok()
        .and_then(|e| e.into_value::<serde_json::Value>().ok())
    {
        let p = Path::new("../data/pinterest_upload_debug.json");
        if let Ok(s) = serde_json::to_string_pretty(&v) {
            match tokio::fs::write(p, s).await {
                Err(e) => warn!(error = %e, path = %p.display(), "failed to write pinterest debug json"),
                Ok(_) => warn!(path = %p.display(), "wrote pinterest debug json"),
            }
        }
    }
}
