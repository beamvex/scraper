use anyhow::{Context, Result, bail};
use chromiumoxide::browser::Browser;
use chromiumoxide::cdp::browser_protocol::input::InsertTextParams;
use chromiumoxide::cdp::browser_protocol::input::{DispatchKeyEventParams, DispatchKeyEventType};
use chromiumoxide::cdp::browser_protocol::input::{
    DispatchMouseEventParams, DispatchMouseEventType, MouseButton,
};
use chromiumoxide::cdp::browser_protocol::input::{
    DispatchMouseEventParams, DispatchMouseEventType, MouseButton,
};
use chromiumoxide::page::Page;
use rand::Rng;
use std::path::Path;
use tracing::{info, warn};

pub async fn create_medium_draft_from_review_html(
    browser: &Browser,
    review_html_path: &Path,
) -> Result<()> {
    let review_url = format!("file://{}", review_html_path.display());
    info!(%review_url, "opening review html for copy source");

    let review_page = browser
        .new_page(review_url)
        .await
        .context("failed to open review html in new tab")?;

    tokio::time::sleep(std::time::Duration::from_millis(800)).await;

    let (title, body_text) = extract_title_and_body_text(&review_page).await?;
    info!(
        title_len = title.len(),
        body_len = body_text.len(),
        "extracted review title/body"
    );

    info!("opening medium new story");
    let medium_page = browser
        .new_page("https://medium.com/new-story")
        .await
        .context("failed to open medium new story page")?;

    let _ = medium_page.enable_stealth_mode().await;
    tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
    ensure_logged_in(&medium_page).await?;

    focus_medium_title(&medium_page).await?;
    if let Ok((x, y)) = get_medium_title_point(&medium_page).await {
        mouse_move_and_click(&medium_page, x, y).await?;
    }
    if let Ok((x, y)) = get_medium_title_point(&medium_page).await {
        mouse_move_and_click(&medium_page, x, y).await?;
    }
    tokio::time::sleep(std::time::Duration::from_millis(180)).await;
    type_text_like_user(&medium_page, &title).await?;

    press_enter(&medium_page).await?;
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;

    if let Ok((x, y)) = get_medium_body_point(&medium_page).await {
        mouse_move_and_click(&medium_page, x, y).await?;
    }

    if let Ok((x, y)) = get_medium_body_point(&medium_page).await {
        mouse_move_and_click(&medium_page, x, y).await?;
    }

    copy_text_via_clipboard(&review_page, &body_text).await?;
    paste_from_clipboard(&medium_page).await?;

    open_publish_flow(&medium_page).await;

    let _ = review_page.close().await;
    Ok(())
}

async fn get_medium_title_point(page: &Page) -> Result<(f64, f64)> {
    let v: serde_json::Value = page
        .evaluate(
            r#"(() => {
  const editables = Array.from(document.querySelectorAll('[contenteditable="true"]'));
  const titleEl = editables.find((el) => el.tagName === 'H1') || editables[0];
  if (!titleEl) return { ok: false };
  const r = titleEl.getBoundingClientRect();
  return { ok: true, x: r.left + r.width * 0.5, y: r.top + r.height * 0.5 };
})()"#,
        )
        .await?
        .into_value()?;

    if v.get("ok").and_then(|b| b.as_bool()) != Some(true) {
        bail!("could not locate Medium title rect");
    }
    let x = v.get("x").and_then(|n| n.as_f64()).context("missing x")?;
    let y = v.get("y").and_then(|n| n.as_f64()).context("missing y")?;
    Ok((x, y))
}

