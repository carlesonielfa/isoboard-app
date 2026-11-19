//! Tauri runtime adapters and startup wiring.

use super::controller::{ConfigController, ShortcutRuntime};
use super::types::{ConfigError, ShortcutRecordingState};
use super::watcher::{emit_changed, emit_error, start_config_watcher};
use super::ManagedCredentialsState;
use crate::managed_credentials::ManagedCredentialStore;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::{App, AppHandle, Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState};

/// The native adapter keeps the Tauri plugin behind the testable runtime seam.
struct TauriShortcutRuntime<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> ShortcutRuntime for TauriShortcutRuntime<R> {
    fn register(&mut self, shortcut: &str) -> Result<(), String> {
        self.app
            .global_shortcut()
            .register(shortcut)
            .map_err(|error| error.to_string())
    }

    fn unregister(&mut self, shortcut: &str) -> Result<(), String> {
        self.app
            .global_shortcut()
            .unregister(shortcut)
            .map_err(|error| error.to_string())
    }
}

/// Build the official Tauri global-shortcut plugin and route pressed events to
/// the capture owner. Release events are deliberately ignored. The callback
/// receives the native hotkey ID so config can resolve its binding while the
/// config lock is held only for that short lookup.
pub fn global_shortcut_plugin<R, F>(on_capture: F) -> tauri::plugin::TauriPlugin<R>
where
    R: Runtime,
    F: Fn(&AppHandle<R>, u32) + Send + Sync + 'static,
{
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(move |app, shortcut: &Shortcut, event: ShortcutEvent| {
            if event.state() == ShortcutState::Pressed
                && !app.state::<ShortcutRecordingState>().is_recording()
            {
                on_capture(app, shortcut.id());
            }
        })
        .build()
}

/// Initialize config state, activate the last-good shortcut, and install the
/// external-edit watcher. Configuration errors are emitted but do not abort
/// startup when an in-memory default can still be used.
pub fn setup<R: Runtime + 'static>(app: &mut App<R>) -> Result<(), ConfigError> {
    let config_path = resolve_config_path(app)?;
    let runtime = Box::new(TauriShortcutRuntime {
        app: app.handle().clone(),
    });
    let (controller, startup_error) = ConfigController::load_or_default(config_path, runtime)?;
    let managed_credentials = ManagedCredentialStore::open(controller.config_path())
        .ok()
        .map(Arc::new);
    let state = Arc::new(Mutex::new(controller));
    app.manage(state.clone());
    app.manage(ManagedCredentialsState(managed_credentials));

    if let Some(error) = startup_error {
        let error = {
            let mut controller = state.lock().expect("config lock poisoned");
            controller.remember_error(error)
        };
        emit_error(app.handle(), &error);
    }

    let activation = {
        let mut controller = state.lock().expect("config lock poisoned");
        controller.activate()
    };
    match activation {
        Ok(snapshot) => emit_changed(app.handle(), &snapshot),
        Err(error) => emit_error(app.handle(), &error),
    }

    match start_config_watcher(app.handle().clone(), state.clone()) {
        Ok(watcher) => {
            app.manage(watcher);
        }
        Err(error) => {
            let error = {
                let mut controller = state.lock().expect("config lock poisoned");
                controller.remember_error(error)
            };
            emit_error(app.handle(), &error);
        }
    }

    Ok(())
}

/// Keep development runs isolated from the user's packaged-app settings.
/// Release builds always use Tauri's platform app-config directory.
fn resolve_config_path<R: Runtime>(_app: &App<R>) -> Result<PathBuf, ConfigError> {
    #[cfg(debug_assertions)]
    if let Some(path) = std::env::var_os("ISOBOARD_CONFIG_PATH") {
        if !path.is_empty() {
            return Ok(PathBuf::from(path));
        }
    }

    #[cfg(debug_assertions)]
    {
        // macOS Quit & Reopen may relaunch without the dev runner's environment.
        Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("the desktop crate has a workspace parent")
            .join(".dev/config.toml"))
    }

    #[cfg(not(debug_assertions))]
    {
        let config_dir = _app.path().app_config_dir().map_err(|error| {
            ConfigError::io(
                Path::new("config.toml"),
                format!("could not resolve app config directory: {error}"),
            )
        })?;
        Ok(config_dir.join("config.toml"))
    }
}
