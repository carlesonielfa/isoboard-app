//! Version-one inference and provider configuration.
//!
//! Profiles own model settings that select a shared provider configuration.
//! Provider configuration owns transport details and the opaque managed
//! credential reference; no secret bytes are represented here.

use crate::application_config::APPLICATION;
use crate::secret_store::ManagedSecretRef;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::net::IpAddr;
use std::path::Path;
use url::Url;

const MODEL_MAX_BYTES: usize = 256;
const ENDPOINT_MAX_BYTES: usize = 2 * 1024;

/// The closed set of provider wire protocols supported by version one.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum InferenceBackend {
    Local,
    OpenaiApi,
    AnthropicApi,
    GeminiApi,
    OpenaiCompatible,
}

impl InferenceBackend {
    pub const ALL: [Self; 5] = [
        Self::OpenaiApi,
        Self::AnthropicApi,
        Self::GeminiApi,
        Self::OpenaiCompatible,
        Self::Local,
    ];

    pub const fn id(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::OpenaiApi => "openai_api",
            Self::AnthropicApi => "anthropic_api",
            Self::GeminiApi => "gemini_api",
            Self::OpenaiCompatible => "openai_compatible",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "local" => Some(Self::Local),
            "openai_api" => Some(Self::OpenaiApi),
            "anthropic_api" => Some(Self::AnthropicApi),
            "gemini_api" => Some(Self::GeminiApi),
            "openai_compatible" => Some(Self::OpenaiCompatible),
            _ => None,
        }
    }

    pub const fn supports_custom_endpoint(self) -> bool {
        matches!(self, Self::OpenaiCompatible)
    }

    /// Self-hosted OpenAI-compatible servers often run without authentication,
    /// so their API key is optional and an endpoint alone connects the provider.
    pub const fn credential_optional(self) -> bool {
        matches!(self, Self::OpenaiCompatible)
    }

    pub const fn credential_kind(self) -> CredentialKind {
        match self {
            Self::Local => CredentialKind::None,
            Self::OpenaiApi | Self::AnthropicApi | Self::GeminiApi | Self::OpenaiCompatible => {
                CredentialKind::ApiKey
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum CredentialKind {
    None,
    ApiKey,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize, specta::Type)]
#[serde(deny_unknown_fields)]
#[serde(default)]
pub struct InferenceConfig {
    pub providers: BTreeMap<String, ProviderConfig>,
    pub provisioned_providers: Vec<InferenceBackend>,
}

/// Reusable transport and authentication configuration for one fixed backend.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, specta::Type)]
#[serde(deny_unknown_fields)]
pub struct ProviderConfig {
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub credential_ref: Option<String>,
}

/// A model selection that references a reusable provider configuration.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, specta::Type)]
#[serde(deny_unknown_fields)]
pub struct ModelSettings {
    pub provider: InferenceBackend,
    pub model: String,
    #[serde(default, skip_serializing_if = "is_none_or_empty_options")]
    pub options: Option<InferenceOptions>,
}

fn is_none_or_empty_options(options: &Option<InferenceOptions>) -> bool {
    options.as_ref().is_none_or(InferenceOptions::is_empty)
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize, specta::Type)]
#[serde(deny_unknown_fields)]
#[serde(default)]
pub struct InferenceOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<ReasoningLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_timeout_seconds: Option<u32>,
}

/// Camel-case options payload accepted by the frontend profile editor.
/// Persisted configuration continues to use [`InferenceOptions`] directly.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, specta::Type)]
#[serde(deny_unknown_fields)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct InferenceOptionsDraft {
    pub temperature: Option<f64>,
    pub max_output_tokens: Option<u32>,
    pub reasoning: Option<ReasoningLevel>,
    pub request_timeout_seconds: Option<u32>,
}

/// Complete camel-case profile payload accepted by the frontend editor.
#[derive(Clone, Debug, Deserialize, PartialEq, specta::Type)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ModelSettingsDraft {
    pub provider: InferenceBackend,
    pub model: String,
    pub options: Option<InferenceOptionsDraft>,
}

