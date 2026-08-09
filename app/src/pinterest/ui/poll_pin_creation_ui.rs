use anyhow::Result;
use chromiumoxide::page::Page;
use std::time::Duration;

const POLL_JS: &str = r#"(() => {
  const inputs = document.querySelectorAll('input,textarea').length;
  const editables = document.querySelectorAll('[contenteditable="true"]').length;
  const fileInputs = document.querySelectorAll('input[type=file]').length;
  const buttons = document.querySelectorAll('button,div[role=button],a[role=button]').length;
  const iframes = document.querySelectorAll('iframe').length;
  return [inputs, editables, fileInputs, buttons, iframes];
})()"#;

pub(super) async fn poll_pin_creation_ui(page: &Page) -> Result<bool> {
    for _ in 0..360 {
        let counts: (u64, u64, u64, u64, u64) = page.evaluate(POLL_JS).await?
            .into_value::<(u64, u64, u64, u64, u64)>().unwrap_or((0, 0, 0, 0, 0));
        if counts.2 >= 1 { return Ok(true); }
        let formish = counts.0.saturating_add(counts.1);
        if counts.3 >= 6 || (counts.3 >= 4 && formish >= 3) { return Ok(true); }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    Ok(false)
}
