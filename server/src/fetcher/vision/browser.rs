use std::time::Duration;

use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat;
use futures::StreamExt;

use crate::error::AppError;

async fn launch_browser() -> Result<(Browser, tokio::task::JoinHandle<()>), AppError> {
    let config = BrowserConfig::builder()
        .chrome_executable("C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe")
        .no_sandbox()
        .window_size(1400, 900)
        .arg("--disable-gpu")
        .arg("--disable-dev-shm-usage")
        .build()
        .map_err(|err| AppError::Internal(format!("Failed to configure browser: {err}")))?;

    let (browser, mut handler) = Browser::launch(config)
        .await
        .map_err(|err| AppError::Internal(format!("Failed to launch browser: {err}")))?;

    let handler_task = tokio::spawn(async move {
        while let Some(event) = handler.next().await {
            let _ = event;
        }
    });

    Ok((browser, handler_task))
}

/// Get the rendered DOM HTML of a URL using headless Chrome.
pub async fn dump_dom(url: &str) -> Result<String, AppError> {
    let (mut browser, handler_task) = launch_browser().await?;

    let page = browser
        .new_page(url)
        .await
        .map_err(|err| AppError::Internal(format!("Failed to open page: {err}")))?;

    // Wait for page to load and JS to execute
    tokio::time::sleep(Duration::from_secs(8)).await;

    let html: String = page
        .evaluate("document.documentElement.outerHTML")
        .await
        .map_err(|err| AppError::Internal(format!("Failed to get DOM: {err}")))?
        .into_value()
        .unwrap_or_default();

    let _ = browser.close().await;
    handler_task.abort();

    tracing::info!("dump_dom got {} chars from {url}", html.len());
    Ok(html)
}

/// Take full-page screenshots of a URL using headless Chrome.
/// Returns one or more PNG screenshots (scrolls through the page).
pub async fn screenshot_page(url: &str) -> Result<Vec<Vec<u8>>, AppError> {
    let (mut browser, handler_task) = launch_browser().await?;

    let result = screenshot_with_browser(&browser, url).await;

    // Clean up
    let _ = browser.close().await;
    handler_task.abort();

    result
}

async fn screenshot_with_browser(
    browser: &Browser,
    url: &str,
) -> Result<Vec<Vec<u8>>, AppError> {
    let page = browser
        .new_page(url)
        .await
        .map_err(|err| AppError::Internal(format!("Failed to open page: {err}")))?;

    // Wait for page to load
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Try to dismiss any cookie/popup banners
    let _ = page
        .evaluate(
            r#"
            document.querySelectorAll('[class*="cookie"], [class*="banner"], [class*="popup"], [id*="cookie"]')
                .forEach(el => el.remove());
            window.scrollTo(0, 0);
        "#,
        )
        .await;

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Get page height
    let height: f64 = page
        .evaluate("document.body.scrollHeight")
        .await
        .map_err(|err| AppError::Internal(format!("Failed to get page height: {err}")))?
        .into_value()
        .unwrap_or(900.0);

    let viewport_height = 900.0;
    let num_screenshots = ((height / viewport_height).ceil() as usize).min(8);

    let mut screenshots = Vec::new();

    for i in 0..num_screenshots {
        let scroll_y = (i as f64) * viewport_height;
        let _ = page
            .evaluate(format!("window.scrollTo(0, {scroll_y})"))
            .await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let screenshot = page
            .screenshot(
                chromiumoxide::page::ScreenshotParams::builder()
                    .format(CaptureScreenshotFormat::Png)
                    .build(),
            )
            .await
            .map_err(|err| AppError::Internal(format!("Screenshot failed: {err}")))?;

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
