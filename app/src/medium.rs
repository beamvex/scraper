use anyhow::{Context, Result, bail};
use chromiumoxide::browser::Browser;
use chromiumoxide::page::Page;
use tracing::{info, warn};

pub async fn create_medium_draft(browser: &Browser, title: &str, body_text: &str) -> Result<()> {
    info!("opening medium new story");
    let page = browser
        .new_page("https://medium.com/new-story")
        .await
        .context("failed to open medium new story page")?;

    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

    ensure_logged_in(&page).await?;

    let payload_title = js_escape_for_template_literal(title);
    let payload_body = js_escape_for_template_literal(body_text);

    let res: serde_json::Value = page
        .evaluate(format!(
            r#"(() => {{
  const title = `{payload_title}`;
  const body = `{payload_body}`;

  const isEditable = (el) => !!el && el.getAttribute && el.getAttribute('contenteditable') === 'true';

  const editables = Array.from(document.querySelectorAll('[contenteditable="true"]'));
  if (!editables.length) {{
    return {{ ok: false, error: 'no contenteditable elements found (are you blocked by a login wall?)' }};
  }}

  const titleEl = editables.find((el) => el.tagName === 'H1') || editables[0];
  titleEl.focus();
  document.execCommand('selectAll', false, null);
  document.execCommand('insertText', false, title);

  const bodyEl = editables.reverse().find((el) => el.tagName !== 'H1') || titleEl;
  bodyEl.focus();
  document.execCommand('insertText', false, '\n');
  document.execCommand('insertText', false, body);

  return {{ ok: true, editables: editables.length }};
}})()"#
        ))
        .await?
        .into_value()?;

    if res.get("ok").and_then(|v| v.as_bool()) != Some(true) {
        bail!("failed to populate Medium editor: {}", res);
    }

    info!(details = %res, "populated Medium editor");

    open_publish_flow(&page).await;

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
