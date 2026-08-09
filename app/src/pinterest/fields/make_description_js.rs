const DESC_BODY_JS: &str = r#"  // Try known aria-label first (Pinterest uses "Describe your Pin").
  const byAria = document.querySelector('[aria-label="Describe your Pin"]');
  if (byAria) { setValue(byAria, value); return true; }
  // Fallback: heuristic scoring.
  const els = Array.from(document.querySelectorAll('input,textarea,[contenteditable="true"]'));
  const score = (el) => {
    if (el.getAttribute('type') === 'hidden') return -100;
    const tag = (el.tagName||'').toLowerCase();
    const name = (el.getAttribute('name')||'').toLowerCase();
    const ph = (el.getAttribute('placeholder')||'').toLowerCase();
    const aria = (el.getAttribute('aria-label')||'').toLowerCase();
    const id = (el.getAttribute('id')||'').toLowerCase();
    const ce = (el.getAttribute('contenteditable')||'').toLowerCase();
    const s = name + ' ' + ph + ' ' + aria + ' ' + id;
    let sc = 0;
    if (tag === 'textarea') sc += 2;
    if (tag === 'div' && ce === 'true') sc += 1;
    if (s.includes('description')) sc += 3;
    if (s.includes('describe')) sc += 3;
    if (s.includes('tell everyone') || s.includes('add a description')) sc += 3;
    if (s.includes('details')) sc += 1;
    return sc;
  };
  let best = null; let bestScore = 0;
  for (const el of els) { const sc = score(el); if (sc > bestScore) { bestScore = sc; best = el; } }
  if (!best || bestScore < 2) return false;
  setValue(best, value);
  return true;"#;

pub(super) fn make_description_js(desc: &str) -> String {
    let v = serde_json::to_string(desc).unwrap_or_else(|_| "\"\"".into());
    format!("(() => {{\n  const value = {};\n  {}\n{}\n}})()", v, super::SET_VALUE_FN_JS, DESC_BODY_JS)
}
