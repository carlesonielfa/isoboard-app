use super::*;
use crate::{
    managed_credentials::ManagedStoreFactory,
    secret_store::{ManagedSecretNamespace, ManagedSecretScope, SecretStore, SecretStoreError},
};
use std::{
    collections::{HashMap, VecDeque},
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

#[derive(Default)]
struct MemorySecret {
    bytes: Mutex<Option<Vec<u8>>>,
    fail_delete: AtomicBool,
}
impl SecretStore for MemorySecret {
    fn get(&self) -> Result<Option<Vec<u8>>, SecretStoreError> {
        Ok(self.bytes.lock().unwrap().clone())
    }
    fn set(&self, bytes: &[u8]) -> Result<(), SecretStoreError> {
        *self.bytes.lock().unwrap() = Some(bytes.to_vec());
        Ok(())
    }
    fn delete(&self) -> Result<(), SecretStoreError> {
        if self.fail_delete.load(Ordering::SeqCst) {
            return Err(SecretStoreError::Locked);
        }
        *self.bytes.lock().unwrap() = None;
        Ok(())
    }
}
#[derive(Default)]
struct Factory {
    stores: Mutex<HashMap<String, Arc<MemorySecret>>>,
}
impl ManagedStoreFactory for Factory {
    fn open(
        &self,
        _: &ManagedSecretScope,
        kind: ManagedSecretKind,
        reference: &ManagedSecretRef,
    ) -> Result<Arc<dyn SecretStore>, SecretStoreError> {
        assert_eq!(kind, ManagedSecretKind::LicenseKey);
        Ok(self
            .stores
            .lock()
            .unwrap()
            .entry(reference.to_string())
            .or_default()
            .clone())
    }
}
#[derive(Default)]
struct Provider {
    results: Mutex<VecDeque<Validation>>,
    calls: AtomicUsize,
}
impl LicenseProvider for Provider {
    fn validate(&self, _: String) -> polar::ValidationFuture<'_> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let result = self
            .results
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(granted);
        Box::pin(async move { result })
    }
}
fn granted() -> Validation {
    Validation::Granted {
        license_id: "d63fb336-39be-4541-bfe7-a61b8fdbe828".into(),
        support_expires_at: Some(1),
    }
}
fn fixture() -> (
    tempfile::TempDir,
    Arc<CommercialService>,
    Arc<Factory>,
    Arc<Provider>,
) {
    let directory = tempfile::tempdir().unwrap();
    let factory = Arc::new(Factory::default());
    let credentials = Arc::new(ManagedCredentialStore::with_factory(
        ManagedSecretScope::new(
            ManagedSecretNamespace::Development,
            "test-installation".into(),
        ),
        factory.clone(),
    ));
    let provider = Arc::new(Provider::default());
    let service = CommercialService::with_provider(
        &directory.path().join("config.toml"),
        Some(credentials),
        provider.clone(),
    );
    (directory, service, factory, provider)
}

#[tokio::test]
async fn support_refresh_is_independent_of_commercial_permission() {
    let (_directory, service, _factory, provider) = fixture();
    provider
        .results
        .lock()
        .unwrap()
        .push_back(Validation::Granted {
            license_id: "d63fb336-39be-4541-bfe7-a61b8fdbe828".into(),
            support_expires_at: None,
        });
    service.activate("TEST-support".into()).await.unwrap();
    assert!(service.snapshot().active);
    assert_eq!(service.snapshot().support_expires_at, None);
    let end = now() + 86400;
    service
        .apply_validation(
            Validation::Granted {
                license_id: "d63fb336-39be-4541-bfe7-a61b8fdbe828".into(),
                support_expires_at: Some(end),
            },
            now(),
        )
        .unwrap();
    assert!(service.snapshot().support_active);
    assert!(service.snapshot().active);
    service
        .apply_validation(Validation::Unavailable("Offline"), now())
        .unwrap();
    assert_eq!(service.snapshot().support_expires_at, Some(end));
    assert!(service.snapshot().support_active);
    service
        .apply_validation(
            Validation::Granted {
                license_id: "d63fb336-39be-4541-bfe7-a61b8fdbe828".into(),
                support_expires_at: None,
            },
            now(),
        )
        .unwrap();
    assert!(service.snapshot().active);
    assert!(service.snapshot().support_active);
    assert_eq!(service.snapshot().support_expires_at, Some(end));
}

