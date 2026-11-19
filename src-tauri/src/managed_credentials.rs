//! Lifecycle for app-owned, per-installation credentials.
//!
//! This module owns identity files, opaque references, credential validation,
//! and readiness mapping. Platform storage remains behind the shared raw
//! `SecretStore` interface.

use crate::secret_store::{
    managed_secret_store, ManagedSecretKind, ManagedSecretNamespace, ManagedSecretRef,
    ManagedSecretScope, SecretStore, SecretStoreError,
};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;

const API_KEY_MAX_BYTES: usize = 16 * 1024;
const IDENTITY_FILE_PREFIX: &str = ".isoboard-installation";

/// Sanitized result used by settings and inference readiness checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManagedCredentialReadiness {
    Unknown,
    Missing,
    Ready,
    Locked,
    Corrupt,
    Unavailable,
}

/// Sanitized lifecycle failure. No platform error or credential bytes cross
/// this boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManagedCredentialError {
    UnsupportedPlatform,
    Backend,
    Locked,
    Missing,
    InvalidReference,
    InvalidSecret,
    CorruptIdentity,
    IdentityIo,
    CleanupFailed,
}

impl From<SecretStoreError> for ManagedCredentialError {
    fn from(error: SecretStoreError) -> Self {
        match error {
            SecretStoreError::UnsupportedPlatform => Self::UnsupportedPlatform,
            SecretStoreError::Backend => Self::Backend,
            SecretStoreError::Locked => Self::Locked,
            SecretStoreError::InvalidReference => Self::InvalidReference,
            SecretStoreError::Corrupt => Self::InvalidSecret,
        }
    }
}

/// The generated installation identity and its app-owned storage scope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallationIdentity {
    namespace: ManagedSecretNamespace,
    reference: ManagedSecretRef,
    path: PathBuf,
}

impl InstallationIdentity {
    /// Load the existing namespace-specific identity or create it atomically.
    /// A malformed existing identity is an error; it is never replaced.
    pub fn load_or_create(config_path: &Path) -> Result<Self, ManagedCredentialError> {
        let namespace = ManagedSecretNamespace::current();
        let parent = config_path
            .parent()
            .ok_or(ManagedCredentialError::IdentityIo)?;
        let path = parent.join(format!(
            "{IDENTITY_FILE_PREFIX}-{}.id",
            namespace.keychain_label()
        ));

        match fs::read_to_string(&path) {
            Ok(raw) => return Self::from_raw(namespace, path, &raw),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(ManagedCredentialError::IdentityIo),
        }

        fs::create_dir_all(parent).map_err(|_| ManagedCredentialError::IdentityIo)?;
        let identity = ManagedSecretRef::from_uuid(uuid::Uuid::new_v4());
        let bytes = format!("{identity}\n");
        let temp_path = parent.join(format!(
            "{IDENTITY_FILE_PREFIX}-{}-{}-{}.tmp",
            namespace.keychain_label(),
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        let mut temp = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .map_err(|_| ManagedCredentialError::IdentityIo)?;
        set_private_file_mode(&temp_path)?;
        write_identity(&mut temp, bytes.as_bytes())?;

        // Hard-linking is an atomic no-clobber install. A concurrent launch
        // either wins this link or reads the identity installed by the winner.
        let result = match fs::hard_link(&temp_path, &path) {
            Ok(()) => Ok(Self {
                namespace,
                reference: identity,
                path,
            }),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                Self::load_existing(namespace, path)
            }
            Err(_) => Err(ManagedCredentialError::IdentityIo),
        };
        let _ = fs::remove_file(&temp_path);
        result
    }

    pub fn namespace(&self) -> ManagedSecretNamespace {
        self.namespace
    }

    pub fn scope(&self) -> ManagedSecretScope {
        ManagedSecretScope::new(self.namespace, self.reference.to_string())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn load_existing(
        namespace: ManagedSecretNamespace,
        path: PathBuf,
    ) -> Result<Self, ManagedCredentialError> {
        let raw = fs::read_to_string(&path).map_err(|_| ManagedCredentialError::IdentityIo)?;
        Self::from_raw(namespace, path, &raw)
    }

    fn from_raw(
        namespace: ManagedSecretNamespace,
        path: PathBuf,
        raw: &str,
    ) -> Result<Self, ManagedCredentialError> {
        let value = raw.trim();
        let reference =
            ManagedSecretRef::parse(value).map_err(|_| ManagedCredentialError::CorruptIdentity)?;
        Ok(Self {
            namespace,
            reference,
            path,
        })
    }
}

fn write_identity(file: &mut File, bytes: &[u8]) -> Result<(), ManagedCredentialError> {
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| ManagedCredentialError::IdentityIo)
}

#[cfg(unix)]
fn set_private_file_mode(path: &Path) -> Result<(), ManagedCredentialError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|_| ManagedCredentialError::IdentityIo)
}

