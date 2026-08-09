use anyhow::{Context, Result};
use regex::Regex;
use scraper::{Html, Selector};
use serde_json::Value;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn clean_html(html: &str) -> String {
    let re = Regex::new(r"(?is)<(?:script|style|nav|header|footer|link|aside)\b[^>]*>.*?</(?:script|style|nav|header|footer|link|aside)>").unwrap();
    re.replace_all(html, "").into_owned()
}

fn clean_md(text: &str) -> String {
    let re = Regex::new(r"(?m)^\s*\[\d+\]:\s*\S+.*$").unwrap();
    re.replace_all(text, "").into_owned()
}

fn color_image_urls(html: &str) -> Option<Vec<String>> {
    let start = html.find("'initial':")?;
    let arr_start = html[start..].find('[')? + start;
    let mut depth = 0;
    let mut in_str = false;
    let mut esc = false;
    for (i, c) in html[arr_start..].char_indices() {
        match c {
            '\\' if in_str => esc = true,
            '"' if !esc => in_str = !in_str,
            '[' if !in_str => depth += 1,
            ']' if !in_str => {
                depth -= 1;
                if depth == 0 {
                    let arr = &html[arr_start..arr_start + i + c.len_utf8()];
                    let vals: Vec<Value> = serde_json::from_str(arr).ok()?;
                    return Some(vals.into_iter().filter_map(|v| v.get("hiRes").and_then(|u| u.as_str()).map(String::from)).collect());
                }
            }
            _ => esc = false,
        }
    }
    None
}

fn product_image_urls(html: &str) -> Vec<String> {
    if let Some(urls) = color_image_urls(html) { return urls; }
    let doc = Html::parse_document(html);
    let sel = Selector::parse("img").unwrap();
    let mut seen = std::collections::HashSet::new();
    doc.select(&sel)
        .filter_map(|e| {
            let src = e.value().attr("data-old-hires")
                .or(e.value().attr("data-src"))
                .or(e.value().attr("src"))?
                .to_string();
            if !src.contains("m.media-amazon.com/images/I/") { return None; }
            if !seen.insert(src.clone()) { return None; }
            Some(src)
        })
        .collect()
}

fn download_images(dir: &Path, urls: &[String]) -> Result<Vec<String>> {
    fs::create_dir_all(dir)?;
    let client = reqwest::blocking::Client::new();
    let mut names = Vec::new();
    for (i, url) in urls.iter().enumerate() {
        let name = format!("image_{:03}.jpg", i + 1);
        let path = dir.join(&name);
        let bytes = client.get(url).send().context("failed to fetch image")?.bytes().context("failed to read image bytes")?;
        fs::write(&path, bytes).with_context(|| format!("failed to write {}", path.display()))?;
        names.push(name);
    }
    Ok(names)
}

fn image_block(names: &[String]) -> String {
    format!("## Product Images\n\n{}", names.iter().map(|n| format!("![product image](images/{})", n)).collect::<Vec<_>>().join("\n"))
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    let html_path = args.first().context("usage: html2md <input.html> [output.md]")?;
    let md_path = args.get(1).cloned().unwrap_or_else(|| {
        PathBuf::from(html_path).with_extension("md").to_string_lossy().into_owned()
    });
    let html = fs::read_to_string(html_path).with_context(|| format!("failed to read {}", html_path))?;
    let md_path = PathBuf::from(md_path);
    let img_dir = md_path.parent().unwrap_or(Path::new(".")).join("images");
    let urls = product_image_urls(&html);
    let images = if urls.is_empty() { String::new() } else { image_block(&download_images(&img_dir, &urls)?) };
    let html = clean_html(&html);
    let text = html2text::from_read(html.as_bytes(), 80).context("html2text conversion failed")?;
    let text = format!("{}\n\n{}", clean_md(&text), images);
    fs::write(&md_path, text).with_context(|| format!("failed to write {}", md_path.display()))?;
    println!("wrote {}", md_path.display());
    Ok(())
}
