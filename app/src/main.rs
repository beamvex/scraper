mod computer_queries;
#[allow(dead_code)]
mod facebook;
mod ifttt;
mod openai;
mod pinterest;
mod run;
mod util;
mod wordpress_com;
mod x;

use anyhow::Result;

pub(crate) const DEBUG: bool = false;

#[tokio::main]
async fn main() -> Result<()> {
    if let Ok(home) = std::env::var("HOME") {
        let _ = dotenvy::from_filename(format!("{}/.scraper.env", home));
    }
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let browser = run::connect_browser().await?;
    run::run_once(&browser).await
}