#[cfg(not(unix))]
fn set_private_file_mode(_path: &Path) -> Result<(), ManagedCredentialError> {
    Ok(())
}

/// Testable boundary for opening one managed raw store item.
pub trait ManagedStoreFactory: Send + Sync + 'static {
    fn open(
        &self,
        scope: &ManagedSecretScope,
        kind: ManagedSecretKind,
        reference: &ManagedSecretRef,
    ) -> Result<Arc<dyn SecretStore>, SecretStoreError>;
}

struct PlatformStoreFactory;

impl ManagedStoreFactory for PlatformStoreFactory {
    fn open(
        &self,
        scope: &ManagedSecretScope,
        kind: ManagedSecretKind,
        reference: &ManagedSecretRef,
    ) -> Result<Arc<dyn SecretStore>, SecretStoreError> {
        managed_secret_store(scope.clone(), kind, reference.clone())
    }
}

/// Managed credential storage with immutable random references.
pub struct ManagedCredentialStore {
    scope: ManagedSecretScope,
    factory: Arc<dyn ManagedStoreFactory>,
    operation_lock: Mutex<()>,
    cache: Mutex<HashMap<ManagedCredentialCacheKey, Result<Vec<u8>, ManagedCredentialError>>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct ManagedCredentialCacheKey {
    kind: ManagedSecretKind,
    reference: ManagedSecretRef,
}

impl ManagedCredentialStore {
    pub fn open(config_path: &Path) -> Result<Self, ManagedCredentialError> {
        let identity = InstallationIdentity::load_or_create(config_path)?;
        Ok(Self::with_factory(
            identity.scope(),
            Arc::new(PlatformStoreFactory),
        ))
    }

    pub fn with_factory(scope: ManagedSecretScope, factory: Arc<dyn ManagedStoreFactory>) -> Self {
        Self {
            scope,
            factory,
            operation_lock: Mutex::new(()),
            cache: Mutex::new(HashMap::new()),
        }
    }

    pub fn stage(
        &self,
        kind: ManagedSecretKind,
        secret: &[u8],
    ) -> Result<StagedSecret, ManagedCredentialError> {
        validate_secret(secret)?;
        let _operation = self
            .operation_lock
            .lock()
            .expect("credential lock poisoned");

        // A UUID collision is extraordinarily unlikely, but checking before
        // set keeps an existing immutable item from ever being overwritten.
        for _ in 0..3 {
            let reference = ManagedSecretRef::from_uuid(uuid::Uuid::new_v4());
            let store = self.open_store(kind, &reference)?;
            if store.get().map_err(ManagedCredentialError::from)?.is_some() {
                continue;
            }
            if let Err(error) = store.set(secret) {
                let primary = ManagedCredentialError::from(error);
                self.cache_remove(kind, &reference);
                if store.delete().is_err() {
                    return Err(ManagedCredentialError::CleanupFailed);
                }
                return Err(primary);
            }
            self.cache_insert(kind, &reference, secret);
            return Ok(StagedSecret { kind, reference });
        }
        Err(ManagedCredentialError::Backend)
    }

    pub fn get(
        &self,
        kind: ManagedSecretKind,
        reference: &ManagedSecretRef,
    ) -> Result<Vec<u8>, ManagedCredentialError> {
        let _operation = self
            .operation_lock
            .lock()
            .expect("credential lock poisoned");
        if let Some(Ok(secret)) = self.cached(kind, reference) {
            return Ok(secret);
        }
        let result = (|| {
            let secret = self
                .open_store(kind, reference)?
                .get()
                .map_err(ManagedCredentialError::from)?
                .ok_or(ManagedCredentialError::Missing)?;
            validate_secret(&secret)?;
            Ok(secret)
        })();
        self.cache
            .lock()
            .expect("credential cache lock poisoned")
            .insert(Self::cache_key(kind, reference), result.clone());
        result
    }

