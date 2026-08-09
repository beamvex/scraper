use anyhow::Result;
use chromiumoxide::page::Page;
use std::time::Duration;

pub(super) async fn set_all_fields(
    page: &Page,
    title_js: &str,
    description_js: Option<&str>,
    link_js: &str,
) -> Result<(bool, bool, bool)> {
    let (mut t, mut d, mut l) = (false, description_js.is_none(), false);
    for _ in 0..20 {
        if !t { t = page.evaluate(title_js).await?.into_value::<bool>().unwrap_or(false); }
        if !d {
            if let Some(djs) = description_js {
                d = page.evaluate(djs).await?.into_value::<bool>().unwrap_or(false);
            }
        }
        if !l { l = page.evaluate(link_js).await?.into_value::<bool>().unwrap_or(false); }
        if t && d && l { break; }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    Ok((t, d, l))
}
