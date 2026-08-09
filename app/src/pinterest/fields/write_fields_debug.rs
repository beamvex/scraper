use chromiumoxide::page::Page;
use std::path::Path;
use tracing::warn;

const DEBUG_FIELDS_JS: &str = r#"(() => {
  const els = Array.from(document.querySelectorAll('input,textarea,[contenteditable="true"]')).slice(0, 250);
  const fields = els.map(el => ({
    tag: (el.tagName||'').toLowerCase(), type: el.getAttribute('type') || '',
    name: el.getAttribute('name') || '', id: el.getAttribute('id') || '',
    aria: el.getAttribute('aria-label') || '', placeholder: el.getAttribute('placeholder') || '',
    contenteditable: el.getAttribute('contenteditable') || '',
    class: (el.className||'').toString().slice(0,120),
  }));
  const text = (document.documentElement && document.documentElement.innerText) ? document.documentElement.innerText : '';
  return { href: location.href, title: document.title, readyState: document.readyState,
    fieldCount: els.length, fields, innerTextSnippet: text.trim().slice(0, 2000) };
})()"#;

pub(super) async fn write_fields_debug(
    page: &Page,
    title_ok: bool,
    desc_ok: bool,
    link_ok: bool,
    has_desc_js: bool,
) {
    if !title_ok { warn!("could not find pinterest title field; continuing"); }
    if has_desc_js && !desc_ok { warn!("could not find pinterest description field; continuing"); }
    if !link_ok { warn!("could not find pinterest link field; continuing"); }
    if title_ok && link_ok { return; }
    if let Some(v) = page.evaluate(DEBUG_FIELDS_JS).await.ok()
        .and_then(|e| e.into_value::<serde_json::Value>().ok())
    {
        let p = Path::new("../data/pinterest_fields_debug.json");
        if let Ok(s) = serde_json::to_string_pretty(&v) {
            match tokio::fs::write(p, s).await {
                Err(e) => warn!(error = %e, path = %p.display(), "failed to write pinterest fields debug json"),
                Ok(_) => warn!(path = %p.display(), "wrote pinterest fields debug json"),
            }
        }
    }
}
