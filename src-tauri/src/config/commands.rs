//! Tauri IPC commands for configuration and shortcut recording.

use super::controller::SharedConfig;
use super::types::{ConfigError, ConfigSnapshot, OverlayPosition, ShortcutRecordingState, UiMode};
use super::watcher::{emit_changed, emit_error};
use crate::profiles::ProcessingProfileDraft;
use serde::Serialize;
use std::collections::HashSet;
use tauri::{AppHandle, Manager, Runtime, State};

#[derive(Clone, Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CreatedProcessingProfile {
    pub snapshot: ConfigSnapshot,
    pub id: String,
}

#[tauri::command]
#[specta::specta]
pub fn get_capture_config(state: State<'_, SharedConfig>) -> ConfigSnapshot {
    state.lock().expect("config lock poisoned").snapshot()
}

#[tauri::command(rename_all = "camelCase")]
#[specta::specta]
pub fn create_capture_binding(
    app: AppHandle,
    state: State<'_, SharedConfig>,
    profile_id: String,
    shortcut: String,
    expected_revision: u64,
) -> Result<ConfigSnapshot, ConfigError> {
    let result = state.lock().expect("config lock poisoned").create_binding(
        profile_id,
        shortcut,
        expected_revision,
    );
    emit_binding_result(&app, result)
}

#[tauri::command(rename_all = "camelCase")]
#[specta::specta]
pub fn set_capture_binding(
    app: AppHandle,
    state: State<'_, SharedConfig>,
    id: String,
    shortcut: String,
    expected_revision: u64,
) -> Result<ConfigSnapshot, ConfigError> {
    let result =
        state
            .lock()
            .expect("config lock poisoned")
            .set_binding(id, shortcut, expected_revision);
    emit_binding_result(&app, result)
}

#[tauri::command(rename_all = "camelCase")]
#[specta::specta]
pub fn set_open_shortcut(
    app: AppHandle,
    state: State<'_, SharedConfig>,
    shortcut: String,
    expected_revision: u64,
) -> Result<ConfigSnapshot, ConfigError> {
    let result = state
        .lock()
        .expect("config lock poisoned")
        .set_open_shortcut(shortcut, expected_revision);
    emit_binding_result(&app, result)
}

#[tauri::command(rename_all = "camelCase")]
#[specta::specta]
pub fn set_ui_mode(
    app: AppHandle,
    state: State<'_, SharedConfig>,
    mode: UiMode,
    expected_revision: u64,
) -> Result<ConfigSnapshot, ConfigError> {
    let result = state
        .lock()
        .expect("config lock poisoned")
        .set_ui_mode(mode, expected_revision);
    emit_profile_result(&app, result)
}

#[tauri::command(rename_all = "camelCase")]
#[specta::specta]
pub fn set_sounds_enabled(
    app: AppHandle,
    state: State<'_, SharedConfig>,
    enabled: bool,
    expected_revision: u64,
) -> Result<ConfigSnapshot, ConfigError> {
    let result = state
        .lock()
        .expect("config lock poisoned")
        .set_sounds_enabled(enabled, expected_revision);
    emit_profile_result(&app, result)
}

/// Turning originals off also deletes the ones already kept.
#[tauri::command(rename_all = "camelCase")]
#[specta::specta]
pub fn set_keep_history_originals(
    app: AppHandle,
    state: State<'_, SharedConfig>,
    enabled: bool,
    expected_revision: u64,
) -> Result<ConfigSnapshot, ConfigError> {
    let result = state
        .lock()
        .expect("config lock poisoned")
        .set_keep_history_originals(enabled, expected_revision);
    if result.is_ok() && !enabled {
        crate::history::discard_originals(&app);
    }
    emit_profile_result(&app, result)
}

#[tauri::command(rename_all = "camelCase")]
#[specta::specta]
pub fn set_automatic_update_checks(
    app: AppHandle,
    state: State<'_, SharedConfig>,
    enabled: bool,
    expected_revision: u64,
) -> Result<ConfigSnapshot, ConfigError> {
    let result = state
        .lock()
        .expect("config lock poisoned")
        .set_automatic_update_checks(enabled, expected_revision);
    emit_profile_result(&app, result)
}

#[tauri::command(rename_all = "camelCase")]
#[specta::specta]
pub fn set_overlay_position(
    app: AppHandle,
    state: State<'_, SharedConfig>,
    position: OverlayPosition,
    expected_revision: u64,
) -> Result<ConfigSnapshot, ConfigError> {
    let result = state
        .lock()
        .expect("config lock poisoned")
        .set_overlay_position(position, expected_revision);
    let result = emit_profile_result(&app, result);
    if result.is_ok() {
        crate::capture_overlay::reposition(&app, position);
    }
    result
}

#[tauri::command(rename_all = "camelCase")]
#[specta::specta]
pub fn set_capture_binding_profile(
    app: AppHandle,
    state: State<'_, SharedConfig>,
    id: String,
    profile_id: String,
    expected_revision: u64,
) -> Result<ConfigSnapshot, ConfigError> {
    let result = state
        .lock()
        .expect("config lock poisoned")
        .set_binding_profile(id, profile_id, expected_revision);
    emit_binding_result(&app, result)
}

