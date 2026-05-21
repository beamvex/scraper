use anyhow::{Context, Result, bail};
use chromiumoxide::browser::Browser;
use chromiumoxide::cdp::browser_protocol::dom::SetFileInputFilesParams;
use chromiumoxide::page::Page;
use std::env;
use std::path::Path;
use std::time::Duration;
use tracing::{info, warn};

pub async fn maybe_post_pin_to_board(
    browser: &Browser,
    title: &str,
    description: Option<&str>,
    article_url: Option<&str>,
    image_path: Option<&Path>,
) -> Result<()> {
    info!("maybe posting pin to board");
    let board_url = env::var("PINTEREST_BOARD_URL")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "https://uk.pinterest.com/forster2474/random-thoughts/".to_string());

    let Some(article_url) = article_url.filter(|u| !u.trim().is_empty()) else {
        info!("no article url provided, skipping pinterest post");
        return Ok(());
    };

    let Some(image_path) = image_path.filter(|p| p.exists()) else {
        info!("no image path provided or image does not exist, skipping pinterest post");
        return Ok(());
    };

    // Optional gating.
    let enabled = env::var("PINTEREST_ENABLED")
        .ok()
        .unwrap_or_else(|| "0".to_string());
    if enabled != "1" {
        info!("pinterest is disabled, skipping post");
        return Ok(());
    }

    info!("posting pin to board");
    post_pin_to_board(
        browser,
        &board_url,
        title,
        description,
        article_url,
        image_path,
    )
    .await
}

async fn post_pin_to_board(
    browser: &Browser,
    board_url: &str,
    title: &str,
    description: Option<&str>,
    link: &str,
    image_path: &Path,
) -> Result<()> {
    // Pinterest UI is volatile; we try a best-effort flow via the Pin Builder.
    // Assumption: you are already logged-in in the Chrome profile used by the running container.

    info!(%board_url, "opening pinterest board");
    let _board_page = browser
        .new_page(board_url)
        .await
        .context("failed to open pinterest board")?;

    tokio::time::sleep(Duration::from_millis(1200)).await;

    let pin_builder_url = "https://www.pinterest.com/pin-builder/";
    info!(%pin_builder_url, "opening pinterest pin builder");
    let page = browser
        .new_page(pin_builder_url)
        .await
        .context("failed to open pinterest pin builder")?;

    tokio::time::sleep(Duration::from_millis(3000)).await;

    wait_for_pin_creation_ui(&page).await?;

    if is_login_interstitial(&page).await.unwrap_or(false) {
        bail!(
            "pinterest appears to require login in the current browser session (redirected to login)"
        );
    }

    upload_image(&page, image_path).await?;
    fill_text_fields(&page, title, description, link).await?;
    // choose_board(&page, board_url).await?;
    publish_pin(&page).await?;

    Ok(())
}

