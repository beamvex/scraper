pub(super) fn build_fb_feed_form(
    message: String,
    access_token: &str,
    url: Option<&str>,
) -> Vec<(String, String)> {
    let mut form = vec![
        ("message".into(), message),
        ("access_token".into(), access_token.to_string()),
    ];
    if let Some(u) = url.filter(|u| !u.trim().is_empty()) {
        form.push(("link".into(), u.to_string()));
    }
    form
}
