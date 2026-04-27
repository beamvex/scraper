use std::string::String;

pub fn sanitize_path_component(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        let keep = c.is_ascii_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.');
        if keep {
            out.push(c);
        } else {
            out.push('_');
        }
    }
    let out = out.trim().trim_matches('.').to_string();
    let out = out.split_whitespace().collect::<Vec<_>>().join(" ");

    if out.is_empty() {
        "unknown-product".to_string()
    } else {
        out.chars().take(120).collect()
    }
}

pub fn html_to_text_for_medium(html: &str) -> String {
    let mut out = String::with_capacity(html.len().min(64 * 1024));
    let mut in_tag = false;
    let mut tag_buf = String::new();

    let mut chars = html.chars().peekable();
    while let Some(c) = chars.next() {
        if in_tag {
            if c == '>' {
                in_tag = false;
                let tag = tag_buf.trim().to_ascii_lowercase();
                if tag.starts_with("br")
                    || tag.starts_with("/p")
                    || tag.starts_with("p")
                    || tag.starts_with("/h1")
                    || tag.starts_with("/h2")
                    || tag.starts_with("h2")
                    || tag.starts_with("li")
                    || tag.starts_with("/li")
                {
                    out.push('\n');
                }
                tag_buf.clear();
            } else {
                tag_buf.push(c);
            }
            continue;
        }

        if c == '<' {
            in_tag = true;
            tag_buf.clear();
            continue;
        }

        match c {
            '&' => {
                let mut ent = String::new();
                while let Some(nc) = chars.peek().copied() {
                    chars.next();
                    if nc == ';' {
                        break;
                    }
                    if ent.len() > 16 {
                        break;
                    }
                    ent.push(nc);
                }
                match ent.as_str() {
                    "amp" => out.push('&'),
                    "lt" => out.push('<'),
                    "gt" => out.push('>'),
                    "quot" => out.push('"'),
                    "apos" => out.push('\''),
                    "nbsp" => out.push(' '),
                    _ => {}
                }
            }
            _ => out.push(c),
        }
    }

    let mut cleaned = String::with_capacity(out.len());
    let mut last_was_nl = false;
    for line in out.lines() {
        let line = line.trim();
        if line.is_empty() {
            if !last_was_nl {
                cleaned.push('\n');
                last_was_nl = true;
            }
            continue;
        }
        if !cleaned.is_empty() {
            cleaned.push('\n');
        }
        cleaned.push_str(line);
        last_was_nl = false;
    }

    cleaned.trim().to_string()
}
