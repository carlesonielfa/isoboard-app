//! Native adapters for shared local-model controls.
use super::{LocalModels, LocalModelsChangedEvent};
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

fn models(app: &AppHandle) -> Result<Arc<LocalModels>, String> {
    app.try_state::<Arc<LocalModels>>()
        .map(|state| state.inner().clone())
        .ok_or_else(|| "Local inference is unavailable in this build.".into())
}
fn require_idle_capture(app: &AppHandle) -> Result<(), String> {
    if app
        .try_state::<crate::capture::CaptureCoordinator>()
        .is_some_and(|capture| capture.is_busy())
    {
        return Err("Wait until capture finishes.".into());
    }
    Ok(())
}
#[tauri::command]
#[specta::specta]
pub fn get_local_models_state(app: AppHandle) -> LocalModelsChangedEvent {
    models(&app).map_or(
        LocalModelsChangedEvent {
            revision: 0,
            models: Vec::new(),
        },
        |models| models.snapshot(),
    )
}
#[tauri::command]
#[specta::specta]
pub async fn prepare_local_model(
    app: AppHandle,
    model: String,
) -> Result<LocalModelsChangedEvent, String> {
    require_idle_capture(&app)?;
    #[cfg(feature = "local-inference")]
    {
        let service = app.state::<Arc<crate::inference::InferenceService>>();
        let worker = service
            .local_worker()
            .ok_or("Local inference is unavailable.")?;
        worker
            .warm_up(&model)
            .await
            .map_err(|error| error.to_string())?;
        Ok(models(&app)?.snapshot())
    }
    #[cfg(not(feature = "local-inference"))]
    {
        let _ = model;
        Err("Local inference is unavailable in this build.".into())
    }
}
#[tauri::command]
#[specta::specta]
pub async fn download_local_model(
    app: AppHandle,
    model: String,
) -> Result<LocalModelsChangedEvent, String> {
    let models = models(&app)?;
    models.start_download(&model)?;
    Ok(models.snapshot())
}
#[tauri::command]
#[specta::specta]
pub async fn cancel_local_model_download(
    app: AppHandle,
    model: String,
) -> Result<LocalModelsChangedEvent, String> {
    let models = models(&app)?;
    models.cancel_download(&model).await?;
    Ok(models.snapshot())
}
#[tauri::command]
#[specta::specta]
pub async fn remove_local_model_download(
    app: AppHandle,
    model: String,
) -> Result<LocalModelsChangedEvent, String> {
    require_idle_capture(&app)?;
    let models = models(&app)?;
    let name = super::definition(&model)
        .ok_or("Unknown local model.")?
        .name
        .clone();
    let (send, receive) = tokio::sync::oneshot::channel();
    let mut dialog = app.dialog()
        .message(format!("Remove {name} from this device? Profiles using it will need the model to be downloaded again."))
        .title("Delete local model?")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom("Delete model".into(), "Cancel".into()));
    if let Some(window) = app.get_webview_window("main") {
        dialog = dialog.parent(&window);
    }
    dialog.show(move |confirmed| {
        let _ = send.send(confirmed);
    });
    if !receive.await.map_err(|error| error.to_string())? {
        return Ok(models.snapshot());
    }
    require_idle_capture(&app)?;
    let guard = models.begin_operation(&model, true)?;
    unload(&app, &model).await?;
    models.remove_download(&model).await?;
    drop(guard);
    Ok(models.snapshot())
}
async fn unload(app: &AppHandle, model: &str) -> Result<(), String> {
    #[cfg(feature = "local-inference")]
    {
        let service = app.state::<Arc<crate::inference::InferenceService>>();
        let worker = service
            .local_worker()
            .ok_or("Local inference is unavailable.")?;
        worker
            .unload_model(Some(model.into()))
            .await
            .map_err(|error| error.to_string())
    }
    #[cfg(not(feature = "local-inference"))]
    {
        let _ = (app, model);
        Err("Local inference is unavailable in this build.".into())
    }
}
