use serde::{Deserialize, Serialize};

mod build_tweet_text;
mod create_tweet;
mod default_token_path;
mod load_token;
mod maybe_tweet_new_post;
mod now_epoch_secs;
mod post_tweet_with_retry;
mod refresh_token;
mod save_token;

pub use default_token_path::default_token_path;
pub use maybe_tweet_new_post::maybe_tweet_new_post;
pub use post_tweet_with_retry::post_tweet_with_retry;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XToken {
    pub token_type: Option<String>,
    pub expires_in: Option<u64>,
    pub access_token: String,
    pub scope: Option<String>,
    pub refresh_token: Option<String>,
    pub obtained_at: Option<u64>,
}

#[derive(Debug)]
pub(super) struct HttpStatusError(pub(super) reqwest::StatusCode);

impl std::fmt::Display for HttpStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "http status {}", self.0)
    }
}

impl std::error::Error for HttpStatusError {}
