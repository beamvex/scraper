pub fn extract_first_paragraph_text(html: &str) -> Option<String> {
    let start = html.to_lowercase().find("<p")?;
    let slice = &html[start..];
    let gt = slice.find('>')?;
    let after_open = &slice[(gt + 1)..];
    let end = after_open.to_lowercase().find("</p>")?;
    let inner = after_open[..end].trim();
    if inner.is_empty() {
        return None;
    }
    let mut out = String::with_capacity(inner.len());
    let mut in_tag = false;
    for ch in inner.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    let out = out.split_whitespace().collect::<Vec<_>>().join(" ");
    if out.is_empty() { None } else { Some(out) }
}
