use anyhow::{bail, Context, Result};

pub(super) async fn refresh_token(
    token: &super::XToken,
    client_id: &str,
    client_secret: Option<&str>,
) -> Result<super::XToken> {
    let refresh = token.refresh_token.as_deref().context("x.json missing refresh_token")?;
    let req = super::build_refresh_request::build_refresh_request(client_id, client_secret, refresh);
    let resp = req.send().await.context("failed to send refresh request")?;
    let status = resp.status();
    let text = resp.text().await.context("failed to read refresh response")?;
    if !status.is_success() { bail!("refresh failed: HTTP {}: {}", status, text); }
    let mut new_token: super::XToken =
        serde_json::from_str(&text).context("failed to parse refresh json")?;
    new_token.obtained_at = Some(super::now_epoch_secs::now_epoch_secs());
    Ok(new_token)
}
