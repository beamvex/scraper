use super::maybe_post_pin_to_board;
use chromiumoxide::browser::Browser;
use std::path::Path;
use tracing::info;

#[tokio::test]
async fn test_maybe_post_pin_to_board() {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).init();
    info!("running test!!");
    let (browser, mut handler) = Browser::connect("http://127.0.0.1:9222").await.unwrap();
    tokio::spawn(async move {
        use futures::StreamExt;
        while let Some(e) = handler.next().await { let _ = e; }
    });
    let result = maybe_post_pin_to_board(
        &browser,
        "title",
        Some("description"),
        Some("https://example.com"),
        Some(Path::new("../data/image_01.jpg")),
    ).await;
    assert!(result.is_ok());
}