async fn get_medium_body_point(page: &Page) -> Result<(f64, f64)> {
    let v: serde_json::Value = page
        .evaluate(
            r#"(() => {
  const editables = Array.from(document.querySelectorAll('[contenteditable="true"]'));
  // Find a non-H1 editable for body.
  const bodyEl = editables.find((el) => el.tagName !== 'H1') || editables[editables.length - 1];
  if (!bodyEl) return { ok: false };
  const r = bodyEl.getBoundingClientRect();
  return { ok: true, x: r.left + Math.min(r.width * 0.25, 80), y: r.top + Math.min(r.height * 0.6, 120) };
})()"#,
        )
        .await?
        .into_value()?;

    if v.get("ok").and_then(|b| b.as_bool()) != Some(true) {
        bail!("could not locate Medium body rect");
    }
    let x = v.get("x").and_then(|n| n.as_f64()).context("missing x")?;
    let y = v.get("y").and_then(|n| n.as_f64()).context("missing y")?;
    Ok((x, y))
}

async fn mouse_move_and_click(page: &Page, x: f64, y: f64) -> Result<()> {
    let mut rng = rand::rng();

    // Start somewhere near the target with some jitter.
    let mut cx = x + rng.random_range(-120.0..=120.0);
    let mut cy = y + rng.random_range(-80.0..=80.0);

    let steps = rng.random_range(6..=14);
    for i in 0..steps {
        let t = (i + 1) as f64 / steps as f64;
        let nx = cx + (x - cx) * t + rng.random_range(-2.5..=2.5);
        let ny = cy + (y - cy) * t + rng.random_range(-2.0..=2.0);
        let mv: DispatchMouseEventParams = DispatchMouseEventParams::builder()
            .r#type(DispatchMouseEventType::MouseMoved)
            .x(nx)
            .y(ny)
            .build()
            .map_err(|e| anyhow::anyhow!(e))?;
        page.execute(mv).await.context("failed mouse move")?;
        tokio::time::sleep(std::time::Duration::from_millis(rng.random_range(8..=24))).await;
        cx = nx;
        cy = ny;
    }

    tokio::time::sleep(std::time::Duration::from_millis(rng.random_range(40..=120))).await;

    let down: DispatchMouseEventParams = DispatchMouseEventParams::builder()
        .r#type(DispatchMouseEventType::MousePressed)
        .x(x)
        .y(y)
        .button(MouseButton::Left)
        .click_count(1)
        .build()
        .map_err(|e| anyhow::anyhow!(e))?;
    page.execute(down).await.context("failed mouse down")?;

    tokio::time::sleep(std::time::Duration::from_millis(rng.random_range(30..=90))).await;

    let up: DispatchMouseEventParams = DispatchMouseEventParams::builder()
        .r#type(DispatchMouseEventType::MouseReleased)
        .x(x)
        .y(y)
        .button(MouseButton::Left)
        .click_count(1)
        .build()
        .map_err(|e| anyhow::anyhow!(e))?;
    page.execute(up).await.context("failed mouse up")?;

    Ok(())
}

async fn get_medium_title_point(page: &Page) -> Result<(f64, f64)> {
    let v: serde_json::Value = page
        .evaluate(
            r#"(() => {
  const editables = Array.from(document.querySelectorAll('[contenteditable="true"]'));
  const titleEl = editables.find((el) => el.tagName === 'H1') || editables[0];
  if (!titleEl) return { ok: false };
  const r = titleEl.getBoundingClientRect();
  return { ok: true, x: r.left + r.width * 0.5, y: r.top + r.height * 0.5 };
})()"#,
        )
        .await?
        .into_value()?;

    if v.get("ok").and_then(|b| b.as_bool()) != Some(true) {
        bail!("could not locate Medium title rect");
    }
    let x = v.get("x").and_then(|n| n.as_f64()).context("missing x")?;
    let y = v.get("y").and_then(|n| n.as_f64()).context("missing y")?;
    Ok((x, y))
}

async fn get_medium_body_point(page: &Page) -> Result<(f64, f64)> {
    let v: serde_json::Value = page
        .evaluate(
            r#"(() => {
  const editables = Array.from(document.querySelectorAll('[contenteditable="true"]'));
  // Find a non-H1 editable for body.
  const bodyEl = editables.find((el) => el.tagName !== 'H1') || editables[editables.length - 1];
  if (!bodyEl) return { ok: false };
  const r = bodyEl.getBoundingClientRect();
  return { ok: true, x: r.left + Math.min(r.width * 0.25, 80), y: r.top + Math.min(r.height * 0.6, 120) };
})()"#,
        )
        .await?
        .into_value()?;

    if v.get("ok").and_then(|b| b.as_bool()) != Some(true) {
        bail!("could not locate Medium body rect");
    }
    let x = v.get("x").and_then(|n| n.as_f64()).context("missing x")?;
    let y = v.get("y").and_then(|n| n.as_f64()).context("missing y")?;
    Ok((x, y))
}

