use anyhow::{Context, Result, bail};
use chromiumoxide::browser::Browser;
use chromiumoxide::page::Page;
use std::env;
use std::path::Path;
use std::time::Duration;
use tracing::info;

mod board;
mod fields;
mod publish;
mod ui;
mod upload;

#[cfg(test)]
mod tests;

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

    ui::wait_for_pin_creation_ui(&page).await?;

    if ui::is_login_interstitial(&page).await.unwrap_or(false) {
        bail!(
            "pinterest appears to require login in the current browser session (redirected to login)"
        );
    }

    upload::upload_image(&page, image_path).await?;
    fields::fill_text_fields(&page, title, description, link).await?;
    // board::choose_board(&page, board_url).await?;
    publish::publish_pin(&page).await?;

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
