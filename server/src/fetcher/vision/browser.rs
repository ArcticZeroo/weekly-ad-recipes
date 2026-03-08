use std::time::Duration;

use headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption;
use headless_chrome::{Browser, LaunchOptions};

use crate::error::AppError;

fn launch_browser() -> Result<Browser, AppError> {
    let mut builder = LaunchOptions::default_builder();
    builder
        .window_size(Some((1400, 900)))
        .sandbox(false)
        .idle_browser_timeout(Duration::from_secs(60));

    if let Ok(chrome_bin) = std::env::var("CHROME_BIN") {
        builder.path(Some(chrome_bin.into()));
    }

    let options = builder
        .build()
        .map_err(|error| AppError::Internal(format!("Failed to configure browser: {error}")))?;

    Browser::new(options)
        .map_err(|error| AppError::Internal(format!("Failed to launch browser: {error}")))
}

/// Get the rendered DOM HTML of a URL using headless Chrome.
pub async fn dump_dom(url: &str) -> Result<String, AppError> {
    let url = url.to_string();
    tokio::task::spawn_blocking(move || dump_dom_sync(&url))
        .await
        .map_err(|error| AppError::Internal(format!("Browser task panicked: {error}")))?
}

fn dump_dom_sync(url: &str) -> Result<String, AppError> {
    let browser = launch_browser()?;
    let tab = browser
        .new_tab()
        .map_err(|error| AppError::Internal(format!("Failed to create tab: {error}")))?;

    tab.navigate_to(url)
        .map_err(|error| AppError::Internal(format!("Failed to navigate: {error}")))?;

    std::thread::sleep(Duration::from_secs(8));

    let result = tab
        .evaluate("document.documentElement.outerHTML", false)
        .map_err(|error| AppError::Internal(format!("Failed to get DOM: {error}")))?;

    let html = result
        .value
        .and_then(|value| value.as_str().map(String::from))
        .unwrap_or_default();

    tracing::info!("dump_dom got {} chars from {url}", html.len());
    Ok(html)
}

/// Take full-page screenshots of a URL using headless Chrome.
/// Returns one or more PNG screenshots (scrolls through the page).
pub async fn screenshot_page(url: &str) -> Result<Vec<Vec<u8>>, AppError> {
    let url = url.to_string();
    tokio::task::spawn_blocking(move || screenshot_page_sync(&url))
        .await
        .map_err(|error| AppError::Internal(format!("Browser task panicked: {error}")))?
}

fn screenshot_page_sync(url: &str) -> Result<Vec<Vec<u8>>, AppError> {
    let browser = launch_browser()?;
    let tab = browser
        .new_tab()
        .map_err(|error| AppError::Internal(format!("Failed to create tab: {error}")))?;

    tab.navigate_to(url)
        .map_err(|error| AppError::Internal(format!("Failed to navigate: {error}")))?;

    std::thread::sleep(Duration::from_secs(5));

    // Try to dismiss any cookie/popup banners
    let _ = tab.evaluate(
        r#"document.querySelectorAll('[class*="cookie"], [class*="banner"], [class*="popup"], [id*="cookie"]').forEach(el => el.remove()); window.scrollTo(0, 0);"#,
        false,
    );

    std::thread::sleep(Duration::from_millis(500));

    let height_result = tab
        .evaluate("document.body.scrollHeight", false)
        .map_err(|error| AppError::Internal(format!("Failed to get page height: {error}")))?;

    let height: f64 = height_result
        .value
        .and_then(|value| value.as_f64())
        .unwrap_or(900.0);

    let viewport_height = 900.0;
    let num_screenshots = ((height / viewport_height).ceil() as usize).min(8);

    let mut screenshots = Vec::new();

    for i in 0..num_screenshots {
        let scroll_y = (i as f64) * viewport_height;
        let _ = tab.evaluate(&format!("window.scrollTo(0, {scroll_y})"), false);
        std::thread::sleep(Duration::from_millis(500));

        let screenshot = tab
            .capture_screenshot(CaptureScreenshotFormatOption::Png, None, None, true)
            .map_err(|error| AppError::Internal(format!("Screenshot failed: {error}")))?;

        screenshots.push(screenshot);
    }

    tracing::info!(
        "Captured {} screenshots of {} (page height: {:.0}px)",
        screenshots.len(),
        url,
        height
    );

    Ok(screenshots)
}
