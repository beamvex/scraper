use anyhow::Result;
use chromiumoxide::page::Page;

const IMAGE_JS: &str = r#"(() => {
  const landing = document.querySelector('#landingImage');
  if (!landing) return null;
  const dyn = landing.getAttribute('data-a-dynamic-image');
  if (!dyn) return landing.src || null;
  try {
    const obj = JSON.parse(dyn);
    let bestUrl = null;
    let bestScore = -1;
    for (const [url, dims] of Object.entries(obj)) {
      if (!url || typeof url !== 'string') continue;
      if (!Array.isArray(dims) || dims.length < 2) continue;
      const w = Number(dims[0]) || 0;
      const h = Number(dims[1]) || 0;
      const score = w * h;
      if (score > bestScore) { bestScore = score; bestUrl = url; }
    }
    return bestUrl || landing.src || null;
  } catch (e) { return landing.src || null; }
})()"#;

pub(super) async fn get_image_url(page: &Page) -> Result<Option<String>> {
    page.evaluate(IMAGE_JS).await?.into_value::<Option<String>>().map_err(|e| anyhow::anyhow!(e))
}
