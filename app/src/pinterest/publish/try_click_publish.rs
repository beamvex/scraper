use chromiumoxide::page::Page;
use std::time::Duration;

const CLICK_PUBLISH_JS: &str = r#"(() => {
  const btns = Array.from(document.querySelectorAll('button,div[role=button],a[role=button]'));
  console.log('[publish] total buttons found:', btns.length);
  console.log('[publish] buttons:', btns.map(b => ({
    tag: b.tagName, text: (b.innerText||'').trim().slice(0,80),
    aria: b.getAttribute('aria-label')||'', type: b.getAttribute('type')||'',
    role: b.getAttribute('role')||'',
  })));
  const norm = (s) => (s||'').toLowerCase().trim();
  const good = (b) => {
    const t = norm(b.innerText);
    const a = norm(b.getAttribute('aria-label')||'');
    return t === 'publish' || t === 'save' || a === 'publish' || a === 'save'
      || t.includes('publish') || t.includes('save pin') || a.includes('publish') || a.includes('save');
  };
  const target = btns.find(good) || btns.find(b => b.getAttribute('type') === 'submit');
  if (!target) { console.log('[publish] no publish/save button found'); return false; }
  console.log('[publish] clicking:', (target.innerText||'').trim(), target.getAttribute('aria-label')||'');
  target.click();
  return true;
})()"#;

pub(super) async fn try_click_publish(page: &Page) -> anyhow::Result<bool> {
    for _ in 0..14 {
        let clicked = page.evaluate(CLICK_PUBLISH_JS).await?.into_value::<bool>().unwrap_or(false);
        if clicked { return Ok(true); }
        tokio::time::sleep(Duration::from_millis(600)).await;
    }
    Ok(false)
}
