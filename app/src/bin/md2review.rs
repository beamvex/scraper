use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_CHARS: usize = 80_000;
const BOOTSTRAP_CSS: &str = r#"<link href="https://cdn.jsdelivr.net/npm/bootstrap@5.3.8/dist/css/bootstrap.min.css" rel="stylesheet" integrity="sha384-sRIl4kxILFvY47J16cr9ZwB07vP4J8+LH7qKQnuqkuIAvNWLzeN8tE5YBujZqJLB" crossorigin="anonymous">"#;
const BOOTSTRAP_JS: &str = r#"<script src="https://cdn.jsdelivr.net/npm/bootstrap@5.3.8/dist/js/bootstrap.bundle.min.js" integrity="sha384-FKyoEForCGlyvwx9Hj09JcYn3nv7wiPVlz7YYwJrWVcXK/BmnVDxM+D2scQbITxI" crossorigin="anonymous"></script>"#;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let args: Vec<String> = env::args().skip(1).collect();
    let md_path = args.first().context("usage: md2review <input.md> [output.html]")?;
    let out_path = args.get(1).cloned().unwrap_or_else(|| {
        PathBuf::from(md_path).with_extension("html").to_string_lossy().into_owned()
    });
    let md = fs::read_to_string(md_path).with_context(|| format!("failed to read {}", md_path))?;
    let md_path = PathBuf::from(md_path);
    let images = collect_images(&md_path);
    let prompt = build_prompt(&md, &images);
    let api_key = std::env::var("OPENAI_API_KEY").context("OPENAI_API_KEY not set")?;
    let html = generate_html(&api_key, &prompt).await?;
    let html = polish_html(html, &images);
    fs::write(&out_path, html).with_context(|| format!("failed to write {}", out_path))?;
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
        "You are an SEO and consumer-product review expert.\n\n\
        Given the following Markdown product description, identify:\n\n\
        1. Problems this product solves (return as a JSON array of strings under key `problems_solved`)\n\
        2. Long-tail keywords people would search for to find this product (return as a JSON array of strings under key `long_tail_keywords`)\n\
        3. A detailed product review article optimized for those long-tail keywords, written as a complete, valid HTML document (return as a string under key `html`)\n\n\
        Requirements for `html`:\n\
        - Start with `<!DOCTYPE html>` and a full `<html>` document with `<head>` and `<body>`.\n\
        - In the `<head>`, include exactly this Bootstrap CSS link: {2}\n\
        - Just before the closing `</body>`, include exactly this Bootstrap JS script: {3}\n\
        - Write a detailed, in-depth review: at least 8 sections with `<h2>` headings; each section should have 4-6 paragraphs; each paragraph should have 4-6 sentences.\n\
        - Include a dedicated 'Customer Reviews' or 'What Buyers Are Saying' section near the bottom with 4-5 positive review comments. Each comment should include a reviewer name and a short quote.\n\
        - Use all of the product images listed below. Reference them with `src=\"images/FILENAME\"` (e.g., `src=\"images/image_001.jpg\"`).\n\
        - Include the problems and keywords naturally throughout the article.\n\
        - Use semantic HTML: `<article>`, `<h1>`, `<h2>`, `<p>`, `<ul>`, `<img>`, `<figure>`, `<blockquote>`.\n\
        - Do not output markdown or code fences.\n\n\
        Product images:\n{0}\n\n\
        Markdown product description:\n{1}",
        image_list, md, BOOTSTRAP_CSS, BOOTSTRAP_JS
    )
}

async fn generate_html(api_key: &str, prompt: &str) -> Result<String> {
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
        .get("html")
        .and_then(|h| h.as_str())
        .map(String::from)
        .context("OpenAI JSON missing html field")
}

fn polish_html(mut html: String, images: &[String]) -> String {
    if !html.contains("bootstrap.min.css") {
        if let Some(idx) = html.find("</head>") {
            html.insert_str(idx, &format!("\n{}\n", BOOTSTRAP_CSS));
        } else if let Some(idx) = html.find("<head>") {
            html.insert_str(idx + "<head>".len(), &format!("\n{}\n", BOOTSTRAP_CSS));
        }
    }
    if !html.contains("bootstrap.bundle.min.js") {
        if let Some(idx) = html.find("</body>") {
            html.insert_str(idx, &format!("\n{}\n", BOOTSTRAP_JS));
        }
    }
    if !html.contains("<img") && !images.is_empty() {
        let gallery = images
            .iter()
            .map(|i| format!("<figure><img src=\"images/{}\" alt=\"product image\" /></figure>", i))
            .collect::<Vec<_>>()
            .join("\n");
        if let Some(idx) = html.find("</body>") {
            html.insert_str(idx, &format!("\n{}\n", gallery));
        }
    }
    html
}
