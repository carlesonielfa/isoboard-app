//! Checks for, downloads and installs signed app updates.
//!
//! The manifest URL comes from `services.jsonc`; the updater plugin verifies
//! each archive against the public key in `tauri.conf.json`. Only newer
//! releases in the running major version are offered, because a commercial
//! license covers one major series. A downloaded update waits in memory until
//! the user restarts, and never installs while a capture is in progress.

use crate::application_config::SERVICES;
use semver::Version;
use serde::Serialize;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Manager, Runtime, State};
use tauri_plugin_updater::{Update, UpdaterExt};

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type, tauri_specta::Event)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[tauri_specta(event_name = "update-changed")]
pub(crate) enum UpdateStatus {
    Idle,
    Checking,
    UpToDate,
    Downloading { version: String },
    Ready { version: String },
    Installing { version: String },
    Failed { message: String },
}

struct DownloadedUpdate {
    update: Update,
    bytes: Vec<u8>,
}

struct Inner {
    status: UpdateStatus,
    downloaded: Option<DownloadedUpdate>,
    /// The newest version the last successful check found.
    latest: Option<Version>,
    /// Releases from the manifest's changelog.
    remote_changelog: Vec<crate::changelog::parse::Release>,
}

pub(crate) struct UpdateManager {
    state: Mutex<Inner>,
}

/// Whether `candidate` may replace `current`: newer, within the same major.
fn offers(current: &Version, candidate: &Version) -> bool {
    candidate > current && candidate.major == current.major
}

impl UpdateManager {
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(Inner {
                status: UpdateStatus::Idle,
                downloaded: None,
                latest: None,
                remote_changelog: Vec::new(),
            }),
        })
    }

    pub(crate) fn status(&self) -> UpdateStatus {
        self.state
            .lock()
            .expect("update lock poisoned")
            .status
            .clone()
    }

    /// The version waiting for a restart, if any.
    pub(crate) fn ready_version(&self) -> Option<String> {
        match self.status() {
            UpdateStatus::Ready { version } => Some(version),
            _ => None,
        }
    }

    fn set_status<R: Runtime>(&self, app: &AppHandle<R>, status: UpdateStatus) {
        self.state.lock().expect("update lock poisoned").status = status.clone();
        publish(app, status);
    }

    /// Reserves the check, unless one is running or an update is ready.
    fn begin_check(&self) -> bool {
        let mut state = self.state.lock().expect("update lock poisoned");
        match state.status {
            UpdateStatus::Checking
            | UpdateStatus::Downloading { .. }
            | UpdateStatus::Ready { .. }
            | UpdateStatus::Installing { .. } => false,
            UpdateStatus::Idle | UpdateStatus::UpToDate | UpdateStatus::Failed { .. } => {
                state.status = UpdateStatus::Checking;
                true
            }
        }
    }

    /// Checks the manifest and downloads a newer release. Automatic checks
    /// fail silently; a manual check reports the failure.
    pub(crate) async fn check<R: Runtime>(&self, app: &AppHandle<R>, manual: bool) {
        if !self.begin_check() {
            return;
        }
        publish(app, UpdateStatus::Checking);
        match self.find_and_download(app).await {
            Ok(Some(downloaded)) => {
                let version = downloaded.update.version.clone();
                let mut state = self.state.lock().expect("update lock poisoned");
                state.downloaded = Some(downloaded);
                state.status = UpdateStatus::Ready { version };
                let status = state.status.clone();
                drop(state);
                publish(app, status);
            }
            Ok(None) => {
                self.state.lock().expect("update lock poisoned").latest =
                    Some(app.package_info().version.clone());
                self.set_status(app, UpdateStatus::UpToDate);
            }
            Err(error) => {
                eprintln!("isoboard: update check failed: {error}");
                let status = if manual {
                    UpdateStatus::Failed {
                        message: "Could not check for updates. Try again later.".into(),
                    }
                } else {
                    UpdateStatus::Idle
                };
                self.set_status(app, status);
            }
        }
    }

    async fn find_and_download<R: Runtime>(
        &self,
        app: &AppHandle<R>,
    ) -> Result<Option<DownloadedUpdate>, String> {
        let manifest = SERVICES
            .updates
            .manifest_url
            .parse()
            .map_err(|error| format!("invalid update manifest URL: {error}"))?;
        let updater = app
            .updater_builder()
            .endpoints(vec![manifest])
            .map_err(|error| error.to_string())?
            .timeout(Duration::from_secs(
                SERVICES.updates.request_timeout_seconds,
            ))
            .version_comparator(|current, release| offers(&current, &release.version))
            .build()
            .map_err(|error| error.to_string())?;
        let Some(update) = updater.check().await.map_err(|error| error.to_string())? else {
            return Ok(None);
        };
        self.remember_release(&update);
        self.set_status(
            app,
            UpdateStatus::Downloading {
                version: update.version.clone(),
            },
        );
        let bytes = update
            .download(|_, _| {}, || {})
            .await
            .map_err(|error| error.to_string())?;
        Ok(Some(DownloadedUpdate { update, bytes }))
    }

    /// Keeps the found version and its changelog, which the manifest carries
    /// before the archive downloads.
    fn remember_release(&self, update: &Update) {
        let changelog = match update
            .raw_json
            .get("changelog")
            .and_then(|value| value.as_str())
        {
            Some(source) => crate::changelog::parse::parse(source).unwrap_or_else(|error| {
                eprintln!("isoboard: update manifest changelog is invalid: {error}");
                Vec::new()
            }),
            None => Vec::new(),
        };
        let mut state = self.state.lock().expect("update lock poisoned");
        state.latest = Version::parse(&update.version).ok();
        state.remote_changelog = changelog;
    }

    pub(crate) fn changelog(&self, current: &Version) -> Vec<crate::changelog::ChangelogEntry> {
        let state = self.state.lock().expect("update lock poisoned");
        crate::changelog::entries(current, state.latest.as_ref(), &state.remote_changelog)
    }

    /// Replaces the app with the downloaded update and relaunches it.
    pub(crate) fn install_and_restart<R: Runtime>(&self, app: &AppHandle<R>) -> Result<(), String> {
        if app.state::<crate::capture::CaptureCoordinator>().is_busy() {
            return Err("Wait for the current capture to finish, then restart.".into());
        }
        let downloaded = {
            let mut state = self.state.lock().expect("update lock poisoned");
            let Some(downloaded) = state.downloaded.take() else {
                return Err("No update is ready to install.".into());
            };
            state.status = UpdateStatus::Installing {
                version: downloaded.update.version.clone(),
            };
            downloaded
        };
        publish(app, self.status());
        if let Err(error) = downloaded.update.install(&downloaded.bytes) {
            eprintln!("isoboard: update install failed: {error}");
            let message =
                "Could not install the update. Download it from isoboard.app.".to_string();
            self.set_status(
                app,
                UpdateStatus::Failed {
                    message: message.clone(),
                },
            );
            return Err(message);
        }
        app.restart();
    }

    /// Runs automatic checks after launch and then on the configured
    /// interval, while the setting allows them. Development builds only check
    /// on request, so a dev bundle is never replaced by a release.
    pub(crate) fn start_background<R: Runtime>(self: &Arc<Self>, app: AppHandle<R>) {
        if cfg!(debug_assertions) {
            return;
        }
        let manager = self.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_secs(SERVICES.updates.initial_delay_seconds)).await;
            loop {
                let enabled = app
                    .state::<crate::config::SharedConfig>()
                    .lock()
                    .expect("config lock poisoned")
                    .automatic_update_checks();
                if enabled {
                    manager.check(&app, false).await;
                }
                tokio::time::sleep(Duration::from_secs(SERVICES.updates.interval_seconds)).await;
            }
        });
    }
}

