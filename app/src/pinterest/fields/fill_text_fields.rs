use anyhow::Result;
use chromiumoxide::page::Page;

pub async fn fill_text_fields(
    page: &Page,
    title: &str,
    description: Option<&str>,
    link: &str,
) -> Result<()> {
    let desc = super::truncate_description::truncate_description(description);
    let title_js = super::make_title_js::make_title_js(title);
    let desc_js = desc.as_deref().map(super::make_description_js::make_description_js);
    let link_js = super::make_link_js::make_link_js(link);
    let (t_ok, d_ok, l_ok) = super::set_all_fields::set_all_fields(
        page, &title_js, desc_js.as_deref(), &link_js,
    ).await?;
    super::write_fields_debug::write_fields_debug(page, t_ok, d_ok, l_ok, desc_js.is_some()).await;
    Ok(())
}