impl From<InferenceOptionsDraft> for InferenceOptions {
    fn from(draft: InferenceOptionsDraft) -> Self {
        Self {
            temperature: draft.temperature,
            max_output_tokens: draft.max_output_tokens,
            reasoning: draft.reasoning,
            request_timeout_seconds: draft.request_timeout_seconds,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningLevel {
    None,
    Minimal,
    Low,
    Medium,
    High,
    Xhigh,
    Max,
    On,
    Off,
}

impl InferenceOptions {
    pub fn is_empty(&self) -> bool {
        self.temperature.is_none()
            && self.max_output_tokens.is_none()
            && self.reasoning.is_none()
            && self.request_timeout_seconds.is_none()
    }

    pub fn validate(&self) -> Result<(), String> {
        if let Some(temperature) = self.temperature {
            if !temperature.is_finite()
                || !(APPLICATION.limits.temperature_min..=APPLICATION.limits.temperature_max)
                    .contains(&temperature)
            {
                return Err(format!(
                    "inference options temperature must be finite and between {} and {}",
                    APPLICATION.limits.temperature_min, APPLICATION.limits.temperature_max
                ));
            }
        }
        if self
            .max_output_tokens
            .is_some_and(|tokens| tokens == 0 || tokens > APPLICATION.limits.max_output_tokens)
        {
            return Err(format!(
                "inference options max_output_tokens must be between 1 and {}",
                APPLICATION.limits.max_output_tokens
            ));
        }
        if let Some(seconds) = self.request_timeout_seconds {
            if !(APPLICATION.limits.request_timeout_min_seconds
                ..=APPLICATION.limits.request_timeout_max_seconds)
                .contains(&seconds)
            {
                return Err(format!(
                    "inference options request_timeout_seconds must be between {} and {}",
                    APPLICATION.limits.request_timeout_min_seconds,
                    APPLICATION.limits.request_timeout_max_seconds
                ));
            }
        }
        Ok(())
    }
}

impl ModelSettings {
    pub fn normalize_options(&mut self) {
        if self
            .options
            .as_ref()
            .is_some_and(InferenceOptions::is_empty)
        {
            self.options = None;
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InferenceSnapshot {
    pub providers: Vec<ProviderSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSnapshot {
    pub id: String,
    pub backend: InferenceBackend,
    pub endpoint: Option<String>,
    pub credential_ref: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ModelSettingsSnapshot {
    pub provider: InferenceBackend,
    pub model: String,
    pub options: Option<InferenceOptionsSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_model_download_available: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_model_downloaded: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_model_supports_text: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InferenceOptionsSnapshot {
    pub temperature: Option<f64>,
    pub max_output_tokens: Option<u32>,
    pub reasoning: Option<ReasoningLevel>,
    pub request_timeout_seconds: Option<u32>,
}

impl From<&InferenceOptions> for InferenceOptionsSnapshot {
    fn from(options: &InferenceOptions) -> Self {
        Self {
            temperature: options.temperature,
            max_output_tokens: options.max_output_tokens,
            reasoning: options.reasoning,
            request_timeout_seconds: options.request_timeout_seconds,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InferenceBackendMetadata {
    pub id: InferenceBackend,
    pub credential_kind: CredentialKind,
    pub credential_optional: bool,
    pub supports_custom_endpoint: bool,
    pub built_in_profiles: Vec<crate::builtin_inference_profiles::BuiltinProfileMetadata>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum InferenceCredentialReadiness {
    Unknown,
    Missing,
    Ready,
    Locked,
    Corrupt,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InferenceCredentialReadinessSnapshot {
    pub provider: InferenceBackend,
    pub credential_ref: Option<String>,
    pub readiness: InferenceCredentialReadiness,
}

pub fn backend_metadata() -> Vec<InferenceBackendMetadata> {
    all_backend_metadata()
        .into_iter()
        .filter(|m| m.id != InferenceBackend::Local || crate::local_models::supported())
        .collect()
}

pub(crate) fn all_backend_metadata() -> Vec<InferenceBackendMetadata> {
    InferenceBackend::ALL
        .into_iter()
        .map(|backend| InferenceBackendMetadata {
            id: backend,
            credential_kind: backend.credential_kind(),
            credential_optional: backend.credential_optional(),
            supports_custom_endpoint: backend.supports_custom_endpoint(),
            built_in_profiles: crate::builtin_inference_profiles::metadata_for(backend),
        })
        .collect()
}

impl InferenceConfig {
    pub fn validate(&self, path: &Path) -> Result<(), String> {
        let mut provider_ids = HashSet::new();
        for (id, provider) in &self.providers {
            let backend = InferenceBackend::parse(id)
                .ok_or_else(|| format!("unsupported provider {:?}", id))?;
            if !provider_ids.insert(backend) {
                return Err(format!("duplicate provider {:?}", id));
            }
            if let Some(endpoint) = provider.endpoint.as_deref() {
                validate_endpoint(backend, endpoint)?;
            } else if backend == InferenceBackend::OpenaiCompatible {
                return Err("openai_compatible providers require an endpoint".into());
            }
            if let Some(reference) = provider.credential_ref.as_deref() {
                if backend == InferenceBackend::Local {
                    return Err("local providers do not accept credentials".into());
                }
                validate_credential_ref(reference)?;
            }
        }

        let _ = path;
        Ok(())
    }
}

pub fn validate_model(model: &str) -> Result<(), String> {
    if model.trim().is_empty() {
        return Err("inference model must not be empty".into());
    }
    if model.len() > MODEL_MAX_BYTES || model.chars().any(char::is_control) {
        return Err("inference model is too long or contains control characters".into());
    }
    Ok(())
}

pub fn validate_credential_ref(reference: &str) -> Result<(), String> {
    ManagedSecretRef::parse(reference)
        .map(|_| ())
        .map_err(|_| "credential_ref must be a 32-character lowercase hexadecimal reference".into())
}

pub fn validate_endpoint(backend: InferenceBackend, value: &str) -> Result<(), String> {
    if !backend.supports_custom_endpoint() {
        return Err(format!(
            "backend {} does not accept a custom endpoint",
            backend.id()
        ));
    }
    if value.is_empty() || value.len() > ENDPOINT_MAX_BYTES {
        return Err("inference endpoint is empty or too long".into());
    }
    let url = Url::parse(value).map_err(|_| "inference endpoint is invalid".to_string())?;
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("inference endpoint must not contain credentials, query, or fragment".into());
    }
    let host = url
        .host_str()
        .ok_or_else(|| "inference endpoint must contain a host".to_string())?;
    let loopback = host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<IpAddr>()
            .map(|ip| ip.is_loopback())
            .unwrap_or(false);
    match url.scheme() {
        "https" => Ok(()),
        "http" if loopback => Ok(()),
        _ => Err("inference endpoint must use HTTPS (HTTP is allowed only for localhost)".into()),
    }
}

impl From<&InferenceConfig> for InferenceSnapshot {
    fn from(config: &InferenceConfig) -> Self {
        Self {
            providers: config
                .providers
                .iter()
                .map(|(id, provider)| ProviderSnapshot {
                    id: id.clone(),
                    backend: InferenceBackend::parse(id).expect("validated provider backend"),
                    endpoint: provider.endpoint.clone(),
                    credential_ref: provider.credential_ref.clone(),
                })
                .collect(),
        }
    }
}

impl ModelSettingsDraft {
    pub fn into_settings(self) -> Result<ModelSettings, String> {
        let provider = self.provider;
        let mut settings = ModelSettings {
            provider,
            model: self.model.trim().to_owned(),
            options: self.options.map(Into::into),
        };
        settings.normalize_options();
        settings.validate()?;
        Ok(settings)
    }
}
impl ModelSettings {
    pub fn validate(&self) -> Result<(), String> {
        validate_model(&self.model)?;
        if let Some(options) = &self.options {
            options.validate()?;
        }
        Ok(())
    }
    pub fn snapshot(&self, path: &Path) -> ModelSettingsSnapshot {
        let local = self.provider == InferenceBackend::Local;
        ModelSettingsSnapshot {
            provider: self.provider,
            model: self.model.clone(),
            options: self.options.as_ref().map(InferenceOptionsSnapshot::from),
            local_model_download_available: local.then(|| {
                crate::local_models::supported()
                    && crate::local_models::definition(&self.model).is_some()
            }),
            local_model_downloaded: local.then(|| {
                crate::local_models::ModelStore::for_config(path)
                    .resolve(&self.model)
                    .is_some()
            }),
            local_model_supports_text: local.then(|| {
                crate::local_models::definition(&self.model).is_some_and(|m| m.supports_text)
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_tags_are_closed_and_stable() {
        assert_eq!(
            InferenceBackend::parse("openai_api"),
            Some(InferenceBackend::OpenaiApi)
        );
        assert_eq!(InferenceBackend::parse("openai"), None);
    }

    #[test]
    fn custom_endpoint_is_restricted_to_safe_urls() {
        assert!(
            validate_endpoint(InferenceBackend::OpenaiCompatible, "https://example.com/v1").is_ok()
        );
        assert!(validate_endpoint(
            InferenceBackend::OpenaiCompatible,
            "http://localhost:8080/v1"
        )
        .is_ok());
        assert!(validate_endpoint(
            InferenceBackend::OpenaiCompatible,
            "https://example.com/v1?key=secret"
        )
        .is_err());
        assert!(validate_endpoint(InferenceBackend::OpenaiApi, "https://example.com/v1").is_err());
    }

    #[test]
    fn profile_validation_accepts_fixed_provider_identity_without_auth() {
        let settings = ModelSettings {
            provider: InferenceBackend::OpenaiApi,
            model: "vision-model".into(),
            options: None,
        };
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn inference_options_validate_bounds_and_accept_reasoning_levels() {
        let options = InferenceOptions {
            temperature: Some(0.7),
            max_output_tokens: Some(512),
            reasoning: Some(ReasoningLevel::Xhigh),
            request_timeout_seconds: Some(120),
        };
        assert!(options.validate().is_ok());
        for reasoning in [
            ReasoningLevel::None,
            ReasoningLevel::Minimal,
            ReasoningLevel::Low,
            ReasoningLevel::Medium,
            ReasoningLevel::High,
            ReasoningLevel::Xhigh,
            ReasoningLevel::Max,
            ReasoningLevel::On,
            ReasoningLevel::Off,
        ] {
            assert!(InferenceOptions {
                reasoning: Some(reasoning),
                ..InferenceOptions::default()
            }
            .validate()
            .is_ok());
        }
        assert!(InferenceOptions {
            temperature: Some(-0.1),
            ..InferenceOptions::default()
        }
        .validate()
        .is_err());
        assert!(InferenceOptions {
            temperature: Some(2.1),
            ..InferenceOptions::default()
        }
        .validate()
        .is_err());
        assert!(InferenceOptions {
            temperature: Some(f64::NAN),
            ..InferenceOptions::default()
        }
        .validate()
        .is_err());
        assert!(InferenceOptions {
            max_output_tokens: Some(0),
            ..InferenceOptions::default()
        }
        .validate()
        .is_err());
        assert!(InferenceOptions {
            request_timeout_seconds: Some(0),
            ..InferenceOptions::default()
        }
        .validate()
        .is_err());
        assert!(InferenceOptions {
            request_timeout_seconds: Some(3601),
            ..InferenceOptions::default()
        }
        .validate()
        .is_err());
    }

    #[test]
    fn inference_options_are_strict_and_empty_options_are_omitted() {
        let parsed: InferenceOptions =
            toml::from_str("temperature = 0.5\nmax_output_tokens = 256\nreasoning = \"high\"\n")
                .unwrap();
        assert_eq!(parsed.reasoning, Some(ReasoningLevel::High));
        assert!(toml::from_str::<InferenceOptions>("unexpected = true").is_err());
        assert!(toml::from_str::<InferenceOptions>("maxOutputTokens = 256").is_err());
        let draft: InferenceOptionsDraft = serde_json::from_str(
            r#"{"maxOutputTokens":256,"requestTimeoutSeconds":90,"reasoning":"high"}"#,
        )
        .unwrap();
        let options = InferenceOptions::from(draft);
        assert_eq!(options.max_output_tokens, Some(256));
        assert_eq!(options.request_timeout_seconds, Some(90));
        assert!(
            serde_json::from_str::<InferenceOptionsDraft>(r#"{"max_output_tokens":256}"#).is_err()
        );
        let profile_draft: ModelSettingsDraft = serde_json::from_str(
            r#"{"provider":"openai_api","model":"vision-model","options":null}"#,
        )
        .unwrap();
        assert!(profile_draft.options.is_none());
        let empty_profile_draft: ModelSettingsDraft = serde_json::from_str(
            r#"{"provider":"openai_api","model":"vision-model","options":{}}"#,
        )
        .unwrap();
        assert!(InferenceOptions::from(empty_profile_draft.options.unwrap()).is_empty());

        let mut profile = ModelSettings {
            provider: InferenceBackend::OpenaiApi,
            model: "vision-model".into(),
            options: Some(InferenceOptions::default()),
        };
        profile.normalize_options();
        assert!(profile.options.is_none());
        let serialized = toml::to_string(&profile).unwrap();
        assert!(!serialized.contains("options"));
    }

    #[test]
    fn inference_snapshot_uses_frontend_option_casing() {
        let options = InferenceOptions {
            max_output_tokens: Some(256),
            request_timeout_seconds: Some(90),
            ..InferenceOptions::default()
        };
        let settings = ModelSettings {
            provider: InferenceBackend::OpenaiApi,
            model: "vision-model".into(),
            options: Some(options),
        };
        let json = serde_json::to_value(settings.snapshot(Path::new("config.toml"))).unwrap();
        let options = &json["options"];
        assert_eq!(options["maxOutputTokens"], 256);
        assert_eq!(options["requestTimeoutSeconds"], 90);
        assert!(options.get("max_output_tokens").is_none());
    }
}
