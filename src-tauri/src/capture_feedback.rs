//! Capture status in the menu bar while the floating overlay is transient.

use tauri::{AppHandle, Runtime};

use crate::capture::{CaptureFailureCode, CaptureStatus};

pub(crate) fn status_label(status: &CaptureStatus) -> std::borrow::Cow<'static, str> {
    if let CaptureStatus::DownloadingModel {
        downloaded_bytes,
        total_bytes,
        ..
    } = status
    {
        let percent = downloaded_bytes.saturating_mul(100) / (*total_bytes).max(1);
        return format!("Downloading model · {}%", percent.min(100)).into();
    }
    let label = match status {
        CaptureStatus::Idle { .. } => "No capture in progress",
        CaptureStatus::Started { source, .. } => match source {
            crate::capture::CaptureSource::Region => "Select a region",
            crate::capture::CaptureSource::File => "Choose an image",
        },
        CaptureStatus::ImageReady { .. } => "Image selected · Processing",
        CaptureStatus::DownloadingModel { .. } => "Downloading model",
        CaptureStatus::LoadingModel { .. } => "Loading model",
        CaptureStatus::Processing { .. } => "Processing image",
        CaptureStatus::Completed { .. } => "Copied to clipboard",
        CaptureStatus::Cancelled { .. } => "Capture cancelled",
        CaptureStatus::Failed { code, .. } => match code {
            CaptureFailureCode::PermissionDenied => "Screen access needed",
            CaptureFailureCode::Inference => "Provider failed",
            CaptureFailureCode::Clipboard => "Clipboard failed",
            _ => "Capture failed",
        },
    };
    label.into()
}

pub(crate) fn update<R: Runtime>(app: &AppHandle<R>, status: &CaptureStatus) {
    if let Some(tray) = app.tray_by_id("main") {
        if let Err(error) = tray.set_tooltip(Some(format!("isoboard · {}", status_label(status))))
        {
            eprintln!("isoboard: could not update capture tooltip: {error}");
        }
        if let Err(error) = crate::history::refresh_tray_menu(app) {
            eprintln!("isoboard: could not update capture menu: {error}");
        }
    }
}