async fn mouse_move_and_click(page: &Page, x: f64, y: f64) -> Result<()> {
    let mut rng = rand::rng();

    // Start somewhere near the target with some jitter.
    let mut cx = x + rng.random_range(-120.0..=120.0);
    let mut cy = y + rng.random_range(-80.0..=80.0);

    let steps = rng.random_range(6..=14);
    for i in 0..steps {
        let t = (i + 1) as f64 / steps as f64;
        let nx = cx + (x - cx) * t + rng.random_range(-2.5..=2.5);
        let ny = cy + (y - cy) * t + rng.random_range(-2.0..=2.0);
        let mv: DispatchMouseEventParams = DispatchMouseEventParams::builder()
            .r#type(DispatchMouseEventType::MouseMoved)
            .x(nx)
            .y(ny)
            .build()
            .map_err(|e| anyhow::anyhow!(e))?;
        page.execute(mv).await.context("failed mouse move")?;
        tokio::time::sleep(std::time::Duration::from_millis(rng.random_range(8..=24))).await;
        cx = nx;
        cy = ny;
    }

    tokio::time::sleep(std::time::Duration::from_millis(rng.random_range(40..=120))).await;

    let down: DispatchMouseEventParams = DispatchMouseEventParams::builder()
        .r#type(DispatchMouseEventType::MousePressed)
        .x(x)
        .y(y)
        .button(MouseButton::Left)
        .click_count(1)
        .build()
        .map_err(|e| anyhow::anyhow!(e))?;
    page.execute(down).await.context("failed mouse down")?;

    tokio::time::sleep(std::time::Duration::from_millis(rng.random_range(30..=90))).await;

    let up: DispatchMouseEventParams = DispatchMouseEventParams::builder()
        .r#type(DispatchMouseEventType::MouseReleased)
        .x(x)
        .y(y)
        .button(MouseButton::Left)
        .click_count(1)
        .build()
        .map_err(|e| anyhow::anyhow!(e))?;
    page.execute(up).await.context("failed mouse up")?;

    Ok(())
}

pub async fn create_medium_draft(browser: &Browser, title: &str, body_text: &str) -> Result<()> {
    info!("opening medium new story");
    let page = browser
        .new_page("https://medium.com/new-story")
        .await
        .context("failed to open medium new story page")?;

    let _ = page.enable_stealth_mode().await;

    tokio::time::sleep(std::time::Duration::from_millis(2500)).await;

    ensure_logged_in(&page).await?;

    let payload_title = js_escape_for_template_literal(title);

    let res: serde_json::Value = page
        .evaluate(format!(
            r#"(() => {{
  const title = `{payload_title}`;

  const editables = Array.from(document.querySelectorAll('[contenteditable="true"]'));
  if (!editables.length) {{
    return {{ ok: false, error: 'no contenteditable elements found (are you blocked by a login wall?)' }};
  }}

  const titleEl = editables.find((el) => el.tagName === 'H1') || editables[0];
  titleEl.focus();
  document.execCommand('selectAll', false, null);
  document.execCommand('delete', false, null);

  return {{ ok: true, editables: editables.length, activeTag: (document.activeElement && document.activeElement.tagName) || null, readyForTyping: true }};
}})()"#
        ))
        .await?
        .into_value()?;

    if res.get("ok").and_then(|v| v.as_bool()) != Some(true) {
        bail!("failed to populate Medium editor: {}", res);
    }

    info!(details = %res, "populated Medium editor");

    // Simulate typing: title, Enter, then body.
    type_text_like_user(&page, title).await?;
    press_enter(&page).await?;
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    type_text_like_user(&page, body_text).await?;

    open_publish_flow(&page).await;

    Ok(())
}

