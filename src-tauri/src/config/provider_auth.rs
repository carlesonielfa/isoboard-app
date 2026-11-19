//! Shared fixed-provider credential transactions.

use super::controller::ConfigController;
use super::types::{ConfigError, ConfigSnapshot};
use crate::inference_profiles::{validate_endpoint, InferenceBackend, ProviderConfig};
use crate::managed_credentials::{ManagedCredentialError, ManagedCredentialStore};
use crate::secret_store::{ManagedSecretKind, ManagedSecretRef};
use std::path::Path;

impl ConfigController {
    pub(crate) fn current_provider(&self, backend: InferenceBackend) -> Option<&ProviderConfig> {
        self.current.inference.providers.get(backend.id())
    }

    pub fn delete_unreferenced_credential(
        &mut self,
        kind: ManagedSecretKind,
        reference: &str,
        credentials: &ManagedCredentialStore,
        expected_revision: u64,
    ) -> Result<ConfigSnapshot, ConfigError> {
        self.check_profile_revision(expected_revision)?;
        let reference = ManagedSecretRef::parse(reference).map_err(|_| {
            self.remember_error(ConfigError::validation(
                &self.path,
                "managed credential reference is invalid",
                None,
            ))
        })?;
        let reference_text = reference.to_string();
        if self
            .current
            .inference
            .providers
            .values()
            .any(|provider| provider.credential_ref.as_deref() == Some(reference_text.as_str()))
        {
            return Err(self.remember_error(ConfigError::validation(
                &self.path,
                "managed credential is still referenced by a provider",
                None,
            )));
        }
        credentials.delete(kind, &reference).map_err(|_| {
            self.remember_error(ConfigError::runtime(
                &self.path,
                "could not remove managed inference credential",
                None,
            ))
        })?;
        Ok(self.snapshot())
    }

    pub fn save_provider_with_secret(
        &mut self,
        provider: String,
        endpoint: Option<String>,
        secret: &[u8],
        credentials: &ManagedCredentialStore,
        expected_revision: u64,
    ) -> Result<ConfigSnapshot, ConfigError> {
        self.check_profile_revision(expected_revision)?;
        let backend = parse_inference_backend(&provider, &self.path)
            .map_err(|error| self.remember_error(error))?;
        if backend.credential_kind() != crate::inference_profiles::CredentialKind::ApiKey {
            return Err(self.remember_error(ConfigError::validation(
                &self.path,
                "provider does not accept API keys",
                None,
            )));
        }
        let endpoint = normalize_optional(endpoint);
        if let Some(value) = endpoint.as_deref() {
            validate_endpoint(backend, value).map_err(|message| {
                self.remember_error(ConfigError::validation(&self.path, message, None))
            })?;
        }
        if backend.credential_optional() && secret_is_blank(secret) {
            return self.save_keyless_provider(backend, endpoint);
        }
        let staged = match credentials.stage(ManagedSecretKind::ApiKey, secret) {
            Ok(staged) => staged,
            Err(ManagedCredentialError::InvalidSecret) => {
                return Err(self.remember_error(ConfigError::validation(
                    &self.path,
                    "managed inference credential is invalid",
                    None,
                )))
            }
            Err(ManagedCredentialError::CleanupFailed) => {
                return Err(self.remember_error(ConfigError::runtime(
                    &self.path,
                    "managed inference credential staging failed and cleanup failed",
                    None,
                )))
            }
            Err(_) => {
                return Err(self.remember_error(ConfigError::runtime(
                    &self.path,
                    "could not stage managed inference credential",
                    None,
                )))
            }
        };
        let mut candidate = self.current.clone();
        candidate.inference.providers.insert(
            backend.id().into(),
            ProviderConfig {
                endpoint,
                credential_ref: Some(staged.reference.to_string()),
            },
        );
        if let Err(error) = candidate.validate(&self.path) {
            return Err(self.cleanup_staged_error(credentials, staged, error));
        }
        match self.apply_candidate(candidate, true) {
            Ok(snapshot) => Ok(snapshot),
            Err(error) => Err(self.cleanup_staged_error(credentials, staged, error)),
        }
    }

    /// Save a provider whose API key is optional without writing an empty
    /// secret. Like a key replacement, the previous key stays stored so a
    /// capture that already froze its reference can finish.
    fn save_keyless_provider(
        &mut self,
        backend: InferenceBackend,
        endpoint: Option<String>,
    ) -> Result<ConfigSnapshot, ConfigError> {
        let mut candidate = self.current.clone();
        candidate.inference.providers.insert(
            backend.id().into(),
            ProviderConfig {
                endpoint,
                credential_ref: None,
            },
        );
        self.apply_candidate(candidate, true)
    }

    fn cleanup_staged_error(
        &mut self,
        credentials: &ManagedCredentialStore,
        staged: crate::managed_credentials::StagedSecret,
        mut error: ConfigError,
    ) -> ConfigError {
        if credentials.delete(staged.kind, &staged.reference).is_err() {
            error.message =
                format!("{}; staged credential cleanup failed", error.message).into_boxed_str();
        }
        self.remember_error(error)
    }
}

fn parse_inference_backend(value: &str, path: &Path) -> Result<InferenceBackend, ConfigError> {
    InferenceBackend::parse(value.trim()).ok_or_else(|| {
        ConfigError::validation(
            path,
            format!("unsupported inference provider {:?}", value),
            None,
        )
    })
}

fn secret_is_blank(secret: &[u8]) -> bool {
    std::str::from_utf8(secret).is_ok_and(|value| value.trim().is_empty())
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim().to_string();
        (!value.is_empty()).then_some(value)
    })
}
