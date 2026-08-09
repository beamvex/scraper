use std::env;

pub(super) fn build_fb_message(title: &str, url: Option<&str>) -> String {
    match env::var("FB_POST_TEMPLATE") {
        Ok(t) if !t.trim().is_empty() => t
            .replace("{title}", title)
            .replace("{url}", url.unwrap_or("")),
        _ => {
            let u = url.unwrap_or("");
            if u.is_empty() { title.to_string() } else { format!("{}\n{}", title, u) }
        }
    }
}
