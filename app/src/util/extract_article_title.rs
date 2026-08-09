use super::extract_between_case_insensitive::extract_between_case_insensitive;
use super::html_decode_minimal::html_decode_minimal;

pub fn extract_article_title(html: &str) -> Option<String> {
    extract_between_case_insensitive(html, "<h1", "</h1>")
        .and_then(|s| strip_first_tag_and_trim(&s))
        .or_else(|| {
            extract_between_case_insensitive(html, "<title", "</title>")
                .and_then(|s| strip_first_tag_and_trim(&s))
        })
}

fn strip_first_tag_and_trim(s: &str) -> Option<String> {
    let gt = s.find('>')?;
    let inner = s[(gt + 1)..].trim();
    let inner = html_decode_minimal(inner);
    if inner.is_empty() { None } else { Some(inner) }
}
