use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime};

#[cfg(target_os = "macos")]
mod macos;

pub(crate) fn cursor_over_window<R: Runtime>(
    window: &tauri::WebviewWindow<R>,
) -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        macos::cursor_over_window(window)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let cursor = window
            .app_handle()
            .cursor_position()
            .map_err(|error| error.to_string())?;
        let origin = window.outer_position().map_err(|error| error.to_string())?;
        let size = window.outer_size().map_err(|error| error.to_string())?;
        let left = f64::from(origin.x);
        let top = f64::from(origin.y);
        Ok(cursor.x >= left
            && cursor.x < left + f64::from(size.width)
            && cursor.y >= top
            && cursor.y < top + f64::from(size.height))
    }
}

pub(crate) fn set_overlay_opacity<R: Runtime>(
    window: &tauri::WebviewWindow<R>,
    opacity: f64,
    duration: std::time::Duration,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        macos::set_opacity(window, opacity, duration)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window, opacity, duration);
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PresentationCapabilities {
    pub(crate) native_backdrop: bool,
    pub(crate) shortcut_presentation_platform: ShortcutPresentationPlatform,
}

#[derive(Clone, Copy, Debug, Serialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ShortcutPresentationPlatform {
    #[cfg(target_os = "macos")]
    Macos,
    #[cfg(target_os = "windows")]
    Windows,
    #[cfg(target_os = "linux")]
    Linux,
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    Generic,
}

pub(crate) fn reveal_main_window<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window is unavailable".to_string())?;
    window.unminimize().map_err(|error| error.to_string())?;
    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())
}

#[tauri::command]
#[specta::specta]
pub(crate) fn show_main_window(app: AppHandle) -> Result<(), String> {
    reveal_main_window(&app)
}

fn shortcut_presentation_platform() -> ShortcutPresentationPlatform {
    #[cfg(target_os = "macos")]
    {
        ShortcutPresentationPlatform::Macos
    }

    #[cfg(target_os = "windows")]
    {
        ShortcutPresentationPlatform::Windows
    }

    #[cfg(target_os = "linux")]
    {
        ShortcutPresentationPlatform::Linux
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        ShortcutPresentationPlatform::Generic
    }
}

#[tauri::command]
#[specta::specta]
pub(crate) fn get_window_presentation_capabilities() -> PresentationCapabilities {
    PresentationCapabilities {
        native_backdrop: cfg!(target_os = "macos"),
        shortcut_presentation_platform: shortcut_presentation_platform(),
    }
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn get_titlebar_controls_center_y(
    window: tauri::WebviewWindow,
) -> Result<Option<f64>, String> {
    #[cfg(target_os = "macos")]
    {
        macos::titlebar_controls_center_y(window).await
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = window;
        Ok(None)
    }
}
