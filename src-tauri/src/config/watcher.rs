//! Debounced config-directory watching and event emission.

use super::controller::{ConfigTransition, SharedConfig};
use super::types::{ConfigError, ConfigSnapshot};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::{mpsc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Runtime};
use tauri_specta::Event;

pub struct ConfigWatcher {
    _watcher: Mutex<RecommendedWatcher>,
}

pub(super) fn emit_changed<R: Runtime>(app: &AppHandle<R>, snapshot: &ConfigSnapshot) {
    if let Err(error) = crate::history::refresh_tray_menu_with_config(app, snapshot) {
        eprintln!("isoboard: could not refresh tray menu after config change: {error}");
    }
    if let Err(error) = snapshot.emit(app) {
        eprintln!("isoboard: could not emit config change: {error}");
    }
}

pub(super) fn emit_error<R: Runtime>(app: &AppHandle<R>, error: &ConfigError) {
    if let Err(emit_error) = error.emit(app) {
        eprintln!("isoboard: could not emit config error: {emit_error}");
    }
}

/// Watch the config directory rather than only the file so atomic replacement
/// and editor save strategies continue to produce reloads.
pub fn start_config_watcher<R: Runtime + 'static>(
    app: AppHandle<R>,
    state: SharedConfig,
) -> Result<ConfigWatcher, ConfigError> {
    let path = state
        .lock()
        .expect("config lock poisoned")
        .config_path()
        .to_path_buf();
    let directory = path.parent().ok_or_else(|| {
        ConfigError::io(
            &path,
            format!("config path {} has no parent directory", path.display()),
        )
    })?;
    let watched_path = path.clone();
    let (sender, receiver) = mpsc::channel::<Result<(), String>>();
    let mut watcher =
        notify::recommended_watcher(move |result: notify::Result<notify::Event>| match result {
            Ok(event)
                if event
                    .paths
                    .iter()
                    .any(|event_path| event_path == &watched_path) =>
            {
                let _ = sender.send(Ok(()));
            }
            Ok(_) => {}
            Err(error) => {
                let _ = sender.send(Err(error.to_string()));
            }
        })
        .map_err(|error| ConfigError::io(&path, format!("could not watch config: {error}")))?;
    watcher
        .watch(directory, RecursiveMode::NonRecursive)
        .map_err(|error| ConfigError::io(&path, format!("could not watch config: {error}")))?;

    let worker_app = app.clone();
    let worker_state = state;
    let worker_path = path.clone();
    thread::spawn(move || {
        while let Ok(result) = receiver.recv() {
            if let Err(message) = result {
                let error = {
                    let mut controller = worker_state.lock().expect("config lock poisoned");
                    controller.remember_error(ConfigError::io(&worker_path, message))
                };
                emit_error(&worker_app, &error);
                continue;
            }

            thread::sleep(Duration::from_millis(150));
            while receiver.try_recv().is_ok() {}

            let transition = {
                let mut controller = worker_state.lock().expect("config lock poisoned");
                controller.reload_from_disk()
            };
            match transition {
                ConfigTransition::Changed(snapshot) => emit_changed(&worker_app, &snapshot),
                ConfigTransition::Rejected(error) => emit_error(&worker_app, &error),
                ConfigTransition::Unchanged(_) => {}
            }
        }
    });

    Ok(ConfigWatcher {
        _watcher: Mutex::new(watcher),
    })
}
