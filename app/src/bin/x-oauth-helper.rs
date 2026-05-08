use anyhow::{Context, Result, bail};
use axum::{
    Form, Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect},
    routing::{get, post},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use once_cell::sync::Lazy;
use rand::Rng;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tracing::{info, warn};

static CLIENT: Lazy<reqwest::Client> = Lazy::new(reqwest::Client::new);

#[derive(Clone, Default)]
struct AppState {
    inner: Arc<Mutex<Inner>>,
}

#[derive(Default)]
struct Inner {
    cfg: Option<Config>,
    // last token response
    token_json: Option<String>,
}

#[derive(Clone)]
struct Config {
    client_id: String,
    client_secret: Option<String>,
    redirect_uri: String,
    scope: String,
    state: String,
    code_verifier: String,
    code_challenge: String,
}

#[derive(Deserialize)]
struct StartForm {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    scope: String,
    use_client_secret: Option<String>,
}

#[derive(Deserialize)]
struct CallbackParams {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    if let Ok(home) = std::env::var("HOME") {
        let _ = dotenvy::from_filename(format!("{}/.env", home));
    }

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let state = AppState::default();

    let app = Router::new()
        .route("/", get(index))
        .route("/start", post(start))
        .route("/callback", get(callback))
        .route("/token", get(show_token))
        .with_state(state);

    let addr = "127.0.0.1:8085";
    info!(%addr, "x oauth helper listening");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context("failed to bind")?;

    axum::serve(listener, app).await.context("server error")?;
    Ok(())
}