async fn wait_for_pin_creation_ui(page: &Page) -> Result<()> {
    // Pin creation tool is a heavy SPA. If it doesn't render, selectors will all fail.
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

        // Best signal: the image upload control exists.
        if counts.2 >= 1 {
            return Ok(());
        }

        // Otherwise: the SPA is likely ready if we see a reasonable number of interactive elements.
        let formish = counts.0.saturating_add(counts.1);
        if counts.3 >= 6 || (counts.3 >= 4 && formish >= 3) {
            return Ok(());
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    let url = page.url().await.ok().flatten().unwrap_or_default();

    if let Ok(html) = page.content().await {
        let debug_path = Path::new("/data/pinterest_upload_debug.html");
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
        let debug_path = Path::new("/data/pinterest_upload_debug.json");
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

async fn is_login_interstitial(page: &Page) -> Result<bool> {
    let url = page.url().await.ok().flatten().unwrap_or_default();
    if url.contains("/login") {
        return Ok(true);
    }

    let html: String = page
        .evaluate("document.documentElement.innerText")
        .await?
        .into_value::<String>()
        .unwrap_or_default();

    Ok(html.to_lowercase().contains("log in") && html.to_lowercase().contains("password"))
}

async fn upload_image(page: &Page, image_path: &Path) -> Result<()> {
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
        let debug_path = Path::new("/data/pinterest_upload_debug.html");
        if let Err(err) = tokio::fs::write(debug_path, html).await {
            warn!(error = %err, path = %debug_path.display(), "failed to write pinterest debug html");
        } else {
            warn!(path = %debug_path.display(), "wrote pinterest debug html");
        }
    }

    // Smaller debug: list iframes and some element metadata (helps when the HTML is mostly scripts).
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
        let debug_path = Path::new("/data/pinterest_upload_debug.json");
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

async fn fill_text_fields(
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
  const norm = (s) => (s||'').toLowerCase();
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
            let debug_path = Path::new("/data/pinterest_fields_debug.json");
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

async fn choose_board(page: &Page, board_url: &str) -> Result<()> {
    let board_name = board_url
        .trim_end_matches('/')
        .split('/')
        .last()
        .unwrap_or("random-thoughts")
        .replace('-', " ");

    // Click the board picker button — retry to handle late rendering.
    let open_js = r#"(() => {
  const buttons = Array.from(document.querySelectorAll('button,div[role=button],a[role=button],[role=combobox]'));
  const norm = (s) => (s||'').toLowerCase();
  const open = buttons.find(b => norm(b.innerText).includes('choose a board'))
    || buttons.find(b => norm(b.getAttribute('placeholder')||'').includes('board'))
    || buttons.find(b => norm(b.getAttribute('aria-label')||'').includes('board'))
    || buttons.find(b => norm(b.innerText) === 'board')
    || buttons.find(b => norm(b.innerText).includes('board'));
  if (!open) return false;
  open.click();
  return true;
})()"#;

    let mut opened = false;
    for _ in 0..12 {
        opened = page
            .evaluate(open_js)
            .await?
            .into_value::<bool>()
            .unwrap_or(false);
        if opened {
            break;
        }
        tokio::time::sleep(Duration::from_millis(600)).await;
    }

    if !opened {
        warn!("could not find pinterest board picker; continuing");
        let _ =
            write_debug_snapshot(page, Path::new("/data/pinterest_choose_board_debug.json")).await;
        return Ok(());
    }

    tokio::time::sleep(Duration::from_millis(1200)).await;

    // Type into search box if one appeared (React-safe native setter).
    let search_js = format!(
        r#"(() => {{
  const needle = {};
  const inputs = Array.from(document.querySelectorAll('input'));
  const box = inputs.find(i => {{
    const ph = (i.getAttribute('placeholder')||'').toLowerCase();
    const aria = (i.getAttribute('aria-label')||'').toLowerCase();
    return ph.includes('search') || aria.includes('search') || ph.includes('board') || aria.includes('board');
  }});
  if (!box) return false;
  box.focus();
  try {{
    const ns = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value');
    if (ns && ns.set) ns.set.call(box, needle); else box.value = needle;
  }} catch(e) {{ box.value = needle; }}
  box.dispatchEvent(new Event('input', {{ bubbles: true }}));
  box.dispatchEvent(new Event('change', {{ bubbles: true }}));
  return true;
}})()
"#,
        serde_json::to_string(&board_name).unwrap_or_else(|_| "\"\"".to_string())
    );
    let _ = page.evaluate(search_js.as_str()).await;
    tokio::time::sleep(Duration::from_millis(800)).await;

    // Pick the board item — prefer role=option/menuitem/listitem, then li, then a.
    let pick_js = format!(
        r#"(() => {{
  const needle = {};
  const norm = (s) => (s||'').toLowerCase().trim();
  const n = norm(needle);
  const tryPick = (sel) => {{
    const items = Array.from(document.querySelectorAll(sel));
    return items.find(el => {{
      const t = norm(el.innerText);
      return t === n || t.startsWith(n) || t.includes(n);
    }}) || null;
  }};
  const target = tryPick('[role=option],[role=menuitem],[role=listitem]')
    || tryPick('li')
    || tryPick('a');
  if (!target) return false;
  target.click();
  return true;
}})()
"#,
        serde_json::to_string(&board_name).unwrap_or_else(|_| "\"\"".to_string())
    );

    let mut picked = false;
    for _ in 0..6 {
        picked = page
            .evaluate(pick_js.as_str())
            .await?
            .into_value::<bool>()
            .unwrap_or(false);
        if picked {
            break;
        }
        tokio::time::sleep(Duration::from_millis(600)).await;
    }

    if !picked {
        warn!(board = %board_name, "failed to pick board by name; continuing");
        let _ =
            write_debug_snapshot(page, Path::new("/data/pinterest_choose_board_debug.json")).await;
    }

    tokio::time::sleep(Duration::from_millis(800)).await;
    Ok(())
}

