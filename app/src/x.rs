use anyhow::{Context, Result, bail};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use base64::Engine;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XToken {
    pub token_type: Option<String>,
    pub expires_in: Option<u64>,
    pub access_token: String,
    pub scope: Option<String>,
    pub refresh_token: Option<String>,
    // Unix epoch seconds
    pub obtained_at: Option<u64>,
}

pub fn default_token_path() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(PathBuf::from(home).join("x.json"))
}

pub fn load_token(path: &Path) -> Result<XToken> {
    let s = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let t: XToken = serde_json::from_str(&s).context("failed to parse x token json")?;
    Ok(t)
}

pub fn save_token(path: &Path, token: &XToken) -> Result<()> {
    let s = serde_json::to_string_pretty(token).context("failed to serialize token json")?;
    std::fs::write(path, s).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

pub async fn post_tweet_with_retry(
    token_path: &Path,
    client_id: &str,
    client_secret: Option<&str>,
    text: &str,
) -> Result<String> {
    let mut token = load_token(token_path)?;

    match create_tweet(&token.access_token, text).await {
        Ok(id) => return Ok(id),
        Err(err) => {
            // If it's unauthorized, try refresh.
            if let Some(status) = err.downcast_ref::<HttpStatusError>().map(|e| e.0) {
                if status != StatusCode::UNAUTHORIZED && status != StatusCode::FORBIDDEN {
                    return Err(err);
                }
            }
        }
    }

    let refreshed = refresh_token(&token, client_id, client_secret).await?;
    token.access_token = refreshed.access_token;
    token.refresh_token = refreshed.refresh_token.or(token.refresh_token);
    token.expires_in = refreshed.expires_in.or(token.expires_in);
    token.scope = refreshed.scope.or(token.scope);
    token.token_type = refreshed.token_type.or(token.token_type);
    token.obtained_at = Some(now_epoch_secs());

    save_token(token_path, &token)?;

    create_tweet(&token.access_token, text).await
}

#[derive(Debug)]
struct HttpStatusError(StatusCode);

impl std::fmt::Display for HttpStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "http status {}", self.0)
    }
}

impl std::error::Error for HttpStatusError {}

async fn create_tweet(access_token: &str, text: &str) -> Result<String> {
    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.twitter.com/2/tweets")
        .bearer_auth(access_token)
        .json(&serde_json::json!({"text": text}))
        .send()
        .await
        .context("failed to send create tweet request")?;

    let status = resp.status();
    let body = resp.text().await.context("failed to read tweet response")?;

    if !status.is_success() {
        return Err(anyhow::Error::new(HttpStatusError(status)).context(body));
    }

    let v: serde_json::Value =
        serde_json::from_str(&body).context("failed to parse tweet response")?;
    let id = v
        .get("data")
        .and_then(|d| d.get("id"))
        .and_then(|x| x.as_str())
        .context("missing tweet id")?;

    Ok(id.to_string())
}

async fn refresh_token(
    token: &XToken,
    client_id: &str,
    client_secret: Option<&str>,
) -> Result<XToken> {
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

    let mut new_token: XToken =
        serde_json::from_str(&text).context("failed to parse refresh json")?;
    new_token.obtained_at = Some(now_epoch_secs());
    Ok(new_token)
}

fn now_epoch_secs() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
