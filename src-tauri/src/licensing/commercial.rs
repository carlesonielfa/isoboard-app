use super::{
    now,
    polar::{self, LicenseProvider, Validation},
};
use crate::{
    managed_credentials::ManagedCredentialStore,
    secret_store::{ManagedSecretKind, ManagedSecretRef},
};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use crate::product_config::PRODUCT;
const STORAGE_ERROR: &str =
    "Could not save license state. Check access to the app data folder and try again.";
const KEYCHAIN_ERROR: &str = "Could not access the saved license key. Unlock Keychain and try again, or enter your key again.";
const KIND: ManagedSecretKind = ManagedSecretKind::LicenseKey;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LicenseRecord {
    reference: ManagedSecretRef,
    masked_key: String,
    license_id: String,
    organization_id: String,
    benefit_id: String,
    covered_major: u32,
    verified_at: u64,
    last_attempt_at: u64,
    granted: bool,
    rejection: Option<String>,
    support_expires_at: Option<u64>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Record {
    license: Option<LicenseRecord>,
    choice_completed: bool,
    pending_cleanup: Vec<ManagedSecretRef>,
}

struct State {
    record: Record,
    load_error: Option<String>,
    verification_error: Option<String>,
    checking: bool,
}

#[derive(Clone, Default, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommercialSnapshot {
    pub(crate) active: bool,
    pub(crate) has_saved_key: bool,
    pub(crate) choice_completed: bool,
    pub(crate) masked_key: Option<String>,
    pub(crate) covered_major: Option<u32>,
    pub(crate) last_verified_at: Option<u64>,
    pub(crate) support_expires_at: Option<u64>,
    pub(crate) support_active: bool,
    pub(crate) checking: bool,
    pub(crate) error: Option<String>,
}

#[derive(Clone, Serialize, Type, tauri_specta::Event)]
pub(crate) struct LicenseChangedEvent {}

type Publisher = Arc<dyn Fn() + Send + Sync>;
pub(crate) struct CommercialService {
    path: PathBuf,
    state: Mutex<State>,
    operation: tokio::sync::Mutex<()>,
    credentials: Option<Arc<ManagedCredentialStore>>,
    provider: Arc<dyn LicenseProvider>,
    publisher: Mutex<Option<Publisher>>,
}

fn current_major() -> u32 {
    // Development package versions 0.x exercise the launch 1.x entitlement.
    env!("CARGO_PKG_VERSION_MAJOR")
        .parse::<u32>()
        .unwrap_or(u32::MAX)
        .max(1)
}

impl CommercialService {
    pub(crate) fn open(
        config_path: &Path,
        credentials: Option<Arc<ManagedCredentialStore>>,
    ) -> Arc<Self> {
        Self::with_provider(config_path, credentials, Arc::new(polar::Polar::new()))
    }

