use base64::Engine;

pub(super) fn build_refresh_request(
    client_id: &str,
    client_secret: Option<&str>,
    refresh: &str,
) -> reqwest::RequestBuilder {
    let mut body = std::collections::HashMap::new();
    body.insert("grant_type", "refresh_token");
    body.insert("refresh_token", refresh);
    body.insert("client_id", client_id);
    let form = serde_urlencoded::to_string(&body).unwrap_or_default();
    let mut req = reqwest::Client::new()
        .post("https://api.twitter.com/2/oauth2/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(form);
    if let Some(secret) = client_secret {
        let basic = base64::engine::general_purpose::STANDARD
            .encode(format!("{}:{}", client_id, secret));
        req = req.header("Authorization", format!("Basic {}", basic));
    }
    req
}
