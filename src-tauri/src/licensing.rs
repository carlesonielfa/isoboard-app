//! Local evaluation permissions and cached commercial licensing.
pub(crate) mod commercial;
mod polar;
mod pricing;
use commercial::{CommercialService, CommercialSnapshot};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::Arc;
use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::product_config::evaluation_seconds;

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    started_at: Option<u64>,
    last_seen: u64,
    notice_dismissed: bool,
}

#[derive(Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LicenseSnapshot {
    commercial: CommercialSnapshot,
    needs_choice: bool,
    evaluation: EvaluationStatus,
    started_at: Option<u64>,
    expires_at: Option<u64>,
    show_expiry_notice: bool,
    error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub(crate) enum EvaluationStatus {
    NotStarted,
    Active,
    Expired,
    Unavailable,
}

pub(crate) struct LicenseStore {
    path: PathBuf,
    record: Record,
    error: Option<String>,
    notice_due: bool,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|v| v.as_secs())
        .unwrap_or(0)
}

impl LicenseStore {
    pub(crate) fn open(config_path: &Path, ordinary_launch: bool) -> Self {
        Self::open_at(config_path, ordinary_launch, now())
    }

    fn open_at(config_path: &Path, ordinary_launch: bool, time: u64) -> Self {
        // The development config lives in .dev, isolating its evaluation too.
        let path = config_path.with_file_name("licensing.json");
        let loaded = match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice::<Record>(&bytes).map_err(|_| {
                "Stored evaluation dates could not be read. Personal use remains available."
                    .to_string()
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Record::default()),
            Err(_) => Err(
                "Evaluation storage could not be read. Check access to the app data folder."
                    .to_string(),
            ),
        };
        let (record, error) = match loaded {
            Ok(record) => (record, None),
            Err(error) => (Record::default(), Some(error)),
        };
        let mut store = Self {
            path,
            record,
            error,
            notice_due: false,
        };
        store.notice_due = ordinary_launch
            && store.status(time) == EvaluationStatus::Expired
            && !store.record.notice_dismissed;
        store
    }

    fn status(&self, time: u64) -> EvaluationStatus {
        if self.error.is_some() {
            return EvaluationStatus::Unavailable;
        }
        let Some(start) = self.record.started_at else {
            return if self.record.last_seen == 0 && !self.record.notice_dismissed {
                EvaluationStatus::NotStarted
            } else {
                EvaluationStatus::Unavailable
            };
        };
        let Some(end) = start.checked_add(evaluation_seconds()) else {
            return EvaluationStatus::Unavailable;
        };
        // Keep dates representable by the frontend and reject inconsistent records.
        const MAX_TIMESTAMP: u64 = 253_402_300_799;
        if start == 0
            || end > MAX_TIMESTAMP
            || self.record.last_seen > MAX_TIMESTAMP
            || self.record.last_seen < start
        {
            return EvaluationStatus::Unavailable;
        }
        // An observed expiry stays expired even if the clock subsequently moves back.
        if self.record.last_seen >= end {
            return EvaluationStatus::Expired;
        }
        if time < start || time < self.record.last_seen {
            return EvaluationStatus::Unavailable;
        }
        if time >= end {
            EvaluationStatus::Expired
        } else {
            EvaluationStatus::Active
        }
    }

    fn snapshot(&self, time: u64) -> LicenseSnapshot {
        let evaluation = self.status(time);
        let error = self.error.clone().or_else(|| (evaluation == EvaluationStatus::Unavailable).then(|| "Evaluation dates are unavailable. Check your system clock. Personal use remains available.".into()));
        LicenseSnapshot {
            commercial: CommercialSnapshot::default(),
            needs_choice: evaluation == EvaluationStatus::NotStarted,
            started_at: self.record.started_at,
            expires_at: self
                .record
                .started_at
                .and_then(|start| start.checked_add(evaluation_seconds())),
            show_expiry_notice: self.notice_due && !self.record.notice_dismissed,
            evaluation,
            error,
        }
    }

