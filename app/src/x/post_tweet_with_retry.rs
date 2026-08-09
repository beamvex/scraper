use anyhow::Result;
use reqwest::StatusCode;
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
        Err(err) => {
            if let Some(status) = err.downcast_ref::<super::HttpStatusError>().map(|e| e.0) {
                if status != StatusCode::UNAUTHORIZED && status != StatusCode::FORBIDDEN {
                    return Err(err);
                }
            }
        }
    }

    let refreshed = super::refresh_token::refresh_token(&token, client_id, client_secret).await?;
    token.access_token = refreshed.access_token;
    token.refresh_token = refreshed.refresh_token.or(token.refresh_token);
    token.expires_in = refreshed.expires_in.or(token.expires_in);
    token.scope = refreshed.scope.or(token.scope);
    token.token_type = refreshed.token_type.or(token.token_type);
    token.obtained_at = Some(super::now_epoch_secs::now_epoch_secs());

    super::save_token::save_token(token_path, &token)?;

    super::create_tweet::create_tweet(&token.access_token, text).await
}
