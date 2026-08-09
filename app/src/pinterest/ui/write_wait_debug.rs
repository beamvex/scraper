use chromiumoxide::page::Page;
use std::path::Path;
use tracing::warn;

const WAIT_DEBUG_JS: &str = r#"(() => {
  const text = (document.documentElement && document.documentElement.innerText) ? document.documentElement.innerText : '';
  const sample = (sel) => Array.from(document.querySelectorAll(sel)).slice(0, 40).map(el => ({
    tag: (el.tagName||'').toLowerCase(), type: el.getAttribute('type') || '',
    role: el.getAttribute('role') || '', aria: el.getAttribute('aria-label') || '',
    name: el.getAttribute('name') || '', id: el.getAttribute('id') || '',
    placeholder: el.getAttribute('placeholder') || '',
    contenteditable: el.getAttribute('contenteditable') || '',
    text: (el.innerText || '').trim().slice(0, 120),
  }));
  return { href: location.href, title: document.title, readyState: document.readyState,
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

pub(super) async fn write_wait_debug(page: &Page) {
    if let Ok(html) = page.content().await {
        let p = Path::new("../data/pinterest_upload_debug.html");
        match tokio::fs::write(p, html).await {
            Err(e) => warn!(error = %e, path = %p.display(), "failed to write pinterest debug html"),
            Ok(_) => warn!(path = %p.display(), "wrote pinterest debug html"),
        }
    }
    if let Some(v) = page.evaluate(WAIT_DEBUG_JS).await.ok()
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