#[tauri::command(rename_all = "camelCase")]
#[specta::specta]
pub fn delete_capture_binding(
    app: AppHandle,
    state: State<'_, SharedConfig>,
    id: String,
    expected_revision: u64,
) -> Result<ConfigSnapshot, ConfigError> {
    let result = state
        .lock()
        .expect("config lock poisoned")
        .delete_binding(id, expected_revision);
    emit_binding_result(&app, result)
}

fn emit_binding_result<R: Runtime>(
    app: &AppHandle<R>,
    result: Result<ConfigSnapshot, ConfigError>,
) -> Result<ConfigSnapshot, ConfigError> {
    match result {
        Ok(snapshot) => {
            emit_changed(app, &snapshot);
            Ok(snapshot)
        }
        Err(error) => {
            emit_error(app, &error);
            Err(error)
        }
    }
}

pub(crate) fn emit_profile_result<R: Runtime>(
    app: &AppHandle<R>,
    result: Result<ConfigSnapshot, ConfigError>,
) -> Result<ConfigSnapshot, ConfigError> {
    match result {
        Ok(snapshot) => {
            emit_changed(app, &snapshot);
            Ok(snapshot)
        }
        Err(error) => {
            emit_error(app, &error);
            Err(error)
        }
    }
}

#[tauri::command(rename_all = "camelCase")]
#[specta::specta]
pub fn create_processing_profile(
    app: AppHandle,
    state: State<'_, SharedConfig>,
    draft: ProcessingProfileDraft,
    expected_revision: u64,
) -> Result<CreatedProcessingProfile, ConfigError> {
    let mut controller = state.lock().expect("config lock poisoned");
    let before: HashSet<String> = controller
        .snapshot()
        .processing
        .profiles
        .iter()
        .map(|profile| profile.id.clone())
        .collect();
    let result = controller.create_processing_profile_draft(draft, expected_revision);
    drop(controller);
    match result {
        Ok(snapshot) => {
            let id = snapshot
                .processing
                .profiles
                .iter()
                .find(|profile| !before.contains(&profile.id))
                .map(|profile| profile.id.clone())
                .expect("successful profile creation adds one profile");
            emit_profile_result(&app, Ok(snapshot.clone()))?;
            Ok(CreatedProcessingProfile { snapshot, id })
        }
        Err(error) => {
            emit_profile_result(&app, Err(error))?;
            unreachable!("error result returned above")
        }
    }
}

#[tauri::command(rename_all = "camelCase")]
#[specta::specta]
pub fn update_processing_profile(
    app: AppHandle,
    state: State<'_, SharedConfig>,
    id: String,
    draft: ProcessingProfileDraft,
    expected_revision: u64,
) -> Result<ConfigSnapshot, ConfigError> {
    let result = state
        .lock()
        .expect("config lock poisoned")
        .update_processing_profile(id, draft, expected_revision);
    emit_profile_result(&app, result)
}

#[tauri::command(rename_all = "camelCase")]
#[specta::specta]
pub fn rename_processing_profile(
    app: AppHandle,
    state: State<'_, SharedConfig>,
    id: String,
    name: String,
    expected_revision: u64,
) -> Result<ConfigSnapshot, ConfigError> {
    let result = state
        .lock()
        .expect("config lock poisoned")
        .rename_processing_profile(id, name, expected_revision);
    emit_profile_result(&app, result)
}

#[tauri::command(rename_all = "camelCase")]
#[specta::specta]
pub fn delete_processing_profile(
    app: AppHandle,
    state: State<'_, SharedConfig>,
    id: String,
    expected_revision: u64,
) -> Result<ConfigSnapshot, ConfigError> {
    let result = state
        .lock()
        .expect("config lock poisoned")
        .delete_processing_profile(id, expected_revision);
    emit_profile_result(&app, result)
}

#[tauri::command(rename_all = "camelCase")]
#[specta::specta]
pub fn select_processing_profile(
    app: AppHandle,
    state: State<'_, SharedConfig>,
    id: String,
    expected_revision: u64,
) -> Result<ConfigSnapshot, ConfigError> {
    let result = state
        .lock()
        .expect("config lock poisoned")
        .select_processing_profile(id, expected_revision);
    emit_profile_result(&app, result)
}

#[tauri::command]
#[specta::specta]
pub fn set_capture_shortcut_recording(
    app: AppHandle,
    state: State<'_, ShortcutRecordingState>,
    config: State<'_, SharedConfig>,
    recording: bool,
) -> Result<bool, String> {
    if recording {
        let window = app
            .get_webview_window("main")
            .ok_or_else(|| "main window is unavailable".to_string())?;
        let mut config = config.lock().expect("config lock poisoned");
        if !window.is_focused().map_err(|error| error.to_string())? {
            state.set_recording(false);
            return Ok(false);
        }
        state.set_recording(true);
        if let Err(error) = config.suspend_capture_shortcut() {
            state.set_recording(false);
            return Err(error.to_string());
        }
        return Ok(true);
    }

    let result = config
        .lock()
        .expect("config lock poisoned")
        .resume_capture_shortcut();
    match result {
        Ok(_) => {
            state.set_recording(false);
            Ok(true)
        }
        Err(error) => Err(error.to_string()),
    }
}
