use anyhow::{Context, Result, bail};
use chromiumoxide::browser::Browser;
use chromiumoxide::cdp::browser_protocol::input::InsertTextParams;
use chromiumoxide::cdp::browser_protocol::input::{DispatchKeyEventParams, DispatchKeyEventType};
use chromiumoxide::page::Page;
use rand::Rng;
use tracing::{info, warn};

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
