pub(super) fn truncate_description(description: Option<&str>) -> Option<String> {
    description
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| {
            if s.chars().count() > 450 {
                s.chars().take(447).collect::<String>() + "..."
            } else {
                s.to_string()
            }
        })
}
