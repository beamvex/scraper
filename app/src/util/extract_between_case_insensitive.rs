pub fn extract_between_case_insensitive(
    haystack: &str,
    start_tag: &str,
    end_tag: &str,
) -> Option<String> {
    let h_lower = haystack.to_ascii_lowercase();
    let s_lower = start_tag.to_ascii_lowercase();
    let e_lower = end_tag.to_ascii_lowercase();

    let start_idx = h_lower.find(&s_lower)?;
    let after_start = &haystack[start_idx..];

    let end_lower_idx = h_lower[start_idx..].find(&e_lower)?;
    let end_idx = start_idx + end_lower_idx;

    Some(after_start[..(end_idx - start_idx)].to_string())
}