async fn index(State(state): State<AppState>) -> impl IntoResponse {
    let inner = state.inner.lock().unwrap();
    let token_snip = inner
        .token_json
        .as_deref()
        .unwrap_or("(no token response yet)");

    let html = format!(
        r#"<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <title>X OAuth Helper</title>
    <style>
      body {{ font-family: system-ui, -apple-system, Segoe UI, Roboto, sans-serif; margin: 24px; max-width: 960px; }}
      input[type=text], input[type=password] {{ width: 100%; padding: 10px; margin: 6px 0 14px; box-sizing: border-box; }}
      textarea {{ width: 100%; height: 220px; }}
      .row {{ margin-bottom: 10px; }}
      .hint {{ color: #666; font-size: 12px; margin-top: -10px; margin-bottom: 12px; }}
      .btn {{ padding: 10px 14px; font-weight: 600; }}
      code {{ background: #f4f4f4; padding: 2px 5px; }}
    </style>
  </head>
  <body>
    <h1>X OAuth Helper</h1>

    <form method="post" action="/start">
      <div class="row">
        <label>Client ID</label>
        <input name="client_id" type="text" placeholder="your client id" required />
      </div>

      <div class="row">
        <label>Client Secret (optional)</label>
        <input name="client_secret" type="password" placeholder="only needed for confidential clients" />
        <div class="hint">If you leave this blank, we attempt PKCE-only token exchange (public client style).</div>
      </div>

      <div class="row">
        <label>Redirect URI</label>
        <input name="redirect_uri" type="text" value="http://127.0.0.1:8085/callback" required />
        <div class="hint">This must exactly match the redirect URI configured in your X developer app.</div>
      </div>

      <div class="row">
        <label>Scopes</label>
        <input name="scope" type="text" value="tweet.read tweet.write users.read offline.access" required />
      </div>

      <div class="row">
        <label>
          <input type="checkbox" name="use_client_secret" value="1" />
          Use client secret (confidential client)
        </label>
      </div>

      <button class="btn" type="submit">Authorize on X</button>
    </form>

    <h2>Last token response</h2>
    <textarea readonly>{}</textarea>

    <p>Direct link: <a href="/token">/token</a></p>
  </body>
</html>"#,
        escape_html(token_snip)
    );

    Html(html)
}

async fn start(State(state): State<AppState>, Form(form): Form<StartForm>) -> impl IntoResponse {
    match start_inner(state, form).await {
        Ok(resp) => resp,
        Err(err) => (StatusCode::BAD_REQUEST, format!("{}", err)).into_response(),
    }
}

async fn start_inner(state: AppState, form: StartForm) -> Result<axum::response::Response> {
    if form.client_id.trim().is_empty() {
        bail!("client_id is required");
    }

    let redirect_uri = form.redirect_uri.trim().to_string();
    if redirect_uri.is_empty() {
        bail!("redirect_uri is required");
    }

    let scope = form.scope.trim().to_string();
    if scope.is_empty() {
        bail!("scope is required");
    }

    let client_secret = if form.use_client_secret.is_some() {
        let s = form.client_secret.trim().to_string();
        if s.is_empty() {
            bail!("use_client_secret checked but client_secret empty");
        }
        Some(s)
    } else {
        None
    };

    let (code_verifier, code_challenge) = gen_pkce();

    let oauth_state = gen_state();

    let cfg = Config {
        client_id: form.client_id.trim().to_string(),
        client_secret,
        redirect_uri: redirect_uri.clone(),
        scope: scope.clone(),
        state: oauth_state.clone(),
        code_verifier,
        code_challenge,
    };

    {
        let mut inner = state.inner.lock().unwrap();
        inner.cfg = Some(cfg.clone());
        inner.token_json = None;
    }

    let authorize_url = format!(
        "https://twitter.com/i/oauth2/authorize?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&code_challenge={}&code_challenge_method=S256",
        urlencoding::encode(&cfg.client_id),
        urlencoding::encode(&cfg.redirect_uri),
        urlencoding::encode(&cfg.scope),
        urlencoding::encode(&cfg.state),
        urlencoding::encode(&cfg.code_challenge),
    );

    Ok(Redirect::temporary(&authorize_url).into_response())
}

async fn callback(
    State(state): State<AppState>,
    Query(params): Query<CallbackParams>,
) -> impl IntoResponse {
    match callback_inner(state, params).await {
        Ok(resp) => resp,
        Err(err) => (StatusCode::BAD_REQUEST, format!("{}", err)).into_response(),
    }
}

async fn callback_inner(
    state: AppState,
    params: CallbackParams,
) -> Result<axum::response::Response> {
    if let Some(err) = params.error {
        let msg = format!(
            "oauth error: {}{}",
            err,
            params
                .error_description
                .as_ref()
                .map(|d| format!(" ({})", d))
                .unwrap_or_default()
        );
        warn!(%msg, "oauth callback error");
        return Ok((StatusCode::BAD_REQUEST, msg).into_response());
    }

    let code = params.code.context("missing code")?;

    let cfg = {
        let inner = state.inner.lock().unwrap();
        inner
            .cfg
            .clone()
            .context("no config in memory; start at /")?
    };

    if params.state.as_deref() != Some(&cfg.state) {
        bail!("state mismatch");
    }

    let token_json = exchange_code_for_token(&cfg, &code).await?;

    {
        let mut inner = state.inner.lock().unwrap();
        inner.token_json = Some(token_json);
    }

    Ok(Redirect::to("/").into_response())
}

async fn show_token(State(state): State<AppState>) -> impl IntoResponse {
    let inner = state.inner.lock().unwrap();
    let token = inner
        .token_json
        .as_deref()
        .unwrap_or("(no token response yet)")
        .to_string();

    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        "application/json; charset=utf-8".parse().unwrap(),
    );

    (headers, token)
}

async fn exchange_code_for_token(cfg: &Config, code: &str) -> Result<String> {
    let url = "https://api.twitter.com/2/oauth2/token";

    let mut body: HashMap<&str, String> = HashMap::new();
    body.insert("grant_type", "authorization_code".to_string());
    body.insert("client_id", cfg.client_id.clone());
    body.insert("code", code.to_string());
    body.insert("redirect_uri", cfg.redirect_uri.clone());
    body.insert("code_verifier", cfg.code_verifier.clone());

    let form_body = serde_urlencoded::to_string(&body).context("failed to encode form body")?;

    let mut req = CLIENT
        .post(url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(form_body);

    if let Some(secret) = &cfg.client_secret {
        // Use HTTP Basic auth for confidential clients.
        let basic = base64::engine::general_purpose::STANDARD
            .encode(format!("{}:{}", cfg.client_id, secret));
        req = req.header("Authorization", format!("Basic {}", basic));
    }

    let resp = req.send().await.context("token request failed")?;

    let status = resp.status();
    let text = resp.text().await.context("failed to read token response")?;

    if !status.is_success() {
        bail!("token exchange failed: HTTP {}: {}", status, text);
    }

    Ok(text)
}

fn gen_state() -> String {
    let mut rng = rand::rng();
    let bytes: [u8; 16] = rng.random();
    URL_SAFE_NO_PAD.encode(bytes)
}

fn gen_pkce() -> (String, String) {
    // RFC 7636 allows 43-128 chars. We'll generate 64 chars from the unreserved set.
    const ALPH: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~";

    let mut rng = rand::rng();
    let mut v = String::with_capacity(64);
    for _ in 0..64 {
        let idx = rng.random_range(0..ALPH.len());
        v.push(ALPH[idx] as char);
    }

    let digest = Sha256::digest(v.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(digest);

    (v, challenge)
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
