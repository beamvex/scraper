use std::string::String;

pub fn sanitize_path_component(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        let keep = c.is_ascii_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.');
        if keep {
            out.push(c);
        } else {
            out.push('_');
        }
    }

    let out = out.trim().trim_matches('.').to_string();
    let out = out.split_whitespace().collect::<Vec<_>>().join(" ");

    if out.is_empty() {
        "unknown-product".to_string()
    } else {
        out.chars().take(120).collect()
    }
}
