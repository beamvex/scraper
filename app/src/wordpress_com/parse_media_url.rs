pub(super) fn parse_media_url(body_text: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(body_text)
        .ok()
        .and_then(|v| {
            v.get("media")
                .and_then(|m| m.get(0))
                .and_then(|m0| m0.get("URL").or_else(|| m0.get("url")))
                .and_then(|u| u.as_str())
                .map(|s| s.to_string())
        })
}
