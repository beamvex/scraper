use anyhow::Result;
use chromiumoxide::cdp::browser_protocol::dom::SetFileInputFilesParams;
use chromiumoxide::page::Page;
use std::path::Path;
use std::time::Duration;
use tracing::warn;

pub(super) async fn try_set_file_input(page: &Page, image_path: &Path) -> Result<bool> {
    let selectors = ["input[type='file']", "input[type=\"file\"]"];
    for _ in 0..40 {
        for sel in selectors {
            if let Ok(el) = page.find_element(sel).await {
                let abs = image_path.canonicalize().unwrap_or_else(|_| image_path.to_path_buf());
                let cmd = SetFileInputFilesParams::builder()
                    .files(vec![abs.display().to_string()])
                    .backend_node_id(el.backend_node_id)
                    .build()
                    .map_err(|e| anyhow::anyhow!(e))?;
                if let Err(err) = page.execute(cmd).await {
                    warn!(selector = sel, error = %err, "failed to set pinterest file input files");
                    continue;
                }
                tokio::time::sleep(Duration::from_millis(4500)).await;
                return Ok(true);
            }
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    Ok(false)
}
