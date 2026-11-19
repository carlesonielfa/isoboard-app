//! Rust-owned IPC contracts and generated TypeScript bindings.
//!
//! The types registered here are the serialized shapes that cross the Tauri
//! boundary. Persisted configuration and internal state intentionally remain
//! separate when their representation serves a different purpose.

use crate::capture::CaptureStatus;
use crate::config::{ConfigError, ConfigSnapshot};
use crate::history::{HistoryChangedEvent, HistoryErrorEvent, OpenHistoryEvent};
use crate::model_catalog::ModelCatalogSnapshot;
use specta_typescript::Typescript;
use std::path::Path;
use tauri_specta::{collect_commands, collect_events, Builder, ErrorHandlingMode};

/// Build the shared contract registry used by the running app and exporter.
pub(crate) fn builder() -> Builder<tauri::Wry> {
    Builder::new()
        .typ::<crate::product_config::ProductConfig>()
        .typ::<crate::application_config::ApplicationConfig>()
        .typ::<crate::application_config::ServiceConfig>()
        .typ::<crate::application_config::ModelDefaults>()
        // Sequence/revision values already use JSON numbers at this IPC
        // boundary. Preserve that established wire representation instead of
        // widening every frontend consumer to bigint/string transport.
        .dangerously_cast_bigints_to_number()
        // Keep the existing invoke() contract: command failures reject the
        // promise and are handled by the controllers' existing error paths.
        .error_handling(ErrorHandlingMode::Throw)
        .events(collect_events![
            crate::licensing::commercial::LicenseChangedEvent,
            CaptureStatus,
            crate::capture_overlay::CaptureOverlayPresentation,
            crate::local_models::LocalModelsChangedEvent,
            ConfigSnapshot,
            ConfigError,
            ModelCatalogSnapshot,
            HistoryChangedEvent,
            HistoryErrorEvent,
            OpenHistoryEvent,
            crate::updates::UpdateStatus,
        ])
        .commands(collect_commands![
            crate::licensing::get_license_status,
            crate::licensing::activate_commercial_license,
            crate::licensing::remove_commercial_license,
            crate::licensing::recheck_commercial_license,
            crate::licensing::continue_without_license_key,
            crate::licensing::dismiss_evaluation_notice,
            crate::capture::start_region_capture_command,
            crate::capture::open_image_command,
            crate::capture::get_capture_snapshot,
            crate::capture_overlay::open_capture_overlay_history,
            crate::resource_usage::get_app_memory_usage,
            crate::local_models::commands::get_local_models_state,
            crate::local_models::commands::prepare_local_model,
            crate::local_models::commands::download_local_model,
            crate::local_models::commands::cancel_local_model_download,
            crate::local_models::commands::remove_local_model_download,
            crate::config::commands::get_capture_config,
            crate::config::commands::create_capture_binding,
            crate::config::commands::set_capture_binding,
            crate::config::commands::set_open_shortcut,
            crate::config::commands::set_ui_mode,
            crate::config::commands::set_sounds_enabled,
            crate::config::commands::set_keep_history_originals,
            crate::config::commands::set_automatic_update_checks,
            crate::config::commands::set_overlay_position,
            crate::config::commands::set_capture_binding_profile,
            crate::config::commands::delete_capture_binding,
            crate::config::commands::set_capture_shortcut_recording,
            crate::config::commands::create_processing_profile,
            crate::config::commands::update_processing_profile,
            crate::config::commands::rename_processing_profile,
            crate::config::commands::delete_processing_profile,
            crate::config::commands::select_processing_profile,
            crate::config::inference_commands::get_inference_backend_metadata,
            crate::config::inference_commands::save_provider_with_credential,
            crate::config::inference_commands::get_inference_credential_readiness,
            crate::config::inference_commands::delete_unreferenced_inference_credential,
            crate::model_catalog::get_inference_model_catalog,
            crate::history::list_history,
            crate::history::get_history_preview,
            crate::history::get_history_facets,
            crate::history::get_pending_history_navigation,
            crate::history::acknowledge_history_navigation,
            crate::history::copy_failed_history_details,
            crate::history::restore_history_entry,
            crate::history::copy_history_plain_text,
            crate::history::preview_history_formatting,
            crate::history::copy_history_formatted,
            crate::history::copy_history_debug_html,
            crate::history::delete_history_entry,
            crate::history::clear_history,
            crate::window_presentation::get_window_presentation_capabilities,
            crate::window_presentation::get_titlebar_controls_center_y,
            crate::window_presentation::show_main_window,
            crate::screen_capture_permission::get_screen_capture_permission_status,
            crate::screen_capture_permission::request_screen_capture_permission,
            crate::screen_capture_permission::open_screen_capture_settings,
            crate::screen_capture_permission::restart_after_screen_capture_access,
            crate::notifications::get_notification_permission,
            crate::notifications::request_notification_permission,
            crate::notifications::open_notification_settings,
            crate::updates::get_update_status,
            crate::updates::get_changelog,
            crate::updates::check_for_updates,
            crate::updates::install_update,
        ])
}

/// Export the committed TypeScript contract file from the Rust definitions.
pub fn export_typescript(path: impl AsRef<Path>) -> Result<(), String> {
    use std::io::Write;
    let path = path.as_ref();
    builder()
        .export(Typescript::default(), path)
        .map_err(|error| format!("could not export TypeScript IPC bindings: {error}"))?;
    let mut fallback = crate::inference_profiles::all_backend_metadata();
    for backend in &mut fallback {
        backend.built_in_profiles.clear();
    }
    let metadata = serde_json::to_string(&fallback).map_err(|error| error.to_string())?;
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(path)
        .map_err(|error| error.to_string())?;
    writeln!(file, "\nexport const DEFAULT_INFERENCE_BACKEND_METADATA: InferenceBackendMetadata[] = {metadata};")
        .map_err(|error| error.to_string())?;
    for (name, schema, value) in [
        (
            "PRODUCT_CONFIG",
            "ProductConfig",
            serde_json::to_value(&*crate::product_config::PRODUCT),
        ),
        (
            "APPLICATION_CONFIG",
            "ApplicationConfig",
            serde_json::to_value(&*crate::application_config::APPLICATION),
        ),
        (
            "MODEL_DEFAULTS",
            "ModelDefaults",
            serde_json::to_value(&*crate::application_config::MODEL_DEFAULTS),
        ),
    ] {
        let value = value.map_err(|error| error.to_string())?;
        writeln!(file, "export const {name} = {value} satisfies {schema};")
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::export_typescript;

    #[test]
    fn exported_bindings_are_regenerated_from_rust_contracts() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../src/lib/generated/bindings.ts");
        let expected = std::fs::read(&path).expect("committed TypeScript bindings should exist");
        let temporary = tempfile::NamedTempFile::new().expect("temporary export path");
        export_typescript(temporary.path()).expect("TypeScript bindings should export");
        let actual = std::fs::read(temporary.path()).expect("generated bindings should exist");
        assert!(
            actual == expected,
            "generated TypeScript bindings are stale; run `make bindings`"
        );
    }
}
