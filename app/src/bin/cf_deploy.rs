use anyhow::{bail, Context, Result};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let data_dir = env::args().nth(1).unwrap_or_else(|| "../data".to_string());
    let data_dir = PathBuf::from(data_dir);
    let account_id = env::var("CF_ACCOUNT_ID").context("CF_ACCOUNT_ID not set")?;
    let api_token = env::var("CF_API_TOKEN").context("CF_API_TOKEN not set")?;
    let project = env::var("CF_PAGE").context("CF_PAGE not set")?;
    let branch = env::var("CF_BRANCH").unwrap_or_else(|_| "main".to_string());

    let files = collect_files(&data_dir)?;
    let manifest = build_manifest(&files);
    let archive = build_tar(&data_dir, &files)?;
    let url = format!(
        "https://api.cloudflare.com/client/v4/accounts/{}/pages/projects/{}/deployments",
        account_id, project
    );
    let manifest_part = reqwest::multipart::Part::text(manifest)
        .mime_str("application/json")?;
    let file_part = reqwest::multipart::Part::bytes(archive)
        .file_name("site.tar")
        .mime_str("application/x-tar")?;
    let form = reqwest::multipart::Form::new()
        .text("branch", branch)
        .part("manifest", manifest_part)
        .part("file", file_part);

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_token))
        .multipart(form)
        .send()
        .await
        .context("failed to call Cloudflare Pages API")?;
    let status = resp.status();
    let body = resp.text().await.context("failed to read response body")?;
    if !status.is_success() {
        bail!("Cloudflare upload failed ({}): {}", status, body);
    }
    println!("{}", body);
    Ok(())
}

fn collect_files(dir: &Path) -> Result<Vec<(String, Vec<u8>)>> {
    let mut files = Vec::new();
    walk(dir, dir, &mut files)?;
    Ok(files)
}

fn walk(base: &Path, dir: &Path, files: &mut Vec<(String, Vec<u8>)>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk(base, &path, files)?;
        } else if path.is_file() {
            let rel = path.strip_prefix(base).unwrap_or(&path);
            let rel = rel.to_string_lossy().into_owned();
            let data = fs::read(&path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            files.push((rel, data));
        }
    }
    Ok(())
}

fn build_manifest(files: &[(String, Vec<u8>)]) -> String {
    let mut map = Map::new();
    for (path, data) in files {
        map.insert(path.clone(), Value::String(sha256_hex(data)));
    }
    Value::Object(map).to_string()
}

fn build_tar(dir: &Path, files: &[(String, Vec<u8>)]) -> Result<Vec<u8>> {
    let mut buf: Vec<u8> = Vec::new();
    {
        let mut builder = tar::Builder::new(&mut buf);
        for (rel, _) in files {
            let full = dir.join(rel);
            builder
                .append_path_with_name(&full, rel)
                .with_context(|| format!("failed to tar {}", full.display()))?;
        }
        builder.finish()?;
    }
    Ok(buf)
}

fn sha256_hex(data: &[u8]) -> String {
    let hash = Sha256::digest(data);
    hash.iter().map(|b| format!("{:02x}", b)).collect::<String>()
}
