use anyhow::{Context, Result, bail};
use axum::{
    Form, Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect},
    routing::{get, post},
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    sync::{Arc, Mutex},
};
use tracing::{info, warn};

static CLIENT: Lazy<reqwest::Client> = Lazy::new(reqwest::Client::new);

const FB_INDEX_HTML: &str = r#"<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <title>Facebook OAuth Helper</title>
    <style>
      body { font-family: system-ui, -apple-system, Segoe UI, Roboto, sans-serif; margin: 24px; max-width: 960px; }
      input[type=text], input[type=password] { width: 100%; padding: 10px; margin: 6px 0 14px; box-sizing: border-box; }
      textarea { width: 100%; height: 260px; }
      .row { margin-bottom: 10px; }
      .hint { color: #666; font-size: 12px; margin-top: -10px; margin-bottom: 12px; }
      .btn { padding: 10px 14px; font-weight: 600; }
      code { background: #f4f4f4; padding: 2px 5px; }
    </style>
  </head>
  <body>
    <h1>Facebook OAuth Helper</h1>
    <form method="post" action="/start">
      <div class="row"><label>App ID</label>
        <input name="app_id" type="text" placeholder="facebook app id" required /></div>
      <div class="row"><label>App Secret</label>
        <input name="app_secret" type="password" placeholder="facebook app secret" required /></div>
      <div class="row"><label>Redirect URI</label>
        <input name="redirect_uri" type="text" value="http://127.0.0.1:8086/callback" required />
        <div class="hint">Must exactly match the redirect URI configured in your Facebook app.</div></div>
      <div class="row"><label>Scopes</label>
        <input name="scope" type="text" value="pages_manage_posts,pages_read_engagement,pages_show_list" required />
        <div class="hint">You may need <code>pages_manage_posts</code> + <code>pages_show_list</code>.</div></div>
      <div class="row"><label>Page ID (optional)</label>
        <input name="page_id" type="text" placeholder="if blank, we use the first page from /me/accounts" /></div>
      <button class="btn" type="submit">Authorize on Facebook</button>
    </form>
    <h2>Last response</h2>
    <textarea readonly>{CONTENT}</textarea>
    <p>Direct link: <a href="/token">/token</a></p>
  </body>
</html>"#;

#[derive(Clone, Default)]
struct AppState {
    inner: Arc<Mutex<Inner>>,
}

#[derive(Default)]
struct Inner {
    cfg: Option<Config>,
    last_json: Option<String>,
}

#[derive(Clone)]
struct Config {
    app_id: String,
    app_secret: String,
    redirect_uri: String,
    scope: String,
    state: String,
    page_id: Option<String>,
}

#[derive(Deserialize)]
struct StartForm {
    app_id: String,
    app_secret: String,
    redirect_uri: String,
    scope: String,
    page_id: String,
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
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).init();
    let state = AppState::default();
    let app = Router::new()
        .route("/", get(index))
        .route("/start", post(start))
        .route("/callback", get(callback))
        .route("/token", get(show_token))
        .with_state(state);
    let addr = "127.0.0.1:8086";
    info!(%addr, "facebook oauth helper listening");
    let listener = tokio::net::TcpListener::bind(addr).await.context("failed to bind")?;
    axum::serve(listener, app).await.context("server error")?;
    Ok(())
}

async fn index(State(state): State<AppState>) -> impl IntoResponse {
    let inner = state.inner.lock().unwrap();
    let snip = inner.last_json.as_deref().unwrap_or("(no token response yet)");
    Html(FB_INDEX_HTML.replace("{CONTENT}", &escape_html(snip)))
}

async fn start(State(state): State<AppState>, Form(form): Form<StartForm>) -> impl IntoResponse {
    match start_inner(state, form).await {
        Ok(resp) => resp,
        Err(err) => (StatusCode::BAD_REQUEST, format!("{}", err)).into_response(),
    }
}

async fn start_inner(state: AppState, form: StartForm) -> Result<axum::response::Response> {
    if form.app_id.trim().is_empty() { bail!("app_id is required"); }
    if form.app_secret.trim().is_empty() { bail!("app_secret is required"); }
    let redirect_uri = form.redirect_uri.trim().to_string();
    if redirect_uri.is_empty() { bail!("redirect_uri is required"); }
    let scope = form.scope.trim().to_string();
    if scope.is_empty() { bail!("scope is required"); }
    let oauth_state = format!("{}", rand::random::<u64>());
    let page_id = form.page_id.trim().to_string();
    let cfg = Config {
        app_id: form.app_id.trim().to_string(), app_secret: form.app_secret.trim().to_string(),
        redirect_uri: redirect_uri.clone(), scope: scope.clone(), state: oauth_state.clone(),
        page_id: (!page_id.is_empty()).then_some(page_id),
    };
    { let mut inner = state.inner.lock().unwrap(); inner.cfg = Some(cfg.clone()); inner.last_json = None; }
    let authorize_url = format!(
        "https://www.facebook.com/v20.0/dialog/oauth?client_id={}&redirect_uri={}&state={}&scope={}&response_type=code",
        urlencoding::encode(&cfg.app_id), urlencoding::encode(&cfg.redirect_uri),
        urlencoding::encode(&cfg.state), urlencoding::encode(&cfg.scope),
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

async fn callback_inner(state: AppState, params: CallbackParams) -> Result<axum::response::Response> {
    if let Some(err) = params.error {
        let msg = format!("oauth error: {}{}", err,
            params.error_description.as_ref().map(|d| format!(" ({})", d)).unwrap_or_default());
        warn!(%msg, "oauth callback error");
        return Ok((StatusCode::BAD_REQUEST, msg).into_response());
    }
    let code = params.code.context("missing code")?;
    let cfg = { let inner = state.inner.lock().unwrap(); inner.cfg.clone().context("no config in memory; start at /")? };
    if params.state.as_deref() != Some(&cfg.state) { bail!("state mismatch"); }
    let final_json = run_flow(&cfg, &code).await?;
    { let mut inner = state.inner.lock().unwrap(); inner.last_json = Some(final_json); }
    Ok(Redirect::to("/").into_response())
}

async fn show_token(State(state): State<AppState>) -> impl IntoResponse {
    let inner = state.inner.lock().unwrap();
    let token = inner
        .last_json
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

async fn run_flow(cfg: &Config, code: &str) -> Result<String> {
    let short = exchange_code_for_short_lived_token(cfg, code).await?;
    let long = exchange_for_long_lived_token(cfg, &short).await?;

    let pages = get_pages(&long).await?;

    let selected = select_page(cfg.page_id.as_deref(), &pages)
        .context("no page access token found; check permissions and that your user manages a Page")?;

    Ok(serde_json::to_string_pretty(&json!({
        "how_to_use": {
            "env": {
                "FB_PAGE_ID": selected.id,
                "FB_PAGE_ACCESS_TOKEN": selected.access_token,
            }
        },
        "user_access_token_long_lived": long,
        "pages": pages,
    }))?)
}

async fn exchange_code_for_short_lived_token(cfg: &Config, code: &str) -> Result<String> {
    let url = "https://graph.facebook.com/v20.0/oauth/access_token";
    let qs = serde_urlencoded::to_string([
        ("client_id", cfg.app_id.as_str()), ("redirect_uri", cfg.redirect_uri.as_str()),
        ("client_secret", cfg.app_secret.as_str()), ("code", code),
    ]).context("failed to build facebook token query string")?;
    let resp = CLIENT.get(format!("{}?{}", url, qs)).send().await.context("facebook token request failed")?;
    let status = resp.status();
    let text = resp.text().await.context("failed to read token response")?;
    if !status.is_success() { bail!("facebook token exchange failed: HTTP {}: {}", status, text); }
    let v: serde_json::Value = serde_json::from_str(&text).context("failed to parse token json")?;
    Ok(v.get("access_token").and_then(|x| x.as_str()).context("missing access_token")?.to_string())
}

async fn exchange_for_long_lived_token(cfg: &Config, short: &str) -> Result<String> {
    let url = "https://graph.facebook.com/v20.0/oauth/access_token";
    let qs = serde_urlencoded::to_string([
        ("grant_type", "fb_exchange_token"), ("client_id", cfg.app_id.as_str()),
        ("client_secret", cfg.app_secret.as_str()), ("fb_exchange_token", short),
    ]).context("failed to build facebook long-lived token query string")?;
    let resp = CLIENT.get(format!("{}?{}", url, qs)).send().await.context("facebook long-lived token request failed")?;
    let status = resp.status();
    let text = resp.text().await.context("failed to read long-lived response")?;
    if !status.is_success() { bail!("facebook long-lived token exchange failed: HTTP {}: {}", status, text); }
    let v: serde_json::Value = serde_json::from_str(&text).context("failed to parse long-lived json")?;
    Ok(v.get("access_token").and_then(|x| x.as_str()).context("missing access_token in long-lived response")?.to_string())
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct AccountsResponse {
    data: Vec<PageInfo>,
    #[serde(default)]
    paging: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PageInfo {
    id: String,
    name: Option<String>,
    access_token: String,
}

async fn get_pages(user_access_token: &str) -> Result<Vec<PageInfo>> {
    // GET /me/accounts
    let url = "https://graph.facebook.com/v20.0/me/accounts";
    let qs = serde_urlencoded::to_string([("access_token", user_access_token)])
        .context("failed to build facebook /me/accounts query string")?;

    let full_url = format!("{}?{}", url, qs);
    let resp = CLIENT
        .get(full_url)
        .send()
        .await
        .context("facebook /me/accounts request failed")?;

    let status = resp.status();
    let text = resp.text().await.context("failed to read /me/accounts response")?;
    if !status.is_success() {
        bail!("facebook /me/accounts failed: HTTP {}: {}", status, text);
    }

    let v: AccountsResponse = serde_json::from_str(&text).context("failed to parse /me/accounts")?;
    Ok(v.data)
}

fn select_page(wanted_id: Option<&str>, pages: &[PageInfo]) -> Option<PageInfo> {
    if let Some(id) = wanted_id {
        if let Some(p) = pages.iter().find(|p| p.id == id) {
            return Some(p.clone());
        }
    }

    pages.first().cloned()
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
