use super::controller::{ConfigController, ShortcutRuntime};
use crate::managed_credentials::{ManagedCredentialStore, ManagedStoreFactory};
use crate::secret_store::{
    ManagedSecretKind, ManagedSecretNamespace, ManagedSecretRef, ManagedSecretScope, SecretStore,
    SecretStoreError,
};
use std::collections::HashMap;
#[cfg(unix)]
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct TestRuntime;
impl ShortcutRuntime for TestRuntime {
    fn register(&mut self, _: &str) -> Result<(), String> {
        Ok(())
    }
    fn unregister(&mut self, _: &str) -> Result<(), String> {
        Ok(())
    }
}
#[derive(Default)]
struct CredentialState {
    values: HashMap<String, Vec<u8>>,
    fail_delete: bool,
}
struct MemoryStore {
    state: Arc<Mutex<CredentialState>>,
    reference: String,
}
impl SecretStore for MemoryStore {
    fn get(&self) -> Result<Option<Vec<u8>>, SecretStoreError> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .values
            .get(&self.reference)
            .cloned())
    }
    fn set(&self, secret: &[u8]) -> Result<(), SecretStoreError> {
        self.state
            .lock()
            .unwrap()
            .values
            .insert(self.reference.clone(), secret.to_vec());
        Ok(())
    }
    fn delete(&self) -> Result<(), SecretStoreError> {
        let mut state = self.state.lock().unwrap();
        if state.fail_delete {
            return Err(SecretStoreError::Backend);
        }
        state.values.remove(&self.reference);
        Ok(())
    }
}
struct MemoryFactory {
    state: Arc<Mutex<CredentialState>>,
}
impl ManagedStoreFactory for MemoryFactory {
    fn open(
        &self,
        _: &ManagedSecretScope,
        _: ManagedSecretKind,
        reference: &ManagedSecretRef,
    ) -> Result<Arc<dyn SecretStore>, SecretStoreError> {
        Ok(Arc::new(MemoryStore {
            state: self.state.clone(),
            reference: reference.to_string(),
        }))
    }
}
fn controller(path: &Path) -> ConfigController {
    let (controller, error) =
        ConfigController::load_or_default(path, Box::new(TestRuntime)).unwrap();
    assert!(error.is_none());
    controller
}
fn credentials() -> (ManagedCredentialStore, Arc<Mutex<CredentialState>>) {
    let state = Arc::new(Mutex::new(CredentialState::default()));
    let store = ManagedCredentialStore::with_factory(
        ManagedSecretScope::new(
            ManagedSecretNamespace::Development,
            "transaction-test-installation".into(),
        ),
        Arc::new(MemoryFactory {
            state: state.clone(),
        }),
    );
    (store, state)
}
fn save_api(
    controller: &mut ConfigController,
    credentials: &ManagedCredentialStore,
) -> crate::config::types::ConfigSnapshot {
    controller
        .save_provider_with_secret(
            "openai_api".into(),
            None,
            b"old-secret",
            credentials,
            controller.snapshot().revision,
        )
        .unwrap()
}

#[test]
fn stale_revision_rejects_before_staging_any_secret() {
    let dir = tempfile::tempdir().unwrap();
    let mut controller = controller(&dir.path().join("config.toml"));
    let (credentials, state) = credentials();
    let error = controller
        .save_provider_with_secret(
            "openai_api".into(),
            None,
            b"must-not-stage",
            &credentials,
            controller.snapshot().revision + 1,
        )
        .unwrap_err();
    assert_eq!(error.kind, super::types::ConfigErrorKind::Conflict);
    assert!(state.lock().unwrap().values.is_empty());
}

#[test]
fn replacing_shared_provider_credential_keeps_old_secret() {
    let dir = tempfile::tempdir().unwrap();
    let mut controller = controller(&dir.path().join("config.toml"));
    let (credentials, state) = credentials();
    let first = save_api(&mut controller, &credentials);
    let old = first.inference.providers[0].credential_ref.clone().unwrap();
    let second = controller
        .save_provider_with_secret(
            "openai_api".into(),
            None,
            b"new-secret",
            &credentials,
            first.revision,
        )
        .unwrap();
    let new = second.inference.providers[0]
        .credential_ref
        .clone()
        .unwrap();
    let state = state.lock().unwrap();
    assert_ne!(old, new);
    assert_eq!(
        state.values.get(&old).map(Vec::as_slice),
        Some(b"old-secret".as_slice())
    );
    assert_eq!(
        state.values.get(&new).map(Vec::as_slice),
        Some(b"new-secret".as_slice())
    );
}