    pub fn readiness(
        &self,
        kind: ManagedSecretKind,
        reference: &ManagedSecretRef,
    ) -> ManagedCredentialReadiness {
        match self.cached(kind, reference) {
            None => ManagedCredentialReadiness::Unknown,
            Some(Ok(_)) => ManagedCredentialReadiness::Ready,
            Some(Err(ManagedCredentialError::Missing)) => ManagedCredentialReadiness::Missing,
            Some(Err(ManagedCredentialError::Locked)) => ManagedCredentialReadiness::Locked,
            Some(Err(ManagedCredentialError::InvalidSecret))
            | Some(Err(ManagedCredentialError::CorruptIdentity))
            | Some(Err(ManagedCredentialError::InvalidReference)) => {
                ManagedCredentialReadiness::Corrupt
            }
            Some(Err(ManagedCredentialError::UnsupportedPlatform))
            | Some(Err(ManagedCredentialError::Backend))
            | Some(Err(ManagedCredentialError::CleanupFailed))
            | Some(Err(ManagedCredentialError::IdentityIo)) => {
                ManagedCredentialReadiness::Unavailable
            }
        }
    }

    /// Delete only when the configuration controller has already established
    /// that no live profile references this item.
    pub fn delete(
        &self,
        kind: ManagedSecretKind,
        reference: &ManagedSecretRef,
    ) -> Result<(), ManagedCredentialError> {
        let _operation = self
            .operation_lock
            .lock()
            .expect("credential lock poisoned");
        self.cache_remove(kind, reference);
        self.open_store(kind, reference)?
            .delete()
            .map_err(ManagedCredentialError::from)?;
        self.cache
            .lock()
            .expect("credential cache lock poisoned")
            .insert(
                Self::cache_key(kind, reference),
                Err(ManagedCredentialError::Missing),
            );
        Ok(())
    }

    fn open_store(
        &self,
        kind: ManagedSecretKind,
        reference: &ManagedSecretRef,
    ) -> Result<Arc<dyn SecretStore>, ManagedCredentialError> {
        ManagedSecretRef::parse(reference.as_str()).map_err(ManagedCredentialError::from)?;
        self.factory
            .open(&self.scope, kind, reference)
            .map_err(ManagedCredentialError::from)
    }

    fn cache_key(
        kind: ManagedSecretKind,
        reference: &ManagedSecretRef,
    ) -> ManagedCredentialCacheKey {
        ManagedCredentialCacheKey {
            kind,
            reference: reference.clone(),
        }
    }

    /// Observe session state without accessing Keychain or prompting.
    pub(crate) fn cached(
        &self,
        kind: ManagedSecretKind,
        reference: &ManagedSecretRef,
    ) -> Option<Result<Vec<u8>, ManagedCredentialError>> {
        self.cache
            .lock()
            .expect("credential cache lock poisoned")
            .get(&Self::cache_key(kind, reference))
            .cloned()
    }

    fn cache_insert(&self, kind: ManagedSecretKind, reference: &ManagedSecretRef, secret: &[u8]) {
        self.cache
            .lock()
            .expect("credential cache lock poisoned")
            .insert(Self::cache_key(kind, reference), Ok(secret.to_vec()));
    }

    fn cache_remove(&self, kind: ManagedSecretKind, reference: &ManagedSecretRef) {
        self.cache
            .lock()
            .expect("credential cache lock poisoned")
            .remove(&Self::cache_key(kind, reference));
    }
}

/// Handle returned after a successful staged write. It contains no secret
/// bytes and is suitable for the configuration transaction layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StagedSecret {
    pub kind: ManagedSecretKind,
    pub reference: ManagedSecretRef,
}

