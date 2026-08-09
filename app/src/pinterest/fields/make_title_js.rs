const TITLE_BODY_JS: &str = r#"  // Try known IDs first.
  const byId = document.getElementById('storyboard-selector-title');
  if (byId) { setValue(byId, value); return true; }
  // Fallback: heuristic scoring.
  const els = Array.from(document.querySelectorAll('input,textarea,[contenteditable="true"]'));
  const score = (el) => {
    if (el.getAttribute('type') === 'hidden') return -100;
    const name = (el.getAttribute('name')||'').toLowerCase();
    const ph = (el.getAttribute('placeholder')||'').toLowerCase();
    const aria = (el.getAttribute('aria-label')||'').toLowerCase();
    const id = (el.getAttribute('id')||'').toLowerCase();
    const tag = (el.tagName||'').toLowerCase();
    const ce = (el.getAttribute('contenteditable')||'').toLowerCase();
    const s = name + ' ' + ph + ' ' + aria + ' ' + id;
    let sc = 0;
    if (tag === 'input') sc += 1;
    if (tag === 'div' && ce === 'true') sc += 1;
    if (s.includes('title')) sc += 3;
    if (s.includes('pin title')) sc += 3;
    if (s.includes('tell everyone')) sc += 3;
    if (s.includes('your pin is about')) sc += 3;
    if (s.includes('add your title')) sc += 3;
    if (s.includes('your title')) sc += 2;
    return sc;
  };
  let best = null; let bestScore = 0;
  for (const el of els) { const sc = score(el); if (sc > bestScore) { bestScore = sc; best = el; } }
  if (!best || bestScore < 2) return false;
  setValue(best, value);
  return true;"#;

pub(super) fn make_title_js(title: &str) -> String {
    let v = serde_json::to_string(title).unwrap_or_else(|_| "\"\"".into());
    format!("(() => {{\n  const value = {};\n  {}\n{}\n}})()", v, super::SET_VALUE_FN_JS, TITLE_BODY_JS)
}
