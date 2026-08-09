use chromiumoxide::page::Page;
use std::time::Duration;

const SEARCH_JS_BODY: &str = r#"  const inputs = Array.from(document.querySelectorAll('input'));
  const box = inputs.find(i => {
    const ph = (i.getAttribute('placeholder')||'').toLowerCase();
    const aria = (i.getAttribute('aria-label')||'').toLowerCase();
    return ph.includes('search') || aria.includes('search') || ph.includes('board') || aria.includes('board');
  });
  if (!box) return false;
  box.focus();
  try {
    const ns = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value');
    if (ns && ns.set) ns.set.call(box, needle); else box.value = needle;
  } catch(e) { box.value = needle; }
  box.dispatchEvent(new Event('input', { bubbles: true }));
  box.dispatchEvent(new Event('change', { bubbles: true }));
  return true;"#;

pub(super) async fn type_board_name(page: &Page, board_name: &str) -> anyhow::Result<()> {
    let needle = serde_json::to_string(board_name).unwrap_or_else(|_| "\"\"".into());
    let js = format!("(() => {{\n  const needle = {};\n{}\n}})()", needle, SEARCH_JS_BODY);
    let _ = page.evaluate(js.as_str()).await;
    tokio::time::sleep(Duration::from_millis(800)).await;
    Ok(())
}
