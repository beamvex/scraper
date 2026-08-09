const AMAZON_BASE: &str = "https://www.amazon.com";

pub fn resolve_amazon_url(href: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") {
        href.to_string()
    } else if href.starts_with('/') {
        format!("{}{}", AMAZON_BASE, href)
    } else {
        format!("{}/{}", AMAZON_BASE, href)
    }
}
