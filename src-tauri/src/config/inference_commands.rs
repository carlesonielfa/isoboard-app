//! Tauri IPC commands for shared provider authentication and model metadata.

use super::controller::SharedConfig;
use super::types::{ConfigError, ConfigSnapshot};
use super::ManagedCredentialsState;
use crate::inference_profiles::{
    backend_metadata, InferenceBackend, InferenceCredentialReadiness,
    InferenceCredentialReadinessSnapshot,
};
use crate::managed_credentials::ManagedCredentialReadiness;
use crate::secret_store::{ManagedSecretKind, ManagedSecretRef};
use std::path::Path;
use tauri::{Manager, State};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

#[tauri::command]
#[specta::specta]
pub fn get_inference_backend_metadata() -> Vec<crate::inference_profiles::InferenceBackendMetadata>
{
    backend_metadata()
}

#[tauri::command(rename_all = "camelCase")]
#[specta::specta]
pub async fn save_provider_with_credential(
    app: tauri::AppHandle,
    state: State<'_, SharedConfig>,
    credentials: State<'_, ManagedCredentialsState>,
    provider: String,
    endpoint: Option<String>,
    credential: String,
    expected_revision: u64,
) -> Result<ConfigSnapshot, ConfigError> {
    let config = state.inner().clone();
    let store = credentials.0.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let mut controller = config.lock().expect("config lock poisoned");
        let config_path = controller.config_path().to_path_buf();
        let Some(store) = store else {
            return Err(controller.remember_error(ConfigError::runtime(
                &config_path,
                "managed credential storage is unavailable",
                None,
            )));
        };
        controller.save_provider_with_secret(
            provider,
            endpoint,
            credential.as_bytes(),
            &store,
            expected_revision,
        )
    })
    .await
    .map_err(|_| {
        ConfigError::runtime(
            Path::new("config.toml"),
            "credential save task failed",
            None,
        )
    })?;
    super::commands::emit_profile_result(&app, result)
}

#[tauri::command(rename_all = "camelCase")]
#[specta::specta]
pub async fn get_inference_credential_readiness(
    app: tauri::AppHandle,
    state: State<'_, SharedConfig>,
    credentials: State<'_, ManagedCredentialsState>,
    provider: String,
) -> Result<InferenceCredentialReadinessSnapshot, ConfigError> {
    let config = state.inner().clone();
    let store = credentials.0.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let controller = config.lock().expect("config lock poisoned");
        let backend = InferenceBackend::parse(&provider).ok_or_else(|| {
            ConfigError::validation(
                controller.config_path(),
                "unsupported inference provider",
                None,
            )
        })?;
        let provider_config = controller.current_provider(backend);
        if backend == InferenceBackend::Local {
            return Ok(InferenceCredentialReadinessSnapshot {
                provider: backend,
                credential_ref: None,
                readiness: if crate::local_models::supported() {
                    InferenceCredentialReadiness::Ready
                } else {
                    InferenceCredentialReadiness::Unavailable
                },
            });
        }
        let Some(reference) = provider_config.and_then(|config| config.credential_ref.as_deref())
        else {
            let keyless = backend.credential_optional()
                && provider_config.is_some_and(|config| config.endpoint.is_some());
            return Ok(InferenceCredentialReadinessSnapshot {
                provider: backend,
                credential_ref: None,
                readiness: if keyless {
                    InferenceCredentialReadiness::Ready
                } else {
                    InferenceCredentialReadiness::Missing
                },
            });
        };
        let reference = ManagedSecretRef::parse(reference).map_err(|_| {
            ConfigError::validation(
                controller.config_path(),
                "managed credential reference is invalid",
                None,
            )
        })?;
        let readiness = store
            .as_ref()
            .map(|store| store.readiness(managed_kind(backend), &reference))
            .unwrap_or(ManagedCredentialReadiness::Unavailable);
        Ok(InferenceCredentialReadinessSnapshot {
            provider: backend,
            credential_ref: Some(reference.to_string()),
            readiness: readiness.into(),
        })
    })
    .await
    .map_err(|_| {
        ConfigError::runtime(
            Path::new("config.toml"),
            "credential readiness task failed",
            None,
        )
    })?;
    if let Err(error) = crate::history::refresh_tray_menu(&app) {
        eprintln!("isoboard: could not refresh capture menu after credential check: {error}");
    }
    result
}