#[test]
fn deleting_referenced_provider_credential_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let mut controller = controller(&dir.path().join("config.toml"));
    let (credentials, state) = credentials();
    let first = save_api(&mut controller, &credentials);
    let reference = first.inference.providers[0].credential_ref.clone().unwrap();
    let error = controller
        .delete_unreferenced_credential(
            ManagedSecretKind::ApiKey,
            &reference,
            &credentials,
            first.revision,
        )
        .unwrap_err();
    assert_eq!(error.kind, super::types::ConfigErrorKind::Validation);
    assert!(state.lock().unwrap().values.contains_key(&reference));
}

#[cfg(unix)]
#[test]
fn provider_save_rolls_back_config_and_staged_credential_on_write_failure() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let mut controller = controller(&path);
    let (credentials, state) = credentials();
    let first = save_api(&mut controller, &credentials);
    let old = fs::read_to_string(&path).unwrap();
    fs::set_permissions(path.parent().unwrap(), fs::Permissions::from_mode(0o500)).unwrap();
    let error = controller
        .save_provider_with_secret(
            "openai_api".into(),
            None,
            b"new-secret",
            &credentials,
            first.revision,
        )
        .unwrap_err();
    fs::set_permissions(path.parent().unwrap(), fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(error.kind, super::types::ConfigErrorKind::Io);
    assert_eq!(fs::read_to_string(&path).unwrap(), old);
    assert_eq!(state.lock().unwrap().values.len(), 1);
}

#[cfg(unix)]
#[test]
fn provider_save_reports_staged_cleanup_failure_after_write_failure() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let mut controller = controller(&path);
    let (credentials, state) = credentials();
    let first = save_api(&mut controller, &credentials);
    state.lock().unwrap().fail_delete = true;
    fs::set_permissions(path.parent().unwrap(), fs::Permissions::from_mode(0o500)).unwrap();
    let error = controller
        .save_provider_with_secret(
            "openai_api".into(),
            None,
            b"new-secret",
            &credentials,
            first.revision,
        )
        .unwrap_err();
    fs::set_permissions(path.parent().unwrap(), fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(error.kind, super::types::ConfigErrorKind::Io);
    assert!(error.message.contains("staged credential cleanup failed"));
    assert_eq!(state.lock().unwrap().values.len(), 2);
}

#[test]
fn blank_key_connects_only_openai_compatible_without_storing_a_secret() {
    let dir = tempfile::tempdir().unwrap();
    let mut controller = controller(&dir.path().join("config.toml"));
    let (credentials, state) = credentials();
    let error = controller
        .save_provider_with_secret(
            "openai_api".into(),
            None,
            b"",
            &credentials,
            controller.snapshot().revision,
        )
        .unwrap_err();
    assert_eq!(error.kind, super::types::ConfigErrorKind::Validation);
    let error = controller
        .save_provider_with_secret(
            "openai_compatible".into(),
            None,
            b"",
            &credentials,
            controller.snapshot().revision,
        )
        .unwrap_err();
    assert_eq!(error.kind, super::types::ConfigErrorKind::Validation);

    let keyed = controller
        .save_provider_with_secret(
            "openai_compatible".into(),
            Some("http://localhost:11434/v1".into()),
            b"local-key",
            &credentials,
            controller.snapshot().revision,
        )
        .unwrap();
    let old = keyed.inference.providers[0].credential_ref.clone().unwrap();
    let keyless = controller
        .save_provider_with_secret(
            "openai_compatible".into(),
            Some("http://localhost:11434/v1".into()),
            b"  ",
            &credentials,
            keyed.revision,
        )
        .unwrap();
    let provider = &keyless.inference.providers[0];
    assert_eq!(
        provider.endpoint.as_deref(),
        Some("http://localhost:11434/v1")
    );
    assert!(provider.credential_ref.is_none());
    let state = state.lock().unwrap();
    assert_eq!(state.values.len(), 1);
    assert!(state.values.contains_key(&old));
}
