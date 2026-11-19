//! Development-only provider API-key handling for the opt-in API probe.

use std::sync::Arc;

pub use crate::inference::Provider;
pub use crate::secret_store::{SecretSlot, SecretStore, SecretStoreError};

pub const MAX_API_KEY_BYTES: usize = 16 * 1024;

fn slot_for_provider(provider: Provider) -> SecretSlot {
    match provider {
        Provider::OpenAi => SecretSlot::ApiOpenAi,
        Provider::Anthropic => SecretSlot::ApiAnthropic,
        Provider::Gemini => SecretSlot::ApiGemini,
        Provider::OpenAiCompatible => SecretSlot::ApiOpenAiCompatible,
    }
}

/// Validate an API key at the API credential boundary. The shared store only
/// persists bytes and intentionally does not apply this policy.
pub fn validate_api_key(secret: &[u8]) -> Result<(), SecretStoreError> {
    if secret.is_empty() || secret.len() > MAX_API_KEY_BYTES {
        return Err(SecretStoreError::Corrupt);
    }
    let value = std::str::from_utf8(secret).map_err(|_| SecretStoreError::Corrupt)?;
    if value.trim().is_empty() || value.chars().any(char::is_control) {
        return Err(SecretStoreError::Corrupt);
    }
    Ok(())
}

pub fn api_credential_store(provider: Provider) -> Result<Arc<dyn SecretStore>, SecretStoreError> {
    crate::secret_store::secret_store(slot_for_provider(provider))
}
