use anyhow::{Context, Result};
use regex::Regex;
use std::env;
use std::fs;
use std::path::PathBuf;

fn clean_html(html: &str) -> String {
    let re = Regex::new(r"(?is)<(?:script|style|nav|header|footer|link|aside)\b[^>]*>.*?</(?:script|style|nav|header|footer|link|aside)>").unwrap();
    re.replace_all(html, "").into_owned()
}

fn clean_md(text: &str) -> String {
    let re = Regex::new(r"(?m)^\s*\[\d+\]:\s*\S+.*$").unwrap();
    re.replace_all(text, "").into_owned()
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    let html_path = args.first().context("usage: html2md <input.html> [output.md]")?;
    let md_path = args.get(1).cloned().unwrap_or_else(|| {
        PathBuf::from(html_path).with_extension("md").to_string_lossy().into_owned()
    });
    let html = fs::read_to_string(html_path).with_context(|| format!("failed to read {}", html_path))?;
    let html = clean_html(&html);
    let text = html2text::from_read(html.as_bytes(), 80).context("html2text conversion failed")?;
    let text = clean_md(&text);
    fs::write(&md_path, text).with_context(|| format!("failed to write {}", md_path))?;
    println!("wrote {}", md_path);
    Ok(())
}
