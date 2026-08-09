use anyhow::Result;
use std::path::Path;

pub async fn post_tweet_with_retry(
    token_path: &Path,
    client_id: &str,
    client_secret: Option<&str>,
    text: &str,
) -> Result<String> {
    let mut token = super::load_token::load_token(token_path)?;
    match super::create_tweet::create_tweet(&token.access_token, text).await {
        Ok(id) => return Ok(id),
        Err(err) if !super::is_auth_error::is_auth_error(&err) => return Err(err),
        _ => {}
    }
    let refreshed = super::refresh_token::refresh_token(&token, client_id, client_secret).await?;
    super::merge_token::merge_token(&mut token, refreshed);
    super::save_token::save_token(token_path, &token)?;
    super::create_tweet::create_tweet(&token.access_token, text).await
}