/// Asks for confirmation, then removes a provider's stored API key from the
/// configuration and secure storage. A cancelled dialog returns the current
/// snapshot unchanged.
#[tauri::command(rename_all = "camelCase")]
#[specta::specta]
pub async fn clear_provider_credential(
    app: tauri::AppHandle,
    state: State<'_, SharedConfig>,
    credentials: State<'_, ManagedCredentialsState>,
    provider: String,
    provider_label: String,
    expected_revision: u64,
) -> Result<ConfigSnapshot, ConfigError> {
    let (send, receive) = tokio::sync::oneshot::channel();
    let mut dialog = app
        .dialog()
        .message(format!(
            "Remove the {provider_label} API key from this device? Profiles using {provider_label} will need a new key."
        ))
        .title("Remove API key?")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Remove key".into(),
            "Cancel".into(),
        ));
    if let Some(window) = app.get_webview_window("main") {
        dialog = dialog.parent(&window);
    }
    dialog.show(move |confirmed| {
        let _ = send.send(confirmed);
    });
    let confirmed = receive.await.unwrap_or(false);
    let config = state.inner().clone();
    let store = credentials.0.clone();
    let capture_busy = app
        .try_state::<crate::capture::CaptureCoordinator>()
        .is_some_and(|capture| capture.is_busy());
    let result = tauri::async_runtime::spawn_blocking(move || {
        let mut controller = config.lock().expect("config lock poisoned");
        if !confirmed {
            return Ok(controller.snapshot());
        }
        let config_path = controller.config_path().to_path_buf();
        // A running capture may still read the key it resolved at its start.
        if capture_busy {
            return Err(controller.remember_error(ConfigError::validation(
                &config_path,
                "wait until capture finishes before removing the API key",
                None,
            )));
        }
        let Some(store) = store else {
            return Err(controller.remember_error(ConfigError::runtime(
                &config_path,
                "managed credential storage is unavailable",
                None,
            )));
        };
        controller.clear_provider_credential(provider, &store, expected_revision)
    })
    .await
    .map_err(|_| {
        ConfigError::runtime(
            Path::new("config.toml"),
            "credential removal task failed",
            None,
        )
    })?;
    if !confirmed {
        return result;
    }
    if let Err(error) = crate::history::refresh_tray_menu(&app) {
        eprintln!("isoboard: could not refresh capture menu after removing a credential: {error}");
    }
    super::commands::emit_profile_result(&app, result)
}

#[tauri::command(rename_all = "camelCase")]
#[specta::specta]
pub async fn delete_unreferenced_inference_credential(
    state: State<'_, SharedConfig>,
    credentials: State<'_, ManagedCredentialsState>,
    kind: String,
    credential_ref: String,
    expected_revision: u64,
) -> Result<ConfigSnapshot, ConfigError> {
    let config = state.inner().clone();
    let store = credentials.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut controller = config.lock().expect("config lock poisoned");
        let config_path = controller.config_path().to_path_buf();
        let Some(store) = store else {
            return Err(controller.remember_error(ConfigError::runtime(
                &config_path,
                "managed credential storage is unavailable",
                None,
            )));
        };
        let kind = parse_managed_kind(&kind).ok_or_else(|| {
            controller.remember_error(ConfigError::validation(
                &config_path,
                "managed credential kind is invalid",
                None,
            ))
        })?;
        controller.delete_unreferenced_credential(kind, &credential_ref, &store, expected_revision)
    })
    .await
    .map_err(|_| {
        ConfigError::runtime(
            Path::new("config.toml"),
            "credential delete task failed",
            None,
        )
    })?
}

fn managed_kind(backend: InferenceBackend) -> ManagedSecretKind {
    match backend.credential_kind() {
        crate::inference_profiles::CredentialKind::None => {
            unreachable!("local does not use credentials")
        }
        crate::inference_profiles::CredentialKind::ApiKey => ManagedSecretKind::ApiKey,
    }
}

fn parse_managed_kind(value: &str) -> Option<ManagedSecretKind> {
    match value {
        "api_key" | "apiKey" => Some(ManagedSecretKind::ApiKey),
        _ => None,
    }
}

impl From<ManagedCredentialReadiness> for InferenceCredentialReadiness {
    fn from(value: ManagedCredentialReadiness) -> Self {
        match value {
            ManagedCredentialReadiness::Unknown => Self::Unknown,
            ManagedCredentialReadiness::Missing => Self::Missing,
            ManagedCredentialReadiness::Ready => Self::Ready,
            ManagedCredentialReadiness::Locked => Self::Locked,
            ManagedCredentialReadiness::Corrupt => Self::Corrupt,
            ManagedCredentialReadiness::Unavailable => Self::Unavailable,
        }
    }
}
