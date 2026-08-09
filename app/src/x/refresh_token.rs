use anyhow::{bail, Context, Result};
use base64::Engine;

pub(super) async fn refresh_token(
    token: &super::XToken,
    client_id: &str,
    client_secret: Option<&str>,
) -> Result<super::XToken> {
    let refresh = token
        .refresh_token
        .as_deref()
        .context("x.json missing refresh_token")?;

    let mut body = std::collections::HashMap::new();
    body.insert("grant_type", "refresh_token");
    body.insert("refresh_token", refresh);
    body.insert("client_id", client_id);

    let form = serde_urlencoded::to_string(&body).context("failed to encode refresh form")?;

    let client = reqwest::Client::new();
    let mut req = client
        .post("https://api.twitter.com/2/oauth2/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(form);

    if let Some(secret) = client_secret {
        let basic =
            base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", client_id, secret));
        req = req.header("Authorization", format!("Basic {}", basic));
    }

    let resp = req.send().await.context("failed to send refresh request")?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .context("failed to read refresh response")?;

    if !status.is_success() {
        bail!("refresh failed: HTTP {}: {}", status, text);
    }

    let mut new_token: super::XToken =
        serde_json::from_str(&text).context("failed to parse refresh json")?;
    new_token.obtained_at = Some(super::now_epoch_secs::now_epoch_secs());
    Ok(new_token)
}
