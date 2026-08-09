use chromiumoxide::page::Page;
use std::time::Duration;

const PICK_JS_BODY: &str = r#"  const norm = (s) => (s||'').toLowerCase().trim();
  const n = norm(needle);
  const tryPick = (sel) => {
    const items = Array.from(document.querySelectorAll(sel));
    return items.find(el => {
      const t = norm(el.innerText);
      return t === n || t.startsWith(n) || t.includes(n);
    }) || null;
  };
  const target = tryPick('[role=option],[role=menuitem],[role=listitem]')
    || tryPick('li')
    || tryPick('a');
  if (!target) { console.log('[board] could not find board item for:', n); return false; }
  console.log('[board] clicking board item:', (target.innerText||'').trim().slice(0,60));
  target.click();
  return true;"#;

pub(super) async fn pick_board_item(page: &Page, board_name: &str) -> anyhow::Result<bool> {
    let needle = serde_json::to_string(board_name).unwrap_or_else(|_| "\"\"".into());
    let js = format!("(() => {{\n  const needle = {};\n{}\n}})()", needle, PICK_JS_BODY);
    for _ in 0..6 {
        let picked = page.evaluate(js.as_str()).await?.into_value::<bool>().unwrap_or(false);
        if picked { return Ok(true); }
        tokio::time::sleep(Duration::from_millis(600)).await;
    }
    Ok(false)
}
