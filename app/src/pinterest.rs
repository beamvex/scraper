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
    article_url: Option<&str>,
    image_path: Option<&Path>,
) -> Result<()> {
    let board_url = env::var("PINTEREST_BOARD_URL")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "https://uk.pinterest.com/forster2474/random-thoughts/".to_string());

    let Some(article_url) = article_url.filter(|u| !u.trim().is_empty()) else {
        return Ok(());
    };

    let Some(image_path) = image_path.filter(|p| p.exists()) else {
        return Ok(());
    };

    // Optional gating.
    let enabled = env::var("PINTEREST_ENABLED")
        .ok()
        .unwrap_or_else(|| "0".to_string());
    if enabled != "1" {
        return Ok(());
    }

    post_pin_to_board(browser, &board_url, title, article_url, image_path).await
}

async fn post_pin_to_board(
    browser: &Browser,
    board_url: &str,
    title: &str,
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

    if is_login_interstitial(&page).await.unwrap_or(false) {
        bail!(
            "pinterest appears to require login in the current browser session (redirected to login)"
        );
    }

    upload_image(&page, image_path).await?;
    fill_text_fields(&page, title, link).await?;
    choose_board(&page, board_url).await?;
    publish_pin(&page).await?;

    Ok(())
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

    for sel in selectors {
        if let Ok(el) = page.find_element(sel).await {
            let files = vec![image_path.display().to_string()];
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

    bail!("could not find pinterest image upload input")
}

async fn fill_text_fields(page: &Page, title: &str, link: &str) -> Result<()> {
    // Pinterest frequently changes selectors; do this via JS and dispatch input events.
    let title_js = format!(
        r#"(() => {{
  const value = {};
  const els = Array.from(document.querySelectorAll('input,textarea'));
  const score = (el) => {{
    const name = (el.getAttribute('name')||'').toLowerCase();
    const ph = (el.getAttribute('placeholder')||'').toLowerCase();
    const aria = (el.getAttribute('aria-label')||'').toLowerCase();
    const id = (el.getAttribute('id')||'').toLowerCase();
    const s = name + ' ' + ph + ' ' + aria + ' ' + id;
    if (s.includes('title')) return 3;
    if (s.includes('pin title')) return 3;
    return 0;
  }};
  let best = null;
  let bestScore = 0;
  for (const el of els) {{
    const sc = score(el);
    if (sc > bestScore) {{ bestScore = sc; best = el; }}
  }}
  if (!best) return false;
  best.focus();
  best.value = value;
  best.dispatchEvent(new Event('input', {{ bubbles: true }}));
  best.dispatchEvent(new Event('change', {{ bubbles: true }}));
  return true;
}})()"#,
        serde_json::to_string(title).unwrap_or_else(|_| "\"\"".to_string())
    );

    let link_js = format!(
        r#"(() => {{
  const value = {};
  const els = Array.from(document.querySelectorAll('input,textarea'));
  const score = (el) => {{
    const name = (el.getAttribute('name')||'').toLowerCase();
    const ph = (el.getAttribute('placeholder')||'').toLowerCase();
    const aria = (el.getAttribute('aria-label')||'').toLowerCase();
    const id = (el.getAttribute('id')||'').toLowerCase();
    const s = name + ' ' + ph + ' ' + aria + ' ' + id;
    if (s.includes('destination')) return 3;
    if (s.includes('website')) return 3;
    if (s.includes('link')) return 2;
    if (s.includes('url')) return 2;
    return 0;
  }};
  let best = null;
  let bestScore = 0;
  for (const el of els) {{
    const sc = score(el);
    if (sc > bestScore) {{ bestScore = sc; best = el; }}
  }}
  if (!best) return false;
  best.focus();
  best.value = value;
  best.dispatchEvent(new Event('input', {{ bubbles: true }}));
  best.dispatchEvent(new Event('change', {{ bubbles: true }}));
  return true;
}})()"#,
        serde_json::to_string(link).unwrap_or_else(|_| "\"\"".to_string())
    );

    let title_ok = page
        .evaluate(title_js)
        .await?
        .into_value::<bool>()
        .unwrap_or(false);
    if !title_ok {
        warn!("could not find pinterest title field; continuing");
    }

    let link_ok = page
        .evaluate(link_js)
        .await?
        .into_value::<bool>()
        .unwrap_or(false);
    if !link_ok {
        warn!("could not find pinterest link field; continuing");
    }

    tokio::time::sleep(Duration::from_millis(1200)).await;
    Ok(())
}

async fn choose_board(page: &Page, board_url: &str) -> Result<()> {
    // Best-effort: try to click a board selector and type the board name extracted from the URL.
    let board_name = board_url
        .trim_end_matches('/')
        .split('/')
        .last()
        .unwrap_or("random-thoughts")
        .replace('-', " ");

    let board_js = format!(
        r#"(() => {{
  const needle = {};
  const buttons = Array.from(document.querySelectorAll('button,div[role=button]'));
  const open = buttons.find(b => (b.innerText||'').toLowerCase().includes('board'))
    || buttons.find(b => (b.getAttribute('aria-label')||'').toLowerCase().includes('board'));
  if (!open) return false;
  open.click();
  return true;
}})()"#,
        serde_json::to_string(&board_name).unwrap_or_else(|_| "\"\"".to_string())
    );

    let opened = page
        .evaluate(board_js)
        .await?
        .into_value::<bool>()
        .unwrap_or(false);

    if !opened {
        warn!("could not find pinterest board picker; continuing");
        return Ok(());
    }

    tokio::time::sleep(Duration::from_millis(900)).await;

    let pick_js = format!(
        r#"(() => {{
  const needle = {};
  const norm = (s) => (s||'').toLowerCase();
  const items = Array.from(document.querySelectorAll('[role=option],a,div'));
  const target = items.find(el => norm(el.innerText).includes(norm(needle)));
  if (!target) return false;
  target.click();
  return true;
}})()"#,
        serde_json::to_string(&board_name).unwrap_or_else(|_| "\"\"".to_string())
    );

    let picked = page
        .evaluate(pick_js)
        .await?
        .into_value::<bool>()
        .unwrap_or(false);

    if !picked {
        warn!(board = %board_name, "failed to pick board by name; continuing");
    }

    tokio::time::sleep(Duration::from_millis(700)).await;
    Ok(())
}

async fn publish_pin(page: &Page) -> Result<()> {
    let js = r#"(() => {
  const btns = Array.from(document.querySelectorAll('button'));
  const norm = (s) => (s||'').toLowerCase().trim();
  const good = (b) => {
    const t = norm(b.innerText);
    const a = norm(b.getAttribute('aria-label'));
    return t === 'publish' || t === 'save' || a === 'publish' || a === 'save' || t.includes('publish') || t.includes('save');
  };
  const target = btns.find(good) || btns.find(b => b.getAttribute('type') === 'submit');
  if (!target) return false;
  target.click();
  return true;
})()"#;

    let clicked = page
        .evaluate(js)
        .await?
        .into_value::<bool>()
        .unwrap_or(false);

    if !clicked {
        bail!("could not find pinterest publish/save button")
    }

    tokio::time::sleep(Duration::from_millis(3500)).await;
    info!("attempted to publish pinterest pin");
    Ok(())
}

async fn find_first(
    page: &Page,
    selectors: &[&str],
) -> Result<Option<chromiumoxide::element::Element>> {
    for sel in selectors {
        if let Ok(el) = page.find_element(*sel).await {
            return Ok(Some(el));
        }
    }
    Ok(None)
}
