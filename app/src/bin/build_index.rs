use anyhow::{Context, Result};
use regex::Regex;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() -> Result<()> {
    let data_dir = env::args().nth(1).unwrap_or_else(|| "../data".to_string());
    let data_dir = PathBuf::from(data_dir);
    let mut links = Vec::new();
    find_index_files(&data_dir, &data_dir, &mut links)?;
    links.sort_by(|a, b| a.1.cmp(&b.1));
    let html = build_index_html(&links);
    let out = data_dir.join("index.html");
    fs::write(&out, html).with_context(|| format!("failed to write {}", out.display()))?;
    println!("wrote {}", out.display());
    Ok(())
}

fn find_index_files(base: &Path, dir: &Path, links: &mut Vec<(String, String)>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            find_index_files(base, &path, links)?;
        } else if path.file_name().and_then(|n| n.to_str()) == Some("index.html") {
            let rel = path.strip_prefix(base).unwrap_or(&path);
            if rel == Path::new("index.html") {
                continue;
            }
            let title = extract_title(&path).unwrap_or_else(|| rel.to_string_lossy().into_owned());
            links.push((rel.to_string_lossy().into_owned(), title));
        }
    }
    Ok(())
}

fn extract_title(path: &Path) -> Option<String> {
    let html = fs::read_to_string(path).ok()?;
    let re = Regex::new(r"(?si)<title\b[^>]*>(.*?)</title>").unwrap();
    re.captures(&html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string())
}

fn build_index_html(links: &[(String, String)]) -> String {
    let items = links
        .iter()
        .map(|(href, text)| {
            let encoded = href
                .split('/')
                .map(|s| urlencoding::encode(s).into_owned())
                .collect::<Vec<_>>()
                .join("/");
            format!(
                "<li><a href=\"{}\">{}</a></li>",
                encoded,
                html_escape(text)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<!-- Google tag (gtag.js) -->\n<script async src=\"https://www.googletagmanager.com/gtag/js?id=G-QQMJVW98SP\"></script>\n<script>\n  window.dataLayer = window.dataLayer || [];\n  function gtag(){{dataLayer.push(arguments);}}\n  gtag('js', new Date());\n\n  gtag('config', 'G-QQMJVW98SP');\n</script>\n<meta charset=\"UTF-8\">\n<title>Product Index</title>\n</head>\n<body>\n<h1>Product Index</h1>\n<ul>\n{}\n</ul>\n</body>\n</html>",
        items
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\"', "&quot;")
}
