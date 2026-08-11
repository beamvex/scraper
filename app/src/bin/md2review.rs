use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_CHARS: usize = 80_000;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let args: Vec<String> = env::args().skip(1).collect();
    let md_path = args.first().context("usage: md2review <input.md> [output.md]")?;
    let out_path = args.get(1).cloned().unwrap_or_else(|| {
        PathBuf::from(md_path).with_extension("review.md").to_string_lossy().into_owned()
    });
    let md = fs::read_to_string(md_path).with_context(|| format!("failed to read {}", md_path))?;
    let md_path = PathBuf::from(md_path);
    let images = collect_images(&md_path);
    let prompt = build_prompt(&md, &images);
    let api_key = std::env::var("OPENAI_API_KEY").context("OPENAI_API_KEY not set")?;
    let review = generate_md(&api_key, &prompt).await?;
    fs::write(&out_path, review).with_context(|| format!("failed to write {}", out_path))?;
    println!("wrote {}", out_path);
    Ok(())
}

fn collect_images(md_path: &Path) -> Vec<String> {
    let img_dir = md_path.parent().unwrap_or(Path::new(".")).join("images");
    let mut names: Vec<String> = match fs::read_dir(&img_dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect(),
        Err(_) => Vec::new(),
    };
    names.sort();
    names
}

fn build_prompt(md: &str, images: &[String]) -> String {
    let md = if md.len() > MAX_CHARS { &md[..MAX_CHARS] } else { md };
    let image_list = images
        .iter()
        .map(|i| format!("- images/{}", i))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "You are an expert product review and SEO writer.\n\n\
        Write a comprehensive product review in Markdown based on the Markdown product description below.\n\
        - Think of 3-5 problems this product solves and cover them in the review.\n\
        - Come up with a long-tail keyword a customer would search for to find this product, and target it naturally throughout the review.\n\
        - Aim for a 20-minute reading time. Expand the description, features, and analysis to be thorough and useful.\n\
        - Include every positive customer review (4 or 5 stars) from the Markdown. Do not summarize, combine, or omit any. Each review should appear as a Markdown quote with its original title and full original body text.\n\
        - Use the product images listed.\n\n\
        Product images:\n{0}\n\n\
        Markdown product description:\n{1}\n\n\
        - Return the complete Markdown review inside a JSON object with a single `md` string key. Do not output HTML or code fences.",
        image_list, md
    )
}

async fn generate_md(api_key: &str, prompt: &str) -> Result<String> {
    let body = json!({
        "model": "gpt-4o-mini",
        "messages": [{"role": "user", "content": prompt}],
        "temperature": 0.7,
        "response_format": {"type": "json_object"}
    });
    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .context("failed to call OpenAI")?;
    let status = resp.status();
    let v: Value = resp.json().await.context("failed to parse OpenAI JSON")?;
    if !status.is_success() {
        bail!("OpenAI request failed ({}): {}", status, v);
    }
    let content = v
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .context("OpenAI response missing content")?;
    let parsed: Value =
        serde_json::from_str(content).context("OpenAI content is not valid JSON")?;
    parsed
        .get("md")
        .and_then(|h| h.as_str())
        .map(String::from)
        .context("OpenAI JSON missing md field")
}

