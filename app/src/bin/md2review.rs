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
    let html = ensure_image_references(html, &images);
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
        3. A product review article optimized for those long-tail keywords, written as a complete, valid HTML document (return as a string under key `html`)\n\n\
        Requirements for `html`:\n\
        - Use all of the product images listed below. Reference them with `src=\"images/FILENAME\"` (e.g., `src=\"images/image_001.jpg\"`).\n\
        - Include the problems and keywords naturally in the article.\n\
        - Use semantic HTML: `<html>`, `<head>`, `<body>`, `<article>`, `<h1>`, `<h2>`, `<p>`, `<ul>`, `<img>`.\n\
        - Do not output markdown or code fences.\n\n\
        Product images:\n{}\n\n\
        Markdown product description:\n{}",
        image_list, md
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

fn ensure_image_references(html: String, images: &[String]) -> String {
    if images.is_empty() || html.contains("<img") {
        return html;
    }
    let gallery = images
        .iter()
        .map(|i| format!("<figure><img src=\"images/{}\" alt=\"product image\" /></figure>", i))
        .collect::<Vec<_>>()
        .join("\n");
    html.replace("</body>", &format!("{}\n</body>", gallery))
}
