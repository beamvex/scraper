const LINK_BODY_JS: &str = r#"  // Try known IDs first.
  const byId = document.getElementById('WebsiteField');
  if (byId) { setValue(byId, value); return true; }
  // Fallback: heuristic scoring.
  const els = Array.from(document.querySelectorAll('input,textarea,[contenteditable="true"]'));
  const score = (el) => {
    const name = (el.getAttribute('name')||'').toLowerCase();
    const ph = (el.getAttribute('placeholder')||'').toLowerCase();
    const aria = (el.getAttribute('aria-label')||'').toLowerCase();
    const id = (el.getAttribute('id')||'').toLowerCase();
    const tag = (el.tagName||'').toLowerCase();
    const ce = (el.getAttribute('contenteditable')||'').toLowerCase();
    const s = name + ' ' + ph + ' ' + aria + ' ' + id;
    if (s.includes('destination')) return 3;
    if (s.includes('website')) return 3;
    if (s.includes('add a link')) return 3;
    if (s.includes('link')) return 2;
    if (s.includes('url')) return 2;
    if (s.includes('source')) return 2;
    if (tag === 'input' && (el.getAttribute('type')||'').toLowerCase() === 'url') return 2;
    if (tag === 'div' && ce === 'true') return 1;
    return 0;
  };
  let best = null; let bestScore = 0;
  for (const el of els) { const sc = score(el); if (sc > bestScore) { bestScore = sc; best = el; } }
  if (!best || bestScore < 2) return false;
  setValue(best, value);
  return true;"#;

pub(super) fn make_link_js(link: &str) -> String {
    let v = serde_json::to_string(link).unwrap_or_else(|_| "\"\"".into());
    format!("(() => {{\n  const value = {};\n  {}\n{}\n}})()", v, super::SET_VALUE_FN_JS, LINK_BODY_JS)
}
