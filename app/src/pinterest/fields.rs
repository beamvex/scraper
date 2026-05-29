use anyhow::Result;
use chromiumoxide::page::Page;
use std::path::Path;
use std::time::Duration;
use tracing::warn;

pub(super) async fn fill_text_fields(
    page: &Page,
    title: &str,
    description: Option<&str>,
    link: &str,
) -> Result<()> {
    // Pinterest frequently changes selectors; do this via JS and dispatch input events.
    let description = description
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| {
            if s.chars().count() > 450 {
                s.chars().take(447).collect::<String>() + "..."
            } else {
                s.to_string()
            }
        });

    let title_js = format!(
        r#"(() => {{
  const value = {};
  const setValue = (el, v) => {{
    try {{ el.focus(); }} catch(e) {{}}
    const tag = (el.tagName||'').toUpperCase();
    const ce = (el.getAttribute('contenteditable')||'').toLowerCase();
    if (ce === 'true' || tag === 'DIV' || tag === 'SPAN') {{
      try {{ document.execCommand('selectAll', false, null); document.execCommand('insertText', false, v); }} catch(e) {{}}
    }} else {{
      try {{
        const proto = tag === 'TEXTAREA' ? window.HTMLTextAreaElement.prototype : window.HTMLInputElement.prototype;
        const ns = Object.getOwnPropertyDescriptor(proto, 'value');
        if (ns && ns.set) ns.set.call(el, v); else el.value = v;
      }} catch(e) {{ try {{ el.value = v; }} catch(e2) {{}} }}
    }}
    try {{ el.dispatchEvent(new Event('input', {{ bubbles: true }})); }} catch(e) {{}}
    try {{ el.dispatchEvent(new Event('change', {{ bubbles: true }})); }} catch(e) {{}}
    try {{ el.dispatchEvent(new Event('blur', {{ bubbles: true }})); }} catch(e) {{}}
  }};
  // Try known IDs first.
  const byId = document.getElementById('storyboard-selector-title');
  if (byId) {{ setValue(byId, value); return true; }}
  // Fallback: heuristic scoring.
  const els = Array.from(document.querySelectorAll('input,textarea,[contenteditable="true"]'));
  const score = (el) => {{
    if (el.getAttribute('type') === 'hidden') return -100;
    const name = (el.getAttribute('name')||'').toLowerCase();
    const ph = (el.getAttribute('placeholder')||'').toLowerCase();
    const aria = (el.getAttribute('aria-label')||'').toLowerCase();
    const id = (el.getAttribute('id')||'').toLowerCase();
    const tag = (el.tagName||'').toLowerCase();
    const ce = (el.getAttribute('contenteditable')||'').toLowerCase();
    const s = name + ' ' + ph + ' ' + aria + ' ' + id;
    let sc = 0;
    if (tag === 'input') sc += 1;
    if (tag === 'div' && ce === 'true') sc += 1;
    if (s.includes('title')) sc += 3;
    if (s.includes('pin title')) sc += 3;
    if (s.includes('tell everyone')) sc += 3;
    if (s.includes('your pin is about')) sc += 3;
    if (s.includes('add your title')) sc += 3;
    if (s.includes('your title')) sc += 2;
    return sc;
  }};
  let best = null; let bestScore = 0;
  for (const el of els) {{ const sc = score(el); if (sc > bestScore) {{ bestScore = sc; best = el; }} }}
  if (!best || bestScore < 2) return false;
  setValue(best, value);
  return true;
}} )()"#,
        serde_json::to_string(title).unwrap_or_else(|_| "\"\"".to_string())
    );

    let description_js = description.as_deref().map(|desc| {
        format!(
            r#"(() => {{
  const value = {};
  const setValue = (el, v) => {{
    try {{ el.focus(); }} catch(e) {{}}
    const tag = (el.tagName||'').toUpperCase();
    const ce = (el.getAttribute('contenteditable')||'').toLowerCase();
    if (ce === 'true' || tag === 'DIV' || tag === 'SPAN') {{
      try {{ document.execCommand('selectAll', false, null); document.execCommand('insertText', false, v); }} catch(e) {{}}
    }} else {{
      try {{
        const proto = tag === 'TEXTAREA' ? window.HTMLTextAreaElement.prototype : window.HTMLInputElement.prototype;
        const ns = Object.getOwnPropertyDescriptor(proto, 'value');
        if (ns && ns.set) ns.set.call(el, v); else el.value = v;
      }} catch(e) {{ try {{ el.value = v; }} catch(e2) {{}} }}
    }}
    try {{ el.dispatchEvent(new Event('input', {{ bubbles: true }})); }} catch(e) {{}}
    try {{ el.dispatchEvent(new Event('change', {{ bubbles: true }})); }} catch(e) {{}}
    try {{ el.dispatchEvent(new Event('blur', {{ bubbles: true }})); }} catch(e) {{}}
  }};
  // Try known aria-label first (Pinterest uses "Describe your Pin").
  const byAria = document.querySelector('[aria-label="Describe your Pin"]');
  if (byAria) {{ setValue(byAria, value); return true; }}
  // Fallback: heuristic scoring.
  const els = Array.from(document.querySelectorAll('input,textarea,[contenteditable="true"]'));
  const score = (el) => {{
    if (el.getAttribute('type') === 'hidden') return -100;
    const tag = (el.tagName||'').toLowerCase();
    const name = (el.getAttribute('name')||'').toLowerCase();
    const ph = (el.getAttribute('placeholder')||'').toLowerCase();
    const aria = (el.getAttribute('aria-label')||'').toLowerCase();
    const id = (el.getAttribute('id')||'').toLowerCase();
    const ce = (el.getAttribute('contenteditable')||'').toLowerCase();
    const s = name + ' ' + ph + ' ' + aria + ' ' + id;
    let sc = 0;
    if (tag === 'textarea') sc += 2;
    if (tag === 'div' && ce === 'true') sc += 1;
    if (s.includes('description')) sc += 3;
    if (s.includes('describe')) sc += 3;
    if (s.includes('tell everyone') || s.includes('add a description')) sc += 3;
    if (s.includes('details')) sc += 1;
    return sc;
  }};
  let best = null; let bestScore = 0;
  for (const el of els) {{ const sc = score(el); if (sc > bestScore) {{ bestScore = sc; best = el; }} }}
  if (!best || bestScore < 2) return false;
  setValue(best, value);
  return true;
}})()"#,
            serde_json::to_string(desc).unwrap_or_else(|_| "\"\"".to_string())
        )
    });

    let link_js = format!(
        r#"(() => {{
  const value = {};
  const setValue = (el, v) => {{
    try {{ el.focus(); }} catch(e) {{}}
    const tag = (el.tagName||'').toUpperCase();
    const ce = (el.getAttribute('contenteditable')||'').toLowerCase();
    if (ce === 'true' || tag === 'DIV' || tag === 'SPAN') {{
      try {{ document.execCommand('selectAll', false, null); document.execCommand('insertText', false, v); }} catch(e) {{}}
    }} else {{
      try {{
        const proto = tag === 'TEXTAREA' ? window.HTMLTextAreaElement.prototype : window.HTMLInputElement.prototype;
        const ns = Object.getOwnPropertyDescriptor(proto, 'value');
        if (ns && ns.set) ns.set.call(el, v); else el.value = v;
      }} catch(e) {{ try {{ el.value = v; }} catch(e2) {{}} }}
    }}
    try {{ el.dispatchEvent(new Event('input', {{ bubbles: true }})); }} catch(e) {{}}
    try {{ el.dispatchEvent(new Event('change', {{ bubbles: true }})); }} catch(e) {{}}
    try {{ el.dispatchEvent(new Event('blur', {{ bubbles: true }})); }} catch(e) {{}}
  }};
  // Try known IDs first.
  const byId = document.getElementById('WebsiteField');
  if (byId) {{ setValue(byId, value); return true; }}
  // Fallback: heuristic scoring.
  const els = Array.from(document.querySelectorAll('input,textarea,[contenteditable="true"]'));
  const score = (el) => {{
    const name = (el.getAttribute('name')||'').toLowerCase();
    const ph = (el.getAttribute('placeholder')||'').toLowerCase();
    const aria = (el.getAttribute('aria-label')||'').toLowerCase();
    const id = (el.getAttribute('id')||'').toLowerCase();
    const tag = (el.tagName||'').toLowerCase();
    const ce = (el.getAttribute('contenteditable')||'').toLowerCase();
    const s = name + ' ' + ph + ' ' + aria + ' ' + id;
    if (s.includes('destination')) return 3;
    if (s.includes('website')) return 3;
    if (s.includes('add a link')) return 3;
    if (s.includes('link')) return 2;
    if (s.includes('url')) return 2;
    if (s.includes('source')) return 2;
    if (tag === 'input' && (el.getAttribute('type')||'').toLowerCase() === 'url') return 2;
    if (tag === 'div' && ce === 'true') return 1;
    return 0;
  }};
  let best = null; let bestScore = 0;
  for (const el of els) {{ const sc = score(el); if (sc > bestScore) {{ bestScore = sc; best = el; }} }}
  if (!best || bestScore < 2) return false;
  setValue(best, value);
  return true;
}} )()"#,
        serde_json::to_string(link).unwrap_or_else(|_| "\"\"".to_string())
    );

    // The form fields often appear only after the image has been processed.
    let mut title_ok = false;
    let mut desc_ok = false;
    let mut link_ok = false;
    for _ in 0..20 {
        if !title_ok {
            title_ok = page
                .evaluate(title_js.as_str())
                .await?
                .into_value::<bool>()
                .unwrap_or(false);
        }

        if !desc_ok {
            if let Some(description_js) = &description_js {
                desc_ok = page
                    .evaluate(description_js.as_str())
                    .await?
                    .into_value::<bool>()
                    .unwrap_or(false);
            } else {
                desc_ok = true;
            }
        }

        if !link_ok {
            link_ok = page
                .evaluate(link_js.as_str())
                .await?
                .into_value::<bool>()
                .unwrap_or(false);
        }

        if title_ok && desc_ok && link_ok {
            break;
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    if !title_ok {
        warn!("could not find pinterest title field; continuing");
    }
    if description_js.is_some() && !desc_ok {
        warn!("could not find pinterest description field; continuing");
    }
    if !link_ok {
        warn!("could not find pinterest link field; continuing");
    }

    if !title_ok || !link_ok {
        let debug_js = r#"(() => {
  const els = Array.from(document.querySelectorAll('input,textarea,[contenteditable="true"]')).slice(0, 250);
  const fields = els.map(el => ({
    tag: (el.tagName||'').toLowerCase(),
    type: el.getAttribute('type') || '',
    name: el.getAttribute('name') || '',
    id: el.getAttribute('id') || '',
    aria: el.getAttribute('aria-label') || '',
    placeholder: el.getAttribute('placeholder') || '',
    contenteditable: el.getAttribute('contenteditable') || '',
    class: (el.className||'').toString().slice(0,120),
  }));
  const text = (document.documentElement && document.documentElement.innerText) ? document.documentElement.innerText : '';
  return {
    href: location.href,
    title: document.title,
    readyState: document.readyState,
    fieldCount: els.length,
    fields,
    innerTextSnippet: text.trim().slice(0, 2000),
  };
})()"#;

        if let Ok(v) = page
            .evaluate(debug_js)
            .await?
            .into_value::<serde_json::Value>()
        {
            let debug_path = Path::new("../data/pinterest_fields_debug.json");
            if let Ok(s) = serde_json::to_string_pretty(&v) {
                if let Err(err) = tokio::fs::write(debug_path, s).await {
                    warn!(error = %err, path = %debug_path.display(), "failed to write pinterest fields debug json");
                } else {
                    warn!(path = %debug_path.display(), "wrote pinterest fields debug json");
                }
            }
        }
    }

    Ok(())
}
