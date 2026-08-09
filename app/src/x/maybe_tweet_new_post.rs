use anyhow::{Context, Result};
use tracing::info;

pub async fn maybe_tweet_new_post(title: &str, post_url: Option<&str>) -> Result<()> {
    let Some(url) = post_url.filter(|u| !u.trim().is_empty()) else {
        return Ok(());
    };

    // Only tweet 1/10 of the time.
    if rand::random::<u8>() % 10 != 0 {
        return Ok(());
    }

    // Default: enabled if X_CLIENT_ID is set.
    let client_id = match std::env::var("X_CLIENT_ID") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => return Ok(()),
    };

    let client_secret = std::env::var("X_CLIENT_SECRET")
        .ok()
        .filter(|s| !s.trim().is_empty());

    let token_path = match std::env::var("X_TOKEN_PATH") {
        Ok(p) if !p.trim().is_empty() => std::path::PathBuf::from(p),
        _ => super::default_token_path().context("HOME not set; cannot infer X token path")?,
    };

    let tweet_text = super::build_tweet_text::build_tweet_text(title, url);
    let tweet_id = super::post_tweet_with_retry(
        &token_path,
        &client_id,
        client_secret.as_deref(),
        &tweet_text,
    )
    .await?;

    info!(%tweet_id, "posted X tweet");
    Ok(())
}
