use crate::util::{extract_between_case_insensitive, html_decode_minimal};

pub(super) fn extract_title(html: &str) -> Option<String> {
    if let Some(h1) = extract_between_case_insensitive(html, "<h1", "</h1>") {
        if let Some(gt) = h1.find('>') {
            let inner = h1[(gt + 1)..].trim();
            if !inner.is_empty() {
                return Some(html_decode_minimal(inner));
            }
        }
    }

    if let Some(t) = extract_between_case_insensitive(html, "<title", "</title>") {
        if let Some(gt) = t.find('>') {
            let inner = t[(gt + 1)..].trim();
            if !inner.is_empty() {
                return Some(html_decode_minimal(inner));
            }
        }
    }

    None
}
