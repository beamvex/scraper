use crate::util::extract_between_case_insensitive;

pub(super) fn extract_body_html(html: &str) -> Option<String> {
    let body = extract_between_case_insensitive(html, "<body", "</body>")?;
    let start = body.find('>')?;
    let inner = body[(start + 1)..].trim();
    if inner.is_empty() {
        None
    } else {
        Some(inner.to_string())
    }
}
