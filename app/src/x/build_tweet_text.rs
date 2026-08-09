pub(super) fn build_tweet_text(title: &str, url: &str) -> String {
    let mut t = title.trim().replace('\n', " ");
    while t.contains("  ") {
        t = t.replace("  ", " ");
    }

    if t.chars().count() > 220 {
        t = t.chars().take(217).collect::<String>();
        t.push_str("...");
    }

    format!("{}\n{}", t, url)
}
