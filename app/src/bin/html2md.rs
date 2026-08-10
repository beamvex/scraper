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

fn extract_product_text(html: &str) -> Option<String> {
    let doc = Html::parse_document(html);
    let title = doc
        .select(&Selector::parse("#productTitle").ok()?)
        .next()
        .map(|e| e.text().collect::<String>().trim().to_string())
        .unwrap_or_default();

    let price = doc
        .select(&Selector::parse(".a-price .a-offscreen, #priceblock_ourprice, #priceblock_dealprice, .a-price-whole").ok()?)
        .filter_map(|e| {
            let t = e.text().collect::<String>().trim().to_string();
            if t.starts_with('$') || t.starts_with('£') || t.starts_with('€') || t.starts_with("GBP") {
                Some(t)
            } else {
                None
            }
        })
        .next()
        .unwrap_or_default();

    let bullets: Vec<String> = doc
        .select(&Selector::parse("#feature-bullets li").ok()?)
        .map(|e| e.text().collect::<String>().trim().to_string())
        .filter(|s| !s.is_empty() && !s.to_lowercase().contains("see more") && !s.to_lowercase().contains("make it"))
        .collect();

    let desc: Vec<String> = doc
        .select(&Selector::parse("#productDescription p, #aplus p").ok()?)
        .map(|e| e.text().collect::<String>().trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let mut reviews = Vec::new();
    let review_sel = Selector::parse("div[data-hook='review']").ok()?;
    let star_sel = Selector::parse("[data-hook='review-star-rating'] .a-icon-alt, [data-hook='review-star-rating']").ok()?;
    let title_sel = Selector::parse("[data-hook='reviewTitle'] span, [data-hook='reviewTitle']").ok()?;
    let body_sel = Selector::parse("[data-hook='reviewText'] span, [data-hook='reviewText']").ok()?;

    for review in doc.select(&review_sel) {
        let star = review.select(&star_sel)
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or_default();
        let title = review.select(&title_sel)
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or_default();
        let body = review.select(&body_sel)
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or_default();
        if body.is_empty() {
            continue;
        }
        let body = body
            .replace("Brief content visible, double tap to read full content.", "")
            .replace("Full content visible, double tap to read brief content.", "")
            .replace("Read more", "")
            .replace("Read less", "")
            .replace("  ", " ")
            .trim()
            .to_string();
        if body.is_empty() {
            continue;
        }
        let star_num = star
            .split_whitespace()
            .next()
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0);
        if star_num >= 4 {
            reviews.push(format!("**{}** | {}: {}", star, title, truncate_review(&body, 280)));
        }
    }

fn truncate_review(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut s = text.chars().take(max).collect::<String>();
    for (i, c) in s.char_indices().rev() {
        if c == '.' || c == '!' || c == '?' {
            s.truncate(i + 1);
            return s;
        }
    }
    format!("{}...", s.trim_end())
}

    let mut out = format!("# {}\n\n", title);
    if !price.is_empty() {
        out.push_str(&format!("**Price:** {}\n\n", price));
    }
    if !desc.is_empty() {
        out.push_str(&desc.join("\n\n"));
        out.push_str("\n\n");
    }
    if !bullets.is_empty() {
        out.push_str("## Features\n\n");
        for b in &bullets {
            out.push_str(&format!("- {}\n", b));
        }
        out.push_str("\n");
    }
    if !reviews.is_empty() {
        out.push_str("## Customer Reviews\n\n");
        for r in reviews {
            out.push_str(&format!("- {}\n", r));
        }
        out.push_str("\n");
    }
    if title.is_empty() && desc.is_empty() && bullets.is_empty() {
        None
    } else {
        Some(out)
    }
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
    let text = extract_product_text(&html).unwrap_or_else(|| {
        let html = clean_html(&html);
        html2text::from_read(html.as_bytes(), 80).unwrap_or_default()
    });
    let text = format!("{}\n\n{}", clean_md(&text), images);
    fs::write(&md_path, text).with_context(|| format!("failed to write {}", md_path.display()))?;
    println!("wrote {}", md_path.display());
    Ok(())
}
