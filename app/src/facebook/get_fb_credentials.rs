use std::env;

pub(super) fn get_fb_credentials() -> Option<(String, String)> {
    let page_id = env::var("FB_PAGE_ID").ok().filter(|v| !v.trim().is_empty())?;
    let access_token = env::var("FB_PAGE_ACCESS_TOKEN").ok().filter(|v| !v.trim().is_empty())?;
    Some((page_id, access_token))
}
