use chromiumoxide::page::Page;
use std::time::Duration;

const OPEN_JS: &str = r#"(() => {
  const buttons = Array.from(document.querySelectorAll('button,div[role=button],a[role=button],[role=combobox]'));
  console.log('[board] total buttons:', buttons.length, buttons.map(b => (b.innerText||'').trim().slice(0,40)));
  const norm = (s) => (s||'').toLowerCase();
  const open = buttons.find(b => norm(b.innerText).includes('choose a board'))
    || buttons.find(b => norm(b.getAttribute('placeholder')||'').includes('board'))
    || buttons.find(b => norm(b.getAttribute('aria-label')||'').includes('board'))
    || buttons.find(b => norm(b.innerText) === 'board')
    || buttons.find(b => norm(b.innerText).includes('board'));
  if (!open) { console.log('[board] could not find board picker button'); return false; }
  console.log('[board] clicking board picker:', (open.innerText||'').trim(), open.getAttribute('aria-label')||'');
  open.click();
  return true;
})()"#;

pub(super) async fn open_board_picker(page: &Page) -> anyhow::Result<bool> {
    for _ in 0..12 {
        let opened = page.evaluate(OPEN_JS).await?.into_value::<bool>().unwrap_or(false);
        if opened { return Ok(true); }
        tokio::time::sleep(Duration::from_millis(600)).await;
    }
    Ok(false)
}