async fn extract_title_and_body_text(review_page: &Page) -> Result<(String, String)> {
    let v: serde_json::Value = review_page
        .evaluate(
            r#"(() => {
  const title = (document.querySelector('h1') && document.querySelector('h1').innerText) || document.title || '';
  const article = document.querySelector('article');
  const body = (article ? article.innerText : document.body.innerText) || '';
  return { title, body };
})()"#,
        )
        .await?
        .into_value()?;

    let title = v
        .get("title")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let body = v
        .get("body")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .trim()
        .to_string();

    Ok((title, body))
}

async fn focus_medium_title(medium_page: &Page) -> Result<()> {
    let res: serde_json::Value = medium_page
        .evaluate(
            r#"(() => {
  const editables = Array.from(document.querySelectorAll('[contenteditable="true"]'));
  if (!editables.length) return { ok: false, error: 'no contenteditable elements found' };
  const titleEl = editables.find((el) => el.tagName === 'H1') || editables[0];
  titleEl.focus();
  document.execCommand('selectAll', false, null);
  document.execCommand('delete', false, null);
  return { ok: true, tag: titleEl.tagName };
})()"#,
        )
        .await?
        .into_value()?;

    if res.get("ok").and_then(|v| v.as_bool()) != Some(true) {
        bail!("failed to focus Medium title: {}", res);
    }
    Ok(())
}

async fn copy_text_via_clipboard(source_page: &Page, text: &str) -> Result<()> {
    let payload = js_escape_for_template_literal(text);
    let res: serde_json::Value = source_page
        .evaluate(format!(
            r#"(() => {{
  const text = `{payload}`;
  const ta = document.createElement('textarea');
  ta.value = text;
  ta.style.position = 'fixed';
  ta.style.left = '-9999px';
  ta.style.top = '0';
  document.body.appendChild(ta);
  ta.focus();
  ta.select();
  const ok = document.execCommand('copy');
  document.body.removeChild(ta);
  return {{ ok }};
}})()"#
        ))
        .await?
        .into_value()?;

    if res.get("ok").and_then(|v| v.as_bool()) != Some(true) {
        bail!("copy failed (clipboard may be blocked): {}", res);
    }
    Ok(())
}

async fn paste_from_clipboard(target_page: &Page) -> Result<()> {
    // Try to look like a real user: Ctrl+V.
    send_ctrl_combo(target_page, "v", 86).await
}

async fn send_ctrl_combo(page: &Page, key: &str, vk: i64) -> Result<()> {
    // Ctrl keydown
    let ctrl_down: DispatchKeyEventParams = DispatchKeyEventParams::builder()
        .r#type(DispatchKeyEventType::KeyDown)
        .key("Control")
        .code("ControlLeft")
        .windows_virtual_key_code(17)
        .native_virtual_key_code(17)
        .build()
        .map_err(|e| anyhow::anyhow!(e))?;
    page.execute(ctrl_down)
        .await
        .context("failed ctrl keyDown")?;

    // keydown for letter with Ctrl modifier (modifiers bitfield: Ctrl=2)
    let k_down: DispatchKeyEventParams = DispatchKeyEventParams::builder()
        .r#type(DispatchKeyEventType::KeyDown)
        .modifiers(2)
        .key(key)
        .code(format!("Key{}", key.to_ascii_uppercase()))
        .windows_virtual_key_code(vk)
        .native_virtual_key_code(vk)
        .build()
        .map_err(|e| anyhow::anyhow!(e))?;
    page.execute(k_down).await.context("failed combo keyDown")?;

    let k_up: DispatchKeyEventParams = DispatchKeyEventParams::builder()
        .r#type(DispatchKeyEventType::KeyUp)
        .modifiers(2)
        .key(key)
        .code(format!("Key{}", key.to_ascii_uppercase()))
        .windows_virtual_key_code(vk)
        .native_virtual_key_code(vk)
        .build()
        .map_err(|e| anyhow::anyhow!(e))?;
    page.execute(k_up).await.context("failed combo keyUp")?;

    // Ctrl keyup
    let ctrl_up: DispatchKeyEventParams = DispatchKeyEventParams::builder()
        .r#type(DispatchKeyEventType::KeyUp)
        .key("Control")
        .code("ControlLeft")
        .windows_virtual_key_code(17)
        .native_virtual_key_code(17)
        .build()
        .map_err(|e| anyhow::anyhow!(e))?;
    page.execute(ctrl_up).await.context("failed ctrl keyUp")?;

    Ok(())
}

