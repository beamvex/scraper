use anyhow::{Result, bail};
use chromiumoxide::page::Page;
use std::path::Path;
use std::time::Duration;
use tracing::info;

pub(super) async fn publish_pin(page: &Page) -> Result<()> {
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
        let _ = super::write_debug_snapshot(page, Path::new("../data/pinterest_publish_debug.json")).await;
        bail!("could not find pinterest publish/save button")
    }

    tokio::time::sleep(Duration::from_millis(3500)).await;
    let _ = super::write_debug_snapshot(
        page,
        Path::new("../data/pinterest_publish_after_debug.json"),
    )
    .await;
    info!("attempted to publish pinterest pin");
    Ok(())
}
