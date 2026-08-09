use anyhow::Result;
use chromiumoxide::page::Page;
use std::path::Path;

mod board;
mod fields;
mod maybe_post_pin_to_board;
mod post_pin_to_board;
mod publish;
mod spawn_browser_logger;
mod ui;
mod upload;

#[cfg(test)]
mod tests;

pub use maybe_post_pin_to_board::maybe_post_pin_to_board;

const DEBUG_SNAPSHOT_JS: &str = r#"(() => {
  const text = (document.documentElement && document.documentElement.innerText) ? document.documentElement.innerText : '';
  const sample = (sel) => Array.from(document.querySelectorAll(sel)).slice(0, 60).map(el => ({
    tag: (el.tagName||'').toLowerCase(), type: el.getAttribute('type') || '',
    role: el.getAttribute('role') || '', aria: el.getAttribute('aria-label') || '',
    name: el.getAttribute('name') || '', id: el.getAttribute('id') || '',
    placeholder: el.getAttribute('placeholder') || '',
    contenteditable: el.getAttribute('contenteditable') || '',
    text: (el.innerText || '').trim().slice(0, 140),
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

pub(super) async fn write_debug_snapshot(page: &Page, path: &Path) -> Result<()> {
    let v = page
        .evaluate(DEBUG_SNAPSHOT_JS)
        .await?
        .into_value::<serde_json::Value>()
        .unwrap_or(serde_json::json!({"error": "failed to evaluate debug snapshot"}));
    let s = serde_json::to_string_pretty(&v).unwrap_or_else(|_| "{}".to_string());
    tokio::fs::write(path, s).await?;
    Ok(())
}
