#[cfg(target_os = "macos")]
mod macos;

#[derive(Clone, Copy, Debug, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) enum NotificationPermission {
    Required,
    Granted,
    Denied,
    Unsupported,
}

pub(crate) fn setup() {
    #[cfg(target_os = "macos")]
    macos::setup();
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn get_notification_permission() -> Result<NotificationPermission, String> {
    #[cfg(target_os = "macos")]
    return macos::permission().await;
    #[cfg(not(target_os = "macos"))]
    Ok(NotificationPermission::Unsupported)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn request_notification_permission() -> Result<NotificationPermission, String> {
    #[cfg(target_os = "macos")]
    return macos::request_permission().await;
    #[cfg(not(target_os = "macos"))]
    Ok(NotificationPermission::Unsupported)
}

#[tauri::command]
#[specta::specta]
pub(crate) fn open_notification_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    return macos::open_settings();
    #[cfg(not(target_os = "macos"))]
    Err("Notification settings are only available on macOS.".into())
}

pub(crate) fn model_downloaded(model: &str) {
    #[cfg(target_os = "macos")]
    {
        let model = model.to_owned();
        tauri::async_runtime::spawn(async move {
            if matches!(
                macos::permission().await,
                Ok(NotificationPermission::Granted)
            ) {
                if let Err(error) = macos::model_downloaded(&model) {
                    eprintln!("isoboard: could not send model download notification: {error}");
                }
            }
        });
    }
    #[cfg(not(target_os = "macos"))]
    let _ = model;
}