    fn save(&mut self, record: Record) -> Result<(), String> {
        let parent = self
            .path
            .parent()
            .ok_or("Evaluation storage has no parent folder")?;
        std::fs::create_dir_all(parent).map_err(|_| "Could not create evaluation storage")?;
        let mut temporary = tempfile::NamedTempFile::new_in(parent)
            .map_err(|_| "Could not write evaluation storage")?;
        let bytes = serde_json::to_vec(&record).map_err(|_| "Could not encode evaluation dates")?;
        temporary
            .write_all(&bytes)
            .map_err(|_| "Could not write evaluation dates")?;
        temporary
            .as_file()
            .sync_all()
            .map_err(|_| "Could not save evaluation dates")?;
        temporary
            .persist(&self.path)
            .map_err(|_| "Could not save evaluation dates. Check access to the app data folder.")?;
        self.record = record;
        Ok(())
    }

    fn refresh(&mut self, time: u64) -> LicenseSnapshot {
        if matches!(
            self.status(time),
            EvaluationStatus::Active | EvaluationStatus::Expired
        ) && time > self.record.last_seen
        {
            let mut record = self.record.clone();
            record.last_seen = time;
            if let Err(error) = self.save(record) {
                let mut snapshot = self.snapshot(time);
                snapshot.error = Some(error);
                return snapshot;
            }
        }
        self.snapshot(time)
    }

    fn continue_without_key(&mut self, time: u64) -> Result<LicenseSnapshot, String> {
        if self.status(time) == EvaluationStatus::NotStarted {
            if time == 0 || time > 253_402_300_799 - evaluation_seconds() {
                return Err("Check your system clock before starting the evaluation.".into());
            }
            self.save(Record {
                started_at: Some(time),
                last_seen: time,
                notice_dismissed: false,
            })?;
        }
        Ok(self.refresh(time))
    }
}

pub(crate) type SharedLicensing = Mutex<LicenseStore>;

fn with_commercial(mut snapshot: LicenseSnapshot, service: &CommercialService) -> LicenseSnapshot {
    snapshot.commercial = service.snapshot();
    snapshot.needs_choice &= !snapshot.commercial.choice_completed;
    if snapshot.commercial.active {
        snapshot.needs_choice = false;
        snapshot.show_expiry_notice = false;
        snapshot.error = None;
    }
    snapshot
}

#[tauri::command]
#[specta::specta]
pub(crate) fn get_license_status(
    state: tauri::State<'_, SharedLicensing>,
    commercial: tauri::State<'_, Arc<CommercialService>>,
) -> Result<LicenseSnapshot, String> {
    let snapshot = state
        .lock()
        .map_err(|_| "License state is unavailable")?
        .refresh(now());
    Ok(with_commercial(snapshot, &commercial))
}

#[tauri::command]
#[specta::specta]
pub(crate) fn continue_without_license_key(
    state: tauri::State<'_, SharedLicensing>,
    commercial: tauri::State<'_, Arc<CommercialService>>,
) -> Result<LicenseSnapshot, String> {
    let snapshot = state
        .lock()
        .map_err(|_| "License state is unavailable")?
        .continue_without_key(now())?;
    Ok(with_commercial(snapshot, &commercial))
}