async fn write_debug_snapshot(page: &Page, path: &Path) -> Result<()> {
    let debug_js = r#"(() => {
  const text = (document.documentElement && document.documentElement.innerText) ? document.documentElement.innerText : '';
  const sample = (sel) => Array.from(document.querySelectorAll(sel)).slice(0, 60).map(el => ({
    tag: (el.tagName||'').toLowerCase(),
    type: el.getAttribute('type') || '',
    role: el.getAttribute('role') || '',
    aria: el.getAttribute('aria-label') || '',
    name: el.getAttribute('name') || '',
    id: el.getAttribute('id') || '',
    placeholder: el.getAttribute('placeholder') || '',
    contenteditable: el.getAttribute('contenteditable') || '',
    text: (el.innerText || '').trim().slice(0, 140),
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

    let v = page
        .evaluate(debug_js)
        .await?
        .into_value::<serde_json::Value>()
        .unwrap_or(serde_json::json!({"error": "failed to evaluate debug snapshot"}));

    let s = serde_json::to_string_pretty(&v).unwrap_or_else(|_| "{}".to_string());
    tokio::fs::write(path, s).await?;
    Ok(())
}

async fn publish_pin(page: &Page) -> Result<()> {
    let js = r#"(() => {
  const btns = Array.from(document.querySelectorAll('button,div[role=button],a[role=button]'));
  const norm = (s) => (s||'').toLowerCase().trim();
  const good = (b) => {
    const t = norm(b.innerText);
    const a = norm(b.getAttribute('aria-label')||'');
    return t === 'publish' || t === 'save' || a === 'publish' || a === 'save'
      || t.includes('publish') || t.includes('save pin') || a.includes('publish') || a.includes('save');
  };
  const target = btns.find(good) || btns.find(b => b.getAttribute('type') === 'submit');
  if (!target) return false;
  target.click();
  return true;
})()"#;

    let mut clicked = false;
    for _ in 0..14 {
        clicked = page
            .evaluate(js)
            .await?
            .into_value::<bool>()
            .unwrap_or(false);
        if clicked {
            break;
        }
        tokio::time::sleep(Duration::from_millis(600)).await;
    }

    if !clicked {
        let _ = write_debug_snapshot(page, Path::new("/data/pinterest_publish_debug.json")).await;
        bail!("could not find pinterest publish/save button")
    }

    tokio::time::sleep(Duration::from_millis(3500)).await;
    let _ = write_debug_snapshot(page, Path::new("/data/pinterest_publish_after_debug.json")).await;
    info!("attempted to publish pinterest pin");
    Ok(())
}

mod tests {
    use super::*;

    #[tokio::test]
    async fn test_maybe_post_pin_to_board() {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .init();

        info!("running test!!");
        let (browser, mut handler) = Browser::connect("http://127.0.0.1:9222").await.unwrap();
        tokio::spawn(async move {
            use futures::StreamExt;
            while let Some(e) = handler.next().await {
                let _ = e;
            }
        });

        let result = maybe_post_pin_to_board(
            &browser,
            "title",
            Some("description"),
            Some("https://example.com"),
            Some(Path::new("/home/robert/main-213.jpg")),
        )
        .await;
        assert!(result.is_ok());
    }
}