fn publish<R: Runtime>(app: &AppHandle<R>, status: UpdateStatus) {
    use tauri_specta::Event;
    if let Err(error) = status.emit(app) {
        eprintln!("isoboard: could not emit update status: {error}");
    }
    if let Err(error) = crate::history::refresh_tray_menu(app) {
        eprintln!("isoboard: could not refresh tray menu: {error}");
    }
}

#[tauri::command]
#[specta::specta]
pub(crate) fn get_update_status(manager: State<'_, Arc<UpdateManager>>) -> UpdateStatus {
    manager.status()
}

#[tauri::command]
#[specta::specta]
pub(crate) fn get_changelog(
    app: AppHandle,
    manager: State<'_, Arc<UpdateManager>>,
) -> Vec<crate::changelog::ChangelogEntry> {
    manager.changelog(&app.package_info().version)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn check_for_updates(
    app: AppHandle,
    manager: State<'_, Arc<UpdateManager>>,
) -> Result<UpdateStatus, String> {
    manager.check(&app, true).await;
    Ok(manager.status())
}

#[tauri::command]
#[specta::specta]
pub(crate) fn install_update(
    app: AppHandle,
    manager: State<'_, Arc<UpdateManager>>,
) -> Result<(), String> {
    manager.install_and_restart(&app)
}

#[cfg(test)]
mod tests {
    use super::offers;
    use semver::Version;

    #[test]
    fn offers_only_newer_releases_in_the_licensed_major_series() {
        let current = Version::new(1, 2, 0);
        assert!(offers(&current, &Version::new(1, 2, 1)));
        assert!(offers(&current, &Version::new(1, 9, 0)));
        assert!(!offers(&current, &Version::new(1, 2, 0)));
        assert!(!offers(&current, &Version::new(1, 1, 9)));
        assert!(!offers(&current, &Version::new(2, 0, 0)));
    }
}