#[tokio::test]
async fn activation_survives_reopen_offline_and_removal_does_not_contact_polar() {
    let (directory, service, factory, provider) = fixture();
    service.activate("TEST-license-one".into()).await.unwrap();
    assert!(service.snapshot().active);
    assert!(!service.snapshot().support_active);
    assert_eq!(service.snapshot().support_expires_at, Some(1));
    assert!(service.snapshot().choice_completed);
    let bytes = std::fs::read_to_string(&service.path).unwrap();
    assert!(!bytes.contains("TEST-license-one"));
    let reopened = CommercialService::with_provider(
        &directory.path().join("config.toml"),
        service.credentials.clone(),
        provider.clone(),
    );
    provider
        .results
        .lock()
        .unwrap()
        .push_back(Validation::Unavailable("Offline"));
    reopened.recheck(true).await.unwrap();
    assert!(reopened.snapshot().active);
    assert_eq!(reopened.snapshot().support_expires_at, Some(1));
    assert_eq!(reopened.snapshot().error.as_deref(), Some("Offline"));
    let calls = provider.calls.load(Ordering::SeqCst);
    reopened.remove().await.unwrap();
    assert!(!reopened.snapshot().active);
    assert!(reopened.snapshot().choice_completed);
    assert!(!reopened.snapshot().has_saved_key);
    assert_eq!(provider.calls.load(Ordering::SeqCst), calls);
    assert!(factory
        .stores
        .lock()
        .unwrap()
        .values()
        .all(|s| s.bytes.lock().unwrap().is_none()));
    let reopened = CommercialService::with_provider(
        &directory.path().join("config.toml"),
        service.credentials.clone(),
        provider,
    );
    assert!(!reopened.snapshot().active);
}

#[tokio::test]
async fn failed_replacement_preserves_existing_license_and_failed_save_cleans_staging() {
    let (_directory, service, factory, provider) = fixture();
    service.activate("TEST-original".into()).await.unwrap();
    let original = service.snapshot().masked_key;
    provider
        .results
        .lock()
        .unwrap()
        .push_back(Validation::Rejected("Invalid key"));
    assert!(service.activate("TEST-invalid".into()).await.is_err());
    assert!(service.snapshot().active);
    assert_eq!(service.snapshot().masked_key, original);
    let persisted = std::fs::read(&service.path).unwrap();
    std::fs::remove_file(&service.path).unwrap();
    std::fs::create_dir(&service.path).unwrap();
    assert!(service.activate("TEST-replacement".into()).await.is_err());
    assert!(service.snapshot().active);
    assert_eq!(service.snapshot().masked_key, original);
    assert_eq!(
        factory
            .stores
            .lock()
            .unwrap()
            .values()
            .filter(|s| s.bytes.lock().unwrap().is_some())
            .count(),
        1
    );
    std::fs::remove_dir(&service.path).unwrap();
    std::fs::write(&service.path, persisted).unwrap();
}

