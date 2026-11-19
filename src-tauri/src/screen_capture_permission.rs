use serde::Serialize;
use tauri::{AppHandle, Manager};

#[cfg(target_os = "macos")]
mod macos;

const PROBE_ARGUMENT: &str = "--isoboard-screen-access-status";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ScreenCapturePermissionStatus {
    Granted,
    RestartRequired,
    Required,
    #[allow(dead_code)]
    Unsupported,
}

pub(crate) fn granted() -> bool {
    #[cfg(target_os = "macos")]
    return macos::granted();
    #[cfg(not(target_os = "macos"))]
    false
}

pub(crate) fn run_probe_if_requested() -> bool {
    let mut arguments = std::env::args_os().skip(1);
    if arguments.next().as_deref() != Some(std::ffi::OsStr::new(PROBE_ARGUMENT))
        || arguments.next().is_some()
    {
        return false;
    }
    println!("{}", if granted() { "granted" } else { "required" });
    true
}

#[cfg(any(target_os = "macos", test))]
fn resolved_status(current: bool, fresh: bool) -> ScreenCapturePermissionStatus {
    if !fresh {
        ScreenCapturePermissionStatus::Required
    } else if current {
        ScreenCapturePermissionStatus::Granted
    } else {
        ScreenCapturePermissionStatus::RestartRequired
    }
}

async fn current_status() -> Result<ScreenCapturePermissionStatus, String> {
    #[cfg(target_os = "macos")]
    return tauri::async_runtime::spawn_blocking(|| {
        // CoreGraphics caches permission per process. A fresh non-prompting check detects grants
        // that are saved in macOS but still require this process to restart.
        let current = macos::granted();
        macos::fresh_process_granted().map(|fresh| resolved_status(current, fresh))
    })
    .await
    .map_err(|error| error.to_string())?;
    #[cfg(not(target_os = "macos"))]
    Ok(ScreenCapturePermissionStatus::Unsupported)
}

fn refresh_menu(app: &AppHandle) {
    if let Err(error) = crate::history::refresh_tray_menu(app) {
        eprintln!("isoboard: could not refresh capture menu after permission check: {error}");
    }
}

fn remember_permission_return(app: &AppHandle) -> Result<(), String> {
    let config = app.state::<crate::config::SharedConfig>();
    let config = config.lock().expect("config lock poisoned");
    crate::launch::open_on_next_launch(config.config_path())
        .map_err(|error| format!("Could not remember the permission restart: {error}"))
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn get_screen_capture_permission_status(
    app: AppHandle,
) -> Result<ScreenCapturePermissionStatus, String> {
    let status = current_status().await?;
    refresh_menu(&app);
    Ok(status)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn request_screen_capture_permission(
    app: AppHandle,
) -> Result<ScreenCapturePermissionStatus, String> {
    #[cfg(target_os = "macos")]
    {
        if !macos::granted() {
            remember_permission_return(&app)?;
        }
        macos::request().await?;
    }
    let status = current_status().await?;
    refresh_menu(&app);
    Ok(status)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn open_screen_capture_settings(app: AppHandle) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        if !macos::granted() {
            remember_permission_return(&app)?;
            macos::request().await?;
        }
        macos::open_settings()
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        Err("Screen Recording settings are only available on macOS.".into())
    }
}

#[tauri::command]
#[specta::specta]
pub(crate) fn restart_after_screen_capture_access(app: AppHandle) -> Result<(), String> {
    remember_permission_return(&app)?;
    app.restart();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restart_requires_a_confirmed_grant_in_a_fresh_process() {
        assert_eq!(
            resolved_status(false, false),
            ScreenCapturePermissionStatus::Required
        );
        assert_eq!(
            resolved_status(false, true),
            ScreenCapturePermissionStatus::RestartRequired
        );
        assert_eq!(
            resolved_status(true, true),
            ScreenCapturePermissionStatus::Granted
        );
        assert_eq!(
            resolved_status(true, false),
            ScreenCapturePermissionStatus::Required
        );
    }
}
