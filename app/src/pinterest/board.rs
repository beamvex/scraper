use anyhow::Result;
use chromiumoxide::page::Page;
use std::path::Path;
use std::time::Duration;
use tracing::warn;

#[allow(dead_code)]
pub(super) async fn choose_board(page: &Page, board_url: &str) -> Result<()> {
    let board_name = board_url
        .trim_end_matches('/')
        .split('/')
        .last()
        .unwrap_or("random-thoughts")
        .replace('-', " ");

    // Click the board picker button — retry to handle late rendering.
    let open_js = r#"(() => {
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

    let mut opened = false;
    for _ in 0..12 {
        opened = page
            .evaluate(open_js)
            .await?
            .into_value::<bool>()
            .unwrap_or(false);
        if opened {
            break;
        }
        tokio::time::sleep(Duration::from_millis(600)).await;
    }

    if !opened {
        warn!("could not find pinterest board picker; continuing");
        let _ = super::write_debug_snapshot(page, Path::new("../data/pinterest_choose_board_debug.json"))
            .await;
        return Ok(());
    }

    tokio::time::sleep(Duration::from_millis(1200)).await;

    // Type into search box if one appeared (React-safe native setter).
    let search_js = format!(
        r#"(() => {{
  const needle = {};
  const inputs = Array.from(document.querySelectorAll('input'));
  const box = inputs.find(i => {{
    const ph = (i.getAttribute('placeholder')||'').toLowerCase();
    const aria = (i.getAttribute('aria-label')||'').toLowerCase();
    return ph.includes('search') || aria.includes('search') || ph.includes('board') || aria.includes('board');
  }});
  if (!box) return false;
  box.focus();
  try {{
    const ns = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value');
    if (ns && ns.set) ns.set.call(box, needle); else box.value = needle;
  }} catch(e) {{ box.value = needle; }}
  box.dispatchEvent(new Event('input', {{ bubbles: true }}));
  box.dispatchEvent(new Event('change', {{ bubbles: true }}));
  return true;
}})()
"#,
        serde_json::to_string(&board_name).unwrap_or_else(|_| "\"\"".to_string())
    );
    let _ = page.evaluate(search_js.as_str()).await;
    tokio::time::sleep(Duration::from_millis(800)).await;

    // Pick the board item — prefer role=option/menuitem/listitem, then li, then a.
    let pick_js = format!(
        r#"(() => {{
  const needle = {};
  const norm = (s) => (s||'').toLowerCase().trim();
  const n = norm(needle);
  const tryPick = (sel) => {{
    const items = Array.from(document.querySelectorAll(sel));
    return items.find(el => {{
      const t = norm(el.innerText);
      return t === n || t.startsWith(n) || t.includes(n);
    }}) || null;
  }};
  const target = tryPick('[role=option],[role=menuitem],[role=listitem]')
    || tryPick('li')
    || tryPick('a');
  if (!target) {{ console.log('[board] could not find board item for:', n); return false; }}
  console.log('[board] clicking board item:', (target.innerText||'').trim().slice(0,60));
  target.click();
  return true;
}})()
"#,
        serde_json::to_string(&board_name).unwrap_or_else(|_| "\"\"".to_string())
    );

    let mut picked = false;
    for _ in 0..6 {
        picked = page
            .evaluate(pick_js.as_str())
            .await?
            .into_value::<bool>()
            .unwrap_or(false);
        if picked {
            break;
        }
        tokio::time::sleep(Duration::from_millis(600)).await;
    }

    if !picked {
        warn!(board = %board_name, "failed to pick board by name; continuing");
        let _ = super::write_debug_snapshot(page, Path::new("../data/pinterest_choose_board_debug.json"))
            .await;
    }

    tokio::time::sleep(Duration::from_millis(800)).await;
    Ok(())
}
