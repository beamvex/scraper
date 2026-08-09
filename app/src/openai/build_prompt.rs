pub(super) fn build_prompt(product_title: &str, product_url: Option<&str>, product_page_html: &str) -> String {
    const MAX_CHARS: usize = 120_000;
    let html = if product_page_html.len() > MAX_CHARS { &product_page_html[..MAX_CHARS] } else { product_page_html };
    let url_line = product_url.map(|u| format!("Product URL: {}\n", u)).unwrap_or_default();
    format!(
        "You are an expert consumer tech reviewer. Write a review article that is about a 4-minute read.\n\n\
Output requirements:\n\
- Output valid HTML only (no Markdown).\n\
- Use <article> with a single <h1>, then sections with <h2>.\n\
- The first paragraph must NOT repeat the title verbatim or start with the product name; write a short hook/lead instead.\n\
- Include: overview, key features, who it's for, pros/cons lists, pricing/value discussion, and verdict.\n\
- Do not mention that you were given raw HTML; infer details from the provided page.\n\
- Write at least 5 sections with h2 headers\n\
- each section should have at least 3 paragraphs \n\
- each paragraph should be 3-5 sentences\n\
- If specs are unclear, state assumptions cautiously.\n\nTitle: {}\n{}\nProduct detail page HTML (truncated if needed):\n{}",
        product_title, url_line, html
    )
}
