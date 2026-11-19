use std::sync::Arc;

use super::{
    ManagedSecretKind, ManagedSecretRef, ManagedSecretScope, SecretSlot, SecretStore,
    SecretStoreError,
};

const API_KEYCHAIN_ACCOUNT: &str = "api-key";

fn keychain_identity(slot: SecretSlot) -> (&'static str, &'static str) {
    match slot {
        SecretSlot::ApiOpenAi => ("app.isoboard.dev.api.openai", API_KEYCHAIN_ACCOUNT),
        SecretSlot::ApiAnthropic => ("app.isoboard.dev.api.anthropic", API_KEYCHAIN_ACCOUNT),
        SecretSlot::ApiGemini => ("app.isoboard.dev.api.gemini", API_KEYCHAIN_ACCOUNT),
        SecretSlot::ApiOpenAiCompatible => (
            "app.isoboard.dev.api.openai-compatible",
            API_KEYCHAIN_ACCOUNT,
        ),
    }
}

pub(super) fn store(slot: SecretSlot) -> Result<Arc<dyn SecretStore>, SecretStoreError> {
    Ok(Arc::new(MacKeychainSecretStore::new(slot)?))
}

pub(super) fn managed_store(
    scope: ManagedSecretScope,
    kind: ManagedSecretKind,
    reference: ManagedSecretRef,
) -> Result<Arc<dyn SecretStore>, SecretStoreError> {
    Ok(Arc::new(MacManagedSecretStore::new(
        scope, kind, reference,
    )?))
}

struct MacKeychainSecretStore {
    entry: keyring::Entry,
}

impl MacKeychainSecretStore {
    fn new(slot: SecretSlot) -> Result<Self, SecretStoreError> {
        let (service, account) = keychain_identity(slot);
        let entry = keyring::Entry::new(service, account).map_err(|_| SecretStoreError::Backend)?;
        Ok(Self { entry })
    }
}

impl SecretStore for MacKeychainSecretStore {
    fn get(&self) -> Result<Option<Vec<u8>>, SecretStoreError> {
        match self.entry.get_secret() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(SecretStoreError::Backend),
        }
    }

    fn set(&self, secret: &[u8]) -> Result<(), SecretStoreError> {
        self.entry
            .set_secret(secret)
            .map_err(|_| SecretStoreError::Backend)
    }

    fn delete(&self) -> Result<(), SecretStoreError> {
        match self.entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(SecretStoreError::Backend),
        }
    }
}

struct MacManagedSecretStore {
    scope: ManagedSecretScope,
    kind: ManagedSecretKind,
    reference: ManagedSecretRef,
}

impl MacManagedSecretStore {
    fn new(
        scope: ManagedSecretScope,
        kind: ManagedSecretKind,
        reference: ManagedSecretRef,
    ) -> Result<Self, SecretStoreError> {
        Ok(Self {
            scope,
            kind,
            reference,
        })
    }

    fn entry(&self) -> Result<keyring::Entry, SecretStoreError> {
        let service = format!(
            "app.isoboard.managed.{}",
            self.scope.namespace().keychain_label()
        );
        let account = format!(
            "{}:{}:{}",
            self.scope.installation_id(),
            self.kind.keychain_label(),
            self.reference.as_str()
        );
        keyring::Entry::new(&service, &account).map_err(|_| SecretStoreError::Backend)
    }
}

impl SecretStore for MacManagedSecretStore {
    fn get(&self) -> Result<Option<Vec<u8>>, SecretStoreError> {
        match self.entry()?.get_secret() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(keyring::Error::NoStorageAccess(_)) => Err(SecretStoreError::Locked),
            Err(_) => Err(SecretStoreError::Backend),
        }
    }

    fn set(&self, secret: &[u8]) -> Result<(), SecretStoreError> {
        self.entry()?
            .set_secret(secret)
            .map_err(|error| match error {
                keyring::Error::NoStorageAccess(_) => SecretStoreError::Locked,
                _ => SecretStoreError::Backend,
            })
    }

    fn delete(&self) -> Result<(), SecretStoreError> {
        match self.entry()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(keyring::Error::NoStorageAccess(_)) => Err(SecretStoreError::Locked),
            Err(_) => Err(SecretStoreError::Backend),
        }
    }
}
