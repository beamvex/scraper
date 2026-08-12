use anyhow::{bail, Context, Result};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let data_dir = env::args().nth(1).unwrap_or_else(|| "../data".to_string());
    let account_id = env::var("CF_ACCOUNT_ID").context("CF_ACCOUNT_ID not set")?;
    let api_token = env::var("CF_API_TOKEN").context("CF_API_TOKEN not set")?;
    let project = env::var("CF_PAGE").context("CF_PAGE not set")?;
    let branch = env::var("CF_BRANCH").unwrap_or_else(|_| "main".to_string());

    let use_wrangler = wrangler_available();
    let mut cmd = std::process::Command::new(if use_wrangler { "wrangler" } else { "npx" });
    if !use_wrangler {
        cmd.arg("-y").arg("wrangler");
    }
    cmd.arg("pages")
        .arg("deploy")
        .arg(&data_dir)
        .arg("--project-name")
        .arg(&project)
        .arg("--branch")
        .arg(&branch)
        .arg("--no-bundle");
    cmd.env("CLOUDFLARE_ACCOUNT_ID", &account_id)
        .env("CLOUDFLARE_API_TOKEN", &api_token)
        .env("CF_ACCOUNT_ID", &account_id)
        .env("CF_API_TOKEN", &api_token)
        .env("CF_PAGE", &project)
        .env("CF_BRANCH", &branch);
    cmd.stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());

    let status = cmd
        .status()
        .context("failed to run wrangler pages deploy")?;
    if !status.success() {
        bail!("wrangler pages deploy failed with status {}", status);
    }
    Ok(())
}

fn wrangler_available() -> bool {
    std::process::Command::new("wrangler")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