async fn type_text_like_user(page: &Page, text: &str) -> Result<()> {
    // Small delay to ensure focus is stable.
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;

    let mut rng = rand::rng();
    let bytes = text.as_bytes();
    let mut idx = 0usize;
    let mut burst = 0usize;
    while idx < bytes.len() {
        let chunk_size = rng.random_range(10..=55);
        let end = (idx + chunk_size).min(bytes.len());
        let s = String::from_utf8_lossy(&bytes[idx..end]).to_string();
        // CDP input insertText uses the currently focused element.
        page.execute(InsertTextParams::from(s))
            .await
            .context("failed to insert text via CDP")?;

        burst += 1;
        if burst % rng.random_range(18..=35) == 0 {
            tokio::time::sleep(std::time::Duration::from_millis(
                rng.random_range(400..=1200),
            ))
            .await;
        } else {
            tokio::time::sleep(std::time::Duration::from_millis(rng.random_range(35..=160))).await;
        }

        idx = end;
    }
    Ok(())
}

async fn press_enter(page: &Page) -> Result<()> {
    let key_down: DispatchKeyEventParams = DispatchKeyEventParams::builder()
        .r#type(DispatchKeyEventType::KeyDown)
        .key("Enter")
        .code("Enter")
        .windows_virtual_key_code(13)
        .native_virtual_key_code(13)
        .text("\r")
        .unmodified_text("\r")
        .build()
        .map_err(|e| anyhow::anyhow!(e))?;

    page.execute(key_down)
        .await
        .context("failed to dispatch Enter keyDown")?;

    let key_up: DispatchKeyEventParams = DispatchKeyEventParams::builder()
        .r#type(DispatchKeyEventType::KeyUp)
        .key("Enter")
        .code("Enter")
        .windows_virtual_key_code(13)
        .native_virtual_key_code(13)
        .build()
        .map_err(|e| anyhow::anyhow!(e))?;

    page.execute(key_up)
        .await
        .context("failed to dispatch Enter keyUp")?;

    Ok(())
}

async fn ensure_logged_in(page: &Page) -> Result<()> {
    let res: serde_json::Value = page
        .evaluate(
            r#"(() => {
  const text = document.body ? document.body.innerText || '' : '';
  const hasSignIn = /sign in|log in/i.test(text);
  const hasEditor = document.querySelectorAll('[contenteditable="true"]').length > 0;
  return { hasSignIn, hasEditor, url: location.href };
})()"#,
        )
        .await?
        .into_value()?;

    let has_editor = res
        .get("hasEditor")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let has_sign_in = res
        .get("hasSignIn")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if !has_editor && has_sign_in {
        bail!(
            "Medium editor not available; please make sure you are logged in in the Chrome instance connected on :9222 (details: {})",
            res
        );
    }

    Ok(())
}

async fn open_publish_flow(page: &Page) {
    let res = page
        .evaluate(
            r#"(() => {
  const candidates = Array.from(document.querySelectorAll('button,a'));
  const btn = candidates.find((el) => (el.innerText || '').trim().toLowerCase() === 'publish');
  if (!btn) return { ok: false, error: 'publish button not found' };
  btn.click();
  return { ok: true };
})()"#,
        )
        .await;

    match res {
        Ok(v) => {
            let _ = v.into_value::<serde_json::Value>();
        }
        Err(err) => {
            warn!(error = %err, "failed to click publish button");
        }
    }
}

fn js_escape_for_template_literal(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('`', "\\`")
        .replace("${", "\\${")
}