#[tokio::test]
async fn daily_check_is_throttled_and_confirmed_rejection_survives_restart() {
    let (directory, service, _, provider) = fixture();
    service.activate("TEST-license".into()).await.unwrap();
    service.recheck(false).await.unwrap();
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    {
        let mut state = service.state.lock().unwrap();
        state.record.license.as_mut().unwrap().last_attempt_at =
            now() - PRODUCT.licensing.validation_interval_seconds;
    }
    provider
        .results
        .lock()
        .unwrap()
        .push_back(Validation::Unavailable("Offline"));
    service.recheck(false).await.unwrap();
    service.recheck(false).await.unwrap();
    assert_eq!(provider.calls.load(Ordering::SeqCst), 2);
    assert!(service.snapshot().active);
    provider
        .results
        .lock()
        .unwrap()
        .push_back(Validation::Rejected("Revoked"));
    service.recheck(true).await.unwrap();
    assert!(!service.snapshot().active);
    assert_eq!(service.snapshot().error.as_deref(), Some("Revoked"));
    let reopened = CommercialService::with_provider(
        &directory.path().join("config.toml"),
        service.credentials.clone(),
        provider.clone(),
    );
    assert!(!reopened.snapshot().active);
    reopened.recheck(true).await.unwrap();
    assert!(reopened.snapshot().active);
}

#[tokio::test]
async fn failed_keychain_cleanup_remains_retryable_without_restoring_entitlement() {
    let (_directory, service, factory, _) = fixture();
    service.activate("TEST-license".into()).await.unwrap();
    let secret = factory
        .stores
        .lock()
        .unwrap()
        .values()
        .next()
        .unwrap()
        .clone();
    secret.fail_delete.store(true, Ordering::SeqCst);
    service.remove().await.unwrap();
    assert!(!service.snapshot().active);
    assert!(service.snapshot().has_saved_key);
    assert!(service.snapshot().error.is_some());
    secret.fail_delete.store(false, Ordering::SeqCst);
    service.remove().await.unwrap();
    assert!(!service.snapshot().has_saved_key);
    assert!(service.snapshot().error.is_none());
}

#[tokio::test]
async fn commercial_entitlement_overrides_trial_without_starting_or_resetting_it() {
    let (directory, service, _, _) = fixture();
    let mut trial =
        crate::licensing::LicenseStore::open_at(&directory.path().join("config.toml"), true, 100);
    service.activate("TEST-license".into()).await.unwrap();
    let snapshot = crate::licensing::with_commercial(trial.snapshot(100), &service);
    assert!(!snapshot.needs_choice);
    assert_eq!(snapshot.started_at, None);
    trial.continue_without_key(100).unwrap();
    trial.notice_due = true;
    let snapshot = crate::licensing::with_commercial(
        trial.snapshot(100 + crate::product_config::evaluation_seconds()),
        &service,
    );
    assert!(!snapshot.show_expiry_notice);
    service.remove().await.unwrap();
    assert_eq!(trial.snapshot(200).started_at, Some(100));
}

#[cfg(target_os = "macos")]
#[tokio::test]
#[ignore = "Requires an authorized production license key on stdin; validates once and uses isolated native Keychain storage"]
async fn production_validation_and_native_keychain_lifecycle() {
    let mut key = String::new();
    std::io::stdin()
        .read_line(&mut key)
        .expect("provide key on stdin");
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.toml");
    let credentials = Arc::new(ManagedCredentialStore::open(&path).unwrap());
    let service = CommercialService::open(&path, Some(credentials));
    service
        .activate(key.trim().to_owned())
        .await
        .expect("production validation and secure storage");
    let active = service.snapshot().active;
    let reference = service
        .state
        .lock()
        .unwrap()
        .record
        .license
        .as_ref()
        .unwrap()
        .reference
        .clone();
    let cold_credentials = Arc::new(ManagedCredentialStore::open(&path).unwrap());
    let reopened = CommercialService::open(&path, Some(cold_credentials.clone()));
    let read_back_matches =
        cold_credentials.get(KIND, &reference).unwrap() == key.trim().as_bytes();
    let no_plaintext = !std::fs::read_to_string(&service.path)
        .unwrap()
        .contains(key.trim());
    let offline_active = reopened.snapshot().active;
    reopened
        .remove()
        .await
        .expect("remove only isolated local license");
    let removed = cold_credentials.get(KIND, &reference).is_err();
    assert!(active && read_back_matches && no_plaintext && offline_active && removed);
}