#[tauri::command]
#[specta::specta]
pub(crate) fn dismiss_evaluation_notice(
    state: tauri::State<'_, SharedLicensing>,
    commercial: tauri::State<'_, Arc<CommercialService>>,
) -> Result<LicenseSnapshot, String> {
    let mut store = state.lock().map_err(|_| "License state is unavailable")?;
    let mut record = store.record.clone();
    if store.notice_due {
        record.notice_dismissed = true;
        store.save(record)?;
        store.notice_due = false;
    }
    Ok(with_commercial(store.snapshot(now()), &commercial))
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn activate_commercial_license(
    key: String,
    state: tauri::State<'_, SharedLicensing>,
    commercial: tauri::State<'_, Arc<CommercialService>>,
    capture: tauri::State<'_, crate::capture::CaptureCoordinator>,
) -> Result<LicenseSnapshot, String> {
    if capture.is_busy() {
        return Err("Finish the current capture before activating a license.".into());
    }
    commercial.activate(key).await?;
    get_license_status(state, commercial)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn remove_commercial_license(
    state: tauri::State<'_, SharedLicensing>,
    commercial: tauri::State<'_, Arc<CommercialService>>,
) -> Result<LicenseSnapshot, String> {
    commercial.remove().await?;
    get_license_status(state, commercial)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn recheck_commercial_license(
    state: tauri::State<'_, SharedLicensing>,
    commercial: tauri::State<'_, Arc<CommercialService>>,
    capture: tauri::State<'_, crate::capture::CaptureCoordinator>,
) -> Result<LicenseSnapshot, String> {
    if capture.is_busy() {
        return Err("Finish the current capture before checking a license.".into());
    }
    commercial.recheck(true).await?;
    get_license_status(state, commercial)
}

/// Price of one commercial seat, or `None` when Polar cannot provide it.
#[tauri::command]
#[specta::specta]
pub(crate) async fn get_commercial_seat_price() -> Option<pricing::SeatPrice> {
    pricing::fetch_seat_price().await
}

pub(crate) fn needs_attention(store: &LicenseStore, commercial: &CommercialService) -> bool {
    let snapshot = with_commercial(store.snapshot(now()), commercial);
    snapshot.needs_choice || snapshot.show_expiry_notice
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn evaluation_is_once_and_expires_at_exact_boundary() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut store = LicenseStore::open_at(&path, true, 100);
        assert!(store.snapshot(100).needs_choice);
        store.continue_without_key(100).unwrap();
        let mut reopened = LicenseStore::open_at(&path, true, 200);
        assert_eq!(
            reopened.continue_without_key(200).unwrap().started_at,
            Some(100)
        );
        assert_eq!(
            reopened.status(100 + evaluation_seconds() - 1),
            EvaluationStatus::Active
        );
        assert_eq!(
            reopened.refresh(100 + evaluation_seconds()).evaluation,
            EvaluationStatus::Expired
        );
        assert_eq!(reopened.status(150), EvaluationStatus::Expired);
        assert!(
            !reopened
                .snapshot(100 + evaluation_seconds())
                .show_expiry_notice
        );
        assert!(!LicenseStore::open_at(&path, false, 100 + evaluation_seconds()).notice_due);
        let mut expired = LicenseStore::open_at(&path, true, 100 + evaluation_seconds());
        assert!(expired.notice_due);
        let mut record = expired.record.clone();
        record.notice_dismissed = true;
        expired.save(record).unwrap();
        assert!(!LicenseStore::open_at(&path, true, 100 + evaluation_seconds()).notice_due);
    }
    #[test]
    fn corrupt_dates_and_clock_rollback_never_restart_evaluation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut store = LicenseStore::open_at(&path, true, 100);
        store.continue_without_key(100).unwrap();
        store.refresh(200);
        assert_eq!(
            store.continue_without_key(150).unwrap().evaluation,
            EvaluationStatus::Unavailable
        );
        assert_eq!(store.record.started_at, Some(100));
        std::fs::write(&store.path, b"broken").unwrap();
        let mut corrupt = LicenseStore::open_at(&path, true, 300);
        assert!(!corrupt.snapshot(300).needs_choice);
        assert_eq!(
            corrupt.continue_without_key(300).unwrap().evaluation,
            EvaluationStatus::Unavailable
        );
        assert_eq!(std::fs::read(&store.path).unwrap(), b"broken");
    }
    #[test]
    fn invalid_records_and_failed_writes_do_not_grant_a_trial() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut store = LicenseStore::open_at(&path, true, 100);
        std::fs::create_dir(&store.path).unwrap();
        assert!(store.continue_without_key(100).is_err());
        assert_eq!(store.record.started_at, None);
        std::fs::remove_dir(&store.path).unwrap();
        for start in [0, u64::MAX, 253_402_300_799] {
            store.record = Record {
                started_at: Some(start),
                last_seen: start,
                notice_dismissed: false,
            };
            assert_eq!(store.status(100), EvaluationStatus::Unavailable);
            assert!(!store.snapshot(100).needs_choice);
        }
    }
}