    fn with_provider(
        config_path: &Path,
        credentials: Option<Arc<ManagedCredentialStore>>,
        provider: Arc<dyn LicenseProvider>,
    ) -> Arc<Self> {
        let path = config_path.with_file_name("commercial-license.json");
        let loaded = match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice::<Record>(&bytes).map_err(|_| "Stored license state could not be read. Restore the app data file and restart isoboard.".to_string()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Record::default()),
            Err(_) => Err("License storage could not be read. Check access to the app data folder and restart isoboard.".into()),
        };
        let (record, load_error) = match loaded {
            Ok(record) => (record, None),
            Err(error) => (Record::default(), Some(error)),
        };
        Arc::new(Self {
            path,
            state: Mutex::new(State {
                record,
                load_error,
                verification_error: None,
                checking: false,
            }),
            operation: tokio::sync::Mutex::new(()),
            credentials,
            provider,
            publisher: Mutex::new(None),
        })
    }

    pub(crate) fn snapshot(&self) -> CommercialSnapshot {
        let state = self.state.lock().expect("license lock poisoned");
        let license = state.record.license.as_ref();
        let covered = license.is_some_and(|l| {
            l.organization_id == PRODUCT.polar.organization_id.as_str()
                && l.benefit_id == PRODUCT.polar.benefit_id.as_str()
                && l.covered_major == current_major()
        });
        CommercialSnapshot {
            active: covered && license.is_some_and(|l| l.granted) && state.load_error.is_none(),
            has_saved_key: license.is_some() || !state.record.pending_cleanup.is_empty(),
            choice_completed: state.record.choice_completed,
            masked_key: license.map(|l| l.masked_key.clone()),
            covered_major: license.map(|l| l.covered_major),
            last_verified_at: license.map(|l| l.verified_at),
            support_expires_at: license.and_then(|l| l.support_expires_at),
            support_active: license
                .is_some_and(|l| l.granted && l.support_expires_at.is_some_and(|end| now() < end))
                && state.load_error.is_none(),
            checking: state.checking,
            error: state
                .load_error
                .clone()
                .or_else(|| state.verification_error.clone())
                .or_else(|| license.and_then(|l| l.rejection.clone()))
                .or_else(|| {
                    (license.is_some() && !covered)
                        .then(|| "This license does not cover this version of isoboard.".into())
                }),
        }
    }

    fn save(&self, state: &mut State, record: Record) -> Result<(), String> {
        if let Some(error) = &state.load_error {
            return Err(error.clone());
        }
        let parent = self.path.parent().ok_or(STORAGE_ERROR)?;
        std::fs::create_dir_all(parent).map_err(|_| STORAGE_ERROR)?;
        let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|_| STORAGE_ERROR)?;
        temporary
            .write_all(&serde_json::to_vec(&record).map_err(|_| STORAGE_ERROR)?)
            .map_err(|_| STORAGE_ERROR)?;
        temporary.as_file().sync_all().map_err(|_| STORAGE_ERROR)?;
        temporary.persist(&self.path).map_err(|_| STORAGE_ERROR)?;
        state.record = record;
        Ok(())
    }

    fn publish(&self) {
        if let Some(publish) = self
            .publisher
            .lock()
            .expect("license publisher lock poisoned")
            .as_ref()
        {
            publish();
        }
    }

    pub(crate) fn set_publisher(&self, publisher: Publisher) {
        *self
            .publisher
            .lock()
            .expect("license publisher lock poisoned") = Some(publisher);
    }

    fn finish_check(&self, error: Option<String>) {
        let mut state = self.state.lock().expect("license lock poisoned");
        state.checking = false;
        state.verification_error = error;
        drop(state);
        self.publish();
    }

    pub(crate) async fn activate(self: &Arc<Self>, key: String) -> Result<(), String> {
        let key = key.trim().to_owned();
        if key.is_empty() || key.len() > 1024 || key.chars().any(char::is_control) {
            return Err("Enter a valid license key.".into());
        }
        let _operation = self.operation.lock().await;
        let result = self.provider.validate(key.clone()).await;
        let Validation::Granted {
            license_id,
            support_expires_at,
        } = result
        else {
            return Err(match result {
                Validation::Rejected(message) | Validation::Unavailable(message) => message.into(),
                _ => unreachable!(),
            });
        };
        if current_major() != PRODUCT.licensing.covered_major {
            return Err("This key does not cover this version of isoboard.".into());
        }
        let service = self.clone();
        tokio::task::spawn_blocking(move || service.install(key, license_id, support_expires_at))
            .await
            .map_err(|_| STORAGE_ERROR)??;
        self.publish();
        Ok(())
    }

    fn credentials(&self) -> Result<&Arc<ManagedCredentialStore>, String> {
        self.credentials
            .as_ref()
            .ok_or_else(|| KEYCHAIN_ERROR.into())
    }

    fn install(
        &self,
        key: String,
        license_id: String,
        support_expires_at: Option<u64>,
    ) -> Result<(), String> {
        if let Some(error) = self
            .state
            .lock()
            .expect("license lock poisoned")
            .load_error
            .clone()
        {
            return Err(error);
        }
        let staged = self
            .credentials()?
            .stage(KIND, key.as_bytes())
            .map_err(|_| KEYCHAIN_ERROR)?;
        let suffix: String = key
            .chars()
            .rev()
            .take(4)
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        let time = now();
        let mut state = self.state.lock().expect("license lock poisoned");
        let mut record = state.record.clone();
        if let Some(old) = record.license.take() {
            record.pending_cleanup.push(old.reference);
        }
        record.choice_completed = true;
        record.license = Some(LicenseRecord {
            reference: staged.reference.clone(),
            masked_key: format!("••••-{suffix}"),
            license_id,
            organization_id: PRODUCT.polar.organization_id.as_str().into(),
            benefit_id: PRODUCT.polar.benefit_id.as_str().into(),
            covered_major: PRODUCT.licensing.covered_major,
            verified_at: time,
            last_attempt_at: time,
            granted: true,
            rejection: None,
            support_expires_at,
        });
        if let Err(error) = self.save(&mut state, record) {
            drop(state);
            if self.credentials()?.delete(KIND, &staged.reference).is_err() {
                return Err("Could not save the license or clean up its staged Keychain entry. Your previous license is unchanged.".into());
            }
            return Err(error);
        }
        state.verification_error = None;
        drop(state);
        self.cleanup();
        Ok(())
    }

    fn cleanup(&self) {
        let pending = self
            .state
            .lock()
            .expect("license lock poisoned")
            .record
            .pending_cleanup
            .clone();
        for reference in pending {
            if self
                .credentials()
                .and_then(|store| {
                    store
                        .delete(KIND, &reference)
                        .map_err(|_| KEYCHAIN_ERROR.to_string())
                })
                .is_err()
            {
                self.state.lock().expect("license lock poisoned").verification_error = Some("The license was updated, but an old key could not be removed from Keychain. Unlock Keychain and retry removal.".into());
                continue;
            }
            let mut state = self.state.lock().expect("license lock poisoned");
            let mut record = state.record.clone();
            record.pending_cleanup.retain(|r| r != &reference);
            if let Err(error) = self.save(&mut state, record) {
                state.verification_error = Some(error);
            }
        }
    }

    pub(crate) async fn remove(self: &Arc<Self>) -> Result<(), String> {
        let _operation = self.operation.lock().await;
        let service = self.clone();
        tokio::task::spawn_blocking(move || {
            let mut state = service.state.lock().expect("license lock poisoned");
            let mut record = state.record.clone();
            if let Some(old) = record.license.take() {
                record.pending_cleanup.push(old.reference);
            }
            service.save(&mut state, record)?;
            state.verification_error = None;
            drop(state);
            service.cleanup();
            Ok::<_, String>(())
        })
        .await
        .map_err(|_| STORAGE_ERROR)??;
        self.publish();
        Ok(())
    }

    fn prepare_check(&self, force: bool, time: u64) -> Result<Option<ManagedSecretRef>, String> {
        self.cleanup();
        let mut state = self.state.lock().expect("license lock poisoned");
        if let Some(error) = &state.load_error {
            return Err(error.clone());
        }
        let mut record = state.record.clone();
        let Some(license) = record.license.as_mut() else {
            return Ok(None);
        };
        if !force
            && time.saturating_sub(license.last_attempt_at)
                < PRODUCT.licensing.validation_interval_seconds
        {
            return Ok(None);
        }
        license.last_attempt_at = time;
        let reference = license.reference.clone();
        self.save(&mut state, record)?;
        state.checking = true;
        Ok(Some(reference))
    }

    pub(crate) async fn recheck(self: &Arc<Self>, force: bool) -> Result<(), String> {
        let _operation = if force {
            self.operation.lock().await
        } else {
            let Ok(guard) = self.operation.try_lock() else {
                return Ok(());
            };
            guard
        };
        let service = self.clone();
        let prepared = tokio::task::spawn_blocking(move || {
            let Some(reference) = service.prepare_check(force, now())? else {
                return Ok(None);
            };
            let bytes = service
                .credentials()?
                .get(KIND, &reference)
                .map_err(|_| KEYCHAIN_ERROR)?;
            let key = String::from_utf8(bytes).map_err(|_| KEYCHAIN_ERROR)?;
            Ok::<_, String>(Some(key))
        })
        .await
        .map_err(|_| KEYCHAIN_ERROR)?;
        let key = match prepared {
            Ok(Some(key)) => key,
            Ok(None) => return Ok(()),
            Err(error) => {
                self.finish_check(Some(error));
                return Ok(());
            }
        };
        self.publish();
        let result = self.provider.validate(key).await;
        let service = self.clone();
        let updated = tokio::task::spawn_blocking(move || service.apply_validation(result, now()))
            .await
            .map_err(|_| STORAGE_ERROR);
        match updated {
            Ok(Ok(())) => {}
            Ok(Err(error)) => self.finish_check(Some(error)),
            Err(error) => self.finish_check(Some(error.into())),
        }
        self.publish();
        Ok(())
    }

    fn apply_validation(&self, result: Validation, time: u64) -> Result<(), String> {
        let mut state = self.state.lock().expect("license lock poisoned");
        state.checking = false;
        state.verification_error = None;
        let mut record = state.record.clone();
        let Some(license) = record.license.as_mut() else {
            return Ok(());
        };
        match result {
            Validation::Granted {
                license_id,
                support_expires_at,
            } if license_id == license.license_id => {
                // Missing or malformed support metadata must not erase a verified date.
                // Keep the last verified deadline for this same license.
                if let Some(expires_at) = support_expires_at {
                    license.support_expires_at = Some(expires_at);
                }
                license.granted = true;
                license.rejection = None;
                license.verified_at = time;
            }
            Validation::Granted { .. } => {
                state.verification_error =
                    Some("Polar returned an unexpected license identity. Try again later.".into());
                return Ok(());
            }
            Validation::Rejected(message) => {
                license.granted = false;
                license.rejection = Some(message.into());
            }
            Validation::Unavailable(message) => {
                state.verification_error = Some(message.into());
                return Ok(());
            }
        }
        // Honor a confirmed revocation in memory even if persisting it fails.
        let rejected = !record.license.as_ref().is_some_and(|l| l.granted);
        if let Err(error) = self.save(&mut state, record.clone()) {
            if rejected {
                state.record = record;
            }
            return Err(error);
        }
        Ok(())
    }

    pub(crate) fn start_background(
        self: &Arc<Self>,
        capture_busy: Arc<dyn Fn() -> bool + Send + Sync>,
    ) {
        let service = self.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                if !capture_busy() {
                    let _ = service.recheck(false).await;
                }
                tokio::time::sleep(std::time::Duration::from_secs(
                    PRODUCT.licensing.background_poll_seconds,
                ))
                .await;
            }
        });
    }
}

#[cfg(test)]
#[path = "commercial_tests.rs"]
mod tests;
