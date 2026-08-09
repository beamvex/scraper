use std::path::PathBuf;

pub(super) fn load_x_credentials() -> Option<(String, Option<String>, PathBuf)> {
    let client_id = std::env::var("X_CLIENT_ID").ok().filter(|v| !v.trim().is_empty())?;
    let client_secret = std::env::var("X_CLIENT_SECRET").ok().filter(|s| !s.trim().is_empty());
    let token_path = match std::env::var("X_TOKEN_PATH") {
        Ok(p) if !p.trim().is_empty() => PathBuf::from(p),
        _ => super::default_token_path::default_token_path()?,
    };
    Some((client_id, client_secret, token_path))
}
