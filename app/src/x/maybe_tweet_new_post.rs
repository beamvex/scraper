use anyhow::Result;
use tracing::info;

pub async fn maybe_tweet_new_post(title: &str, post_url: Option<&str>) -> Result<()> {
    let Some(url) = post_url.filter(|u| !u.trim().is_empty()) else { return Ok(()); };
    if rand::random::<u8>() % 10 != 0 { return Ok(()); }
    let Some((client_id, client_secret, token_path)) =
        super::load_x_credentials::load_x_credentials()
    else { return Ok(()); };
    let tweet_text = super::build_tweet_text::build_tweet_text(title, url);
    let tweet_id = super::post_tweet_with_retry(
        &token_path, &client_id, client_secret.as_deref(), &tweet_text,
    ).await?;
    info!(%tweet_id, "posted X tweet");
    Ok(())
}