fn validate_secret(secret: &[u8]) -> Result<(), ManagedCredentialError> {
    if secret.is_empty() || secret.len() > API_KEY_MAX_BYTES {
        return Err(ManagedCredentialError::InvalidSecret);
    }
    let value = std::str::from_utf8(secret).map_err(|_| ManagedCredentialError::InvalidSecret)?;
    if value.trim().is_empty() || value.chars().any(char::is_control) {
        return Err(ManagedCredentialError::InvalidSecret);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    struct MemoryStore {
        value: Mutex<Option<Vec<u8>>>,
        reads: AtomicUsize,
        locked: std::sync::atomic::AtomicBool,
    }

    impl SecretStore for MemoryStore {
        fn get(&self) -> Result<Option<Vec<u8>>, SecretStoreError> {
            self.reads.fetch_add(1, Ordering::Relaxed);
            if self.locked.load(Ordering::Relaxed) {
                return Err(SecretStoreError::Locked);
            }
            Ok(self.value.lock().unwrap().clone())
        }

        fn set(&self, secret: &[u8]) -> Result<(), SecretStoreError> {
            *self.value.lock().unwrap() = Some(secret.to_vec());
            Ok(())
        }

        fn delete(&self) -> Result<(), SecretStoreError> {
            *self.value.lock().unwrap() = None;
            Ok(())
        }
    }

    struct MemoryFactory {
        stores: Mutex<HashMap<(ManagedSecretKind, String), Arc<MemoryStore>>>,
        error: Option<SecretStoreError>,
    }

    impl MemoryFactory {
        fn new() -> Self {
            Self {
                stores: Mutex::new(HashMap::new()),
                error: None,
            }
        }

        fn with_error(error: SecretStoreError) -> Self {
            Self {
                stores: Mutex::new(HashMap::new()),
                error: Some(error),
            }
        }
    }

    impl ManagedStoreFactory for MemoryFactory {
        fn open(
            &self,
            _scope: &ManagedSecretScope,
            kind: ManagedSecretKind,
            reference: &ManagedSecretRef,
        ) -> Result<Arc<dyn SecretStore>, SecretStoreError> {
            if let Some(error) = self.error {
                return Err(error);
            }
            let mut stores = self.stores.lock().unwrap();
            Ok(stores
                .entry((kind, reference.to_string()))
                .or_insert_with(|| {
                    Arc::new(MemoryStore {
                        value: Mutex::new(None),
                        reads: AtomicUsize::new(0),
                        locked: std::sync::atomic::AtomicBool::new(false),
                    })
                })
                .clone())
        }
    }

    fn test_store(factory: Arc<dyn ManagedStoreFactory>) -> ManagedCredentialStore {
        let scope = ManagedSecretScope::new(
            ManagedSecretNamespace::Development,
            "0123456789abcdef0123456789abcdef".to_owned(),
        );
        ManagedCredentialStore::with_factory(scope, factory)
    }

    #[test]
    fn references_are_strict_opaque_lowercase_hex() {
        let valid = ManagedSecretRef::parse("0123456789abcdef0123456789abcdef").unwrap();
        assert_eq!(valid.to_string().len(), ManagedSecretRef::LENGTH);
        assert!(ManagedSecretRef::parse("../config.toml").is_err());
        assert!(ManagedSecretRef::parse("0123456789ABCDEF0123456789abcdef").is_err());
    }

    #[test]
    fn stage_readiness_and_delete_use_raw_store_boundary() {
        let store = test_store(Arc::new(MemoryFactory::new()));
        let staged = store
            .stage(ManagedSecretKind::ApiKey, b"api-key-value")
            .unwrap();
        assert_eq!(
            store.readiness(ManagedSecretKind::ApiKey, &staged.reference),
            ManagedCredentialReadiness::Ready
        );
        assert_eq!(
            store
                .get(ManagedSecretKind::ApiKey, &staged.reference)
                .unwrap(),
            b"api-key-value"
        );
        store
            .delete(ManagedSecretKind::ApiKey, &staged.reference)
            .unwrap();
        assert_eq!(
            store.readiness(ManagedSecretKind::ApiKey, &staged.reference),
            ManagedCredentialReadiness::Missing
        );
    }

    #[test]
    fn readiness_sanitizes_locked_and_corrupt_values() {
        let locked = test_store(Arc::new(MemoryFactory::with_error(
            SecretStoreError::Locked,
        )));
        let reference = ManagedSecretRef::parse("0123456789abcdef0123456789abcdef").unwrap();
        assert_eq!(
            locked.readiness(ManagedSecretKind::ApiKey, &reference),
            ManagedCredentialReadiness::Unknown
        );
        assert_eq!(
            locked.get(ManagedSecretKind::ApiKey, &reference),
            Err(ManagedCredentialError::Locked)
        );
        assert_eq!(
            locked.readiness(ManagedSecretKind::ApiKey, &reference),
            ManagedCredentialReadiness::Locked
        );

        let factory = Arc::new(MemoryFactory::new());
        let store = test_store(factory.clone());
        let staged = store
            .stage(ManagedSecretKind::ApiKey, b"raw-api-key")
            .unwrap();
        assert_eq!(
            store.readiness(ManagedSecretKind::ApiKey, &staged.reference),
            ManagedCredentialReadiness::Ready
        );
        let raw_store = factory
            .stores
            .lock()
            .unwrap()
            .get(&(ManagedSecretKind::ApiKey, staged.reference.to_string()))
            .unwrap()
            .clone();
        *raw_store.value.lock().unwrap() = Some(Vec::new());
        let cold_store = test_store(factory);
        assert_eq!(
            cold_store.readiness(ManagedSecretKind::ApiKey, &staged.reference),
            ManagedCredentialReadiness::Unknown
        );
        assert_eq!(raw_store.reads.load(Ordering::Relaxed), 1);
        assert_eq!(
            cold_store.get(ManagedSecretKind::ApiKey, &staged.reference),
            Err(ManagedCredentialError::InvalidSecret)
        );
        assert_eq!(
            cold_store.readiness(ManagedSecretKind::ApiKey, &staged.reference),
            ManagedCredentialReadiness::Corrupt
        );
    }

    #[test]
    fn passive_checks_never_read_and_explicit_access_retries_denial() {
        let factory = Arc::new(MemoryFactory::new());
        let writer = test_store(factory.clone());
        let staged = writer.stage(ManagedSecretKind::ApiKey, b"secret").unwrap();
        let raw = factory
            .stores
            .lock()
            .unwrap()
            .get(&(ManagedSecretKind::ApiKey, staged.reference.to_string()))
            .unwrap()
            .clone();
        let reader = test_store(factory);
        raw.locked.store(true, Ordering::Relaxed);
        for _ in 0..3 {
            assert_eq!(
                reader.readiness(staged.kind, &staged.reference),
                ManagedCredentialReadiness::Unknown
            );
        }
        assert_eq!(raw.reads.load(Ordering::Relaxed), 1);
        assert_eq!(
            reader.get(staged.kind, &staged.reference),
            Err(ManagedCredentialError::Locked)
        );
        for _ in 0..3 {
            assert_eq!(
                reader.readiness(staged.kind, &staged.reference),
                ManagedCredentialReadiness::Locked
            );
        }
        assert_eq!(raw.reads.load(Ordering::Relaxed), 2);
        raw.locked.store(false, Ordering::Relaxed);
        assert_eq!(
            reader.get(staged.kind, &staged.reference).unwrap(),
            b"secret"
        );
        assert_eq!(
            reader.get(staged.kind, &staged.reference).unwrap(),
            b"secret"
        );
        assert_eq!(raw.reads.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn cached_reads_are_coherent_with_deletes() {
        let factory = Arc::new(MemoryFactory::new());
        let store = test_store(factory.clone());
        let staged = store
            .stage(ManagedSecretKind::ApiKey, b"first-secret")
            .unwrap();
        let raw_store = factory
            .stores
            .lock()
            .unwrap()
            .get(&(ManagedSecretKind::ApiKey, staged.reference.to_string()))
            .unwrap()
            .clone();
        assert_eq!(raw_store.reads.load(Ordering::Relaxed), 1);

        assert_eq!(
            store
                .get(ManagedSecretKind::ApiKey, &staged.reference)
                .unwrap(),
            b"first-secret"
        );
        assert_eq!(raw_store.reads.load(Ordering::Relaxed), 1);

        store
            .delete(ManagedSecretKind::ApiKey, &staged.reference)
            .unwrap();
        assert_eq!(
            store.get(ManagedSecretKind::ApiKey, &staged.reference),
            Err(ManagedCredentialError::Missing)
        );
        assert_eq!(raw_store.reads.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn identity_is_namespace_specific_and_corruption_is_not_regenerated() {
        let directory = tempfile::tempdir().unwrap();
        let config = directory.path().join("config.toml");
        let first = InstallationIdentity::load_or_create(&config).unwrap();
        let second = InstallationIdentity::load_or_create(&config).unwrap();
        assert_eq!(first, second);
        fs::write(first.path(), "not-an-id\n").unwrap();
        assert_eq!(
            InstallationIdentity::load_or_create(&config),
            Err(ManagedCredentialError::CorruptIdentity)
        );
    }
}
