use anyhow::Result;

pub async fn generate_review_article_html(
    api_key: &str,
    product_title: &str,
    product_url: Option<&str>,
    product_page_html: &str,
) -> Result<String> {
    let prompt = super::build_prompt::build_prompt(product_title, product_url, product_page_html);
    let response = super::call_openai::call_openai(api_key, prompt).await?;
    super::parse_openai_response::parse_openai_response(response)
}
