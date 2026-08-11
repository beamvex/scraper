use anyhow::{Context, Result};
use pulldown_cmark::{html, Parser};
use regex::Regex;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn main() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    let md_path = args.first().context("usage: review2html <input.review.md> [output.html]")?;
    let out_path = args.get(1).cloned().unwrap_or_else(|| {
        PathBuf::from(md_path).with_extension("html").to_string_lossy().into_owned()
    });
    let md = fs::read_to_string(md_path).with_context(|| format!("failed to read {}", md_path))?;
    let md_path = PathBuf::from(md_path);
    let images = collect_images(&md_path);
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    let template_path = manifest_dir.join("product_review_template.html");
    let template = fs::read_to_string(&template_path)
        .with_context(|| format!("failed to read template {}", template_path.display()))?;
    let html = render(&md, &images, &template)?;
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

fn wrap_h2_in_cards(html: &str) -> String {
    let h2_re = Regex::new(r"(?s)<h2[^>]*>.*?</h2>").unwrap();
    let mut matches = h2_re.find_iter(html).peekable();
    let mut out = String::new();
    let mut last = 0;
    while let Some(m) = matches.next() {
        out.push_str(&html[last..m.start()]);
        let h2 = m.as_str().replace("<h2>", "<h2 class=\"card-title\">");
        let end = if let Some(next) = matches.peek() {
            next.start()
        } else {
            html.len()
        };
        let body = &html[m.end()..end];
        out.push_str("<div class=\"card my-3\"><div class=\"card-body\">");
        out.push_str(&h2);
        out.push_str(body);
        out.push_str("</div></div>");
        last = end;
    }
    out.push_str(&html[last..]);
    out
}

fn render(md: &str, images: &[String], template: &str) -> Result<String> {
    let title = md
        .lines()
        .find(|l| l.starts_with("# "))
        .map(|l| l.trim_start_matches("# ").trim().to_string())
        .unwrap_or_default();
    let short = first_paragraph(md).unwrap_or_default();
    let price = extract_price(md).unwrap_or_default();
    let image_url = images.first().map(|n| format!("images/{}", n)).unwrap_or_default();
    let gallery = gallery_html(images);
    let year = current_year().to_string();

    // Drop the first H1 (title) from the body so it doesn't appear twice in the page.
    let mut lines = md.lines().skip_while(|l| l.trim().is_empty()).peekable();
    if let Some(first) = lines.peek() {
        if first.starts_with("# ") {
            lines.next();
        }
    }
    let body_md = lines.collect::<Vec<_>>().join("\n");

    let mut content = String::new();
    let parser = Parser::new(&body_md);
    html::push_html(&mut content, parser);
    content = wrap_h2_in_cards(&content);

    let mut out = template.to_string();
    replace_all(&mut out, "{{TITLE}}", &title);
    replace_all(&mut out, "{{SHORT_DESCRIPTION}}", &short);
    replace_all(&mut out, "{{PRICE}}", &price);
    replace_all(&mut out, "{{IMAGE_URL}}", &image_url);
    replace_all(&mut out, "{{BUY_URL}}", "");
    replace_all(&mut out, "{{CONTENT}}", &content);
    replace_all(&mut out, "{{GALLERY}}", &gallery);
    replace_all(&mut out, "{{YEAR}}", &year);
    Ok(out)
}

fn replace_all(s: &mut String, placeholder: &str, value: &str) {
    *s = s.replace(placeholder, value);
}

fn first_paragraph(md: &str) -> Option<String> {
    for line in md.lines().skip_while(|l| l.trim().is_empty()) {
        if line.starts_with("# ") || line.starts_with("##") {
            continue;
        }
        if line.starts_with("!") || line.starts_with(">") || line.starts_with("-") || line.starts_with("*") {
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }
        let mut out = String::new();
        let parser = Parser::new(line);
        html::push_html(&mut out, parser);
        return Some(out.trim_start_matches("<p>").trim_end_matches("</p>").trim().to_string());
    }
    None
}

fn extract_price(md: &str) -> Option<String> {
    let re = Regex::new(r"(?:GBP|\$|£|€)\d+(?:\.\d+)?").unwrap();
    re.find(md).map(|m| m.as_str().to_string())
}

fn gallery_html(images: &[String]) -> String {
    if images.len() <= 1 {
        return String::new();
    }
    images
        .iter()
        .skip(1)
        .map(|n| format!("<img src=\"images/{}\" class=\"img-fluid rounded m-2\" style=\"max-width:300px\" alt=\"product image\">", n))
        .collect::<Vec<_>>()
        .join("\n")
}

fn current_year() -> i64 {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as f64;
    (1970.0 + seconds / 31_557_600.0) as i64
}
