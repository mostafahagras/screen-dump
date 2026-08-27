use std::path::Path;

use screencapturekit::prelude::*;
use screencapturekit::screenshot_manager::SCScreenshotManager;
use thiserror::Error;

use crate::model::DisplayInfo;

#[derive(Debug, Error)]
pub enum Error {
    #[error("screenshot path must be valid UTF-8")]
    NonUtf8Path,
    #[error("refusing to overwrite existing screenshot {0}")]
    AlreadyExists(String),
    #[error("no active main display is available")]
    NoMainDisplay,
    #[error("ScreenCaptureKit failed: {0}")]
    Capture(String),
}

pub fn capture_main_display(
    path: &Path,
    single_display: Option<&DisplayInfo>,
    displays: Option<&[DisplayInfo]>,
) -> Result<(), Error> {
    if path.exists() {
        return Err(Error::AlreadyExists(path.display().to_string()));
    }
    let display = single_display
        .or_else(|| displays.and_then(|items| items.iter().find(|display| display.main)))
        .ok_or(Error::NoMainDisplay)?;
    let path = path.to_str().ok_or(Error::NonUtf8Path)?;

    let content = SCShareableContent::get().map_err(|error| Error::Capture(error.to_string()))?;
    let capture_display = content
        .displays()
        .into_iter()
        .find(|candidate| candidate.display_id() == display.id)
        .ok_or_else(|| Error::Capture(format!("display {} is not capturable", display.id)))?;
    let filter = SCContentFilter::create()
        .with_display(&capture_display)
        .with_excluding_windows(&[])
        .build();
    let config = SCStreamConfiguration::new()
        .with_width(display.pixel_width as u32)
        .with_height(display.pixel_height as u32)
        .with_shows_cursor(false);
    let image = SCScreenshotManager::capture_image(&filter, &config)
        .map_err(|error| Error::Capture(error.to_string()))?;
    image
        .save_png(path)
        .map_err(|error| Error::Capture(error.to_string()))
}
