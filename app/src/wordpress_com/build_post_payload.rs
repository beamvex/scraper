use serde_json::json;

pub(super) fn build_post_payload(
    title: &str,
    content: &str,
    status: &str,
    category: &str,
    featured_image: Option<String>,
) -> serde_json::Map<String, serde_json::Value> {
    let mut payload = serde_json::Map::new();
    payload.insert("title".into(), json!(title));
    payload.insert("content".into(), json!(content));
    payload.insert("status".into(), json!(status));
    payload.insert("categories".into(), json!(category));
    if let Some(fi) = featured_image { payload.insert("featured_image".into(), json!(fi)); }
    payload
}
