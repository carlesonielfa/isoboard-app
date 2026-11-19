//! Small, provider-explicit image and text inference transport.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::inference::params::default_generic_conversion_prompt;
use crate::inference_profiles::{InferenceBackend, InferenceOptions};
use crate::managed_credentials::ManagedCredentialStore;
use crate::secret_store::{ManagedSecretKind, ManagedSecretRef};

mod error;
#[cfg(feature = "local-inference")]
pub mod local;
#[cfg(feature = "local-inference")]
pub mod local_worker;
pub(crate) mod params;
pub(crate) mod runner;
pub(crate) mod target;

pub use error::InferenceError;
pub use target::{InferenceConfig, Provider};

/// The immutable inference chain selected when a capture starts.
///
/// It contains only configuration and an opaque managed credential reference.
/// Secret bytes are resolved by `InferenceService` inside the native process.
#[derive(Clone, Debug, PartialEq)]
pub struct FrozenInference {
    pub processing_profile_id: String,
    pub backend: InferenceBackend,
    pub model: String,
    /// Effective per-profile options captured with the provider chain.
    /// Unset profile options are represented by the empty value so transport
    /// adapters can apply their own provider defaults consistently.
    pub options: InferenceOptions,
    pub endpoint: Option<String>,
    pub credential_ref: Option<ManagedSecretRef>,
}

impl FrozenInference {
    /// A provider with an optional key and no stored key sends unauthenticated
    /// requests to its configured endpoint.
    pub fn keyless(&self) -> bool {
        self.backend.credential_optional()
            && self.credential_ref.is_none()
            && self.endpoint.is_some()
    }

    /// Check the required opaque reference without opening the secret store.
    pub fn validate_credential_reference(&self) -> Result<(), InferenceError> {
        if self.backend == InferenceBackend::Local || self.keyless() {
            return Ok(());
        }
        self.credential_ref
            .as_ref()
            .map(|_| ())
            .ok_or(InferenceError::MissingCredential)
    }
}

pub type InferenceFuture<'a> =
    Pin<Box<dyn Future<Output = Result<String, InferenceError>> + Send + 'a>>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InferenceProgress {
    DownloadingModel {
        model: String,
        downloaded_bytes: u64,
        total_bytes: u64,
    },
    LoadingModel {
        model: String,
    },
    Processing,
}
pub type ProgressCallback = Arc<dyn Fn(InferenceProgress) + Send + Sync>;

/// Narrow product boundary used by capture orchestration and fake-backed tests.
pub trait ProductInference: Send + Sync {
    fn observed(&self, _progress: ProgressCallback) -> Option<Box<dyn ProductInference + '_>> {
        None
    }
    fn extract<'a>(
        &'a self,
        selection: &'a FrozenInference,
        prompt: &'a str,
        image: &'a [u8],
        mime_type: &'a str,
        cancellation: tokio_util::sync::CancellationToken,
    ) -> InferenceFuture<'a>;

    /// Process a bundled-template prompt without an image attachment.
    fn process_text<'a>(
        &'a self,
        selection: &'a FrozenInference,
        prompt: &'a str,
        cancellation: tokio_util::sync::CancellationToken,
    ) -> InferenceFuture<'a>;
}

/// The normal app inference service. API keys are
/// resolved only after capture has frozen the profile/provider chain.
pub struct InferenceService {
    #[cfg(feature = "local-inference")]
    local: Option<local_worker::LocalWorker>,
    api: ApiInference,
    credentials: Option<Arc<ManagedCredentialStore>>,
}

impl InferenceService {
    pub fn shutdown_local(&self) {
        #[cfg(feature = "local-inference")]
        if let Some(worker) = &self.local {
            worker.shutdown();
        }
    }
    #[cfg(feature = "local-inference")]
    pub fn local_worker(&self) -> Option<&local_worker::LocalWorker> {
        self.local.as_ref()
    }
    #[cfg(feature = "local-inference")]
    pub fn with_local(
        mut self,
        store: crate::local_models::ModelStore,
    ) -> Result<Self, InferenceError> {
        self.local = Some(local_worker::LocalWorker::new(store)?);
        Ok(self)
    }
    #[cfg(all(test, feature = "local-inference"))]
    pub(crate) fn with_local_worker(mut self, worker: local_worker::LocalWorker) -> Self {
        self.local = Some(worker);
        self
    }

    pub fn new(credentials: Option<Arc<ManagedCredentialStore>>) -> Result<Self, InferenceError> {
        Ok(Self {
            #[cfg(feature = "local-inference")]
            local: None,
            api: ApiInference::new()?,
            credentials,
        })
    }

    async fn infer_inner(
        &self,
        selection: &FrozenInference,
        prompt: &str,
        input: InferenceInput<'_>,
        cancellation: tokio_util::sync::CancellationToken,
        progress: Option<ProgressCallback>,
    ) -> Result<String, InferenceError> {
        if selection.backend == InferenceBackend::Local {
            #[cfg(feature = "local-inference")]
            {
                if selection
                    .options
                    .temperature
                    .is_some_and(|value| value != 0.0)
                {
                    return Err(InferenceError::UnsupportedOption("temperature"));
                }
                if selection.options.reasoning.is_some_and(|value| {
                    !matches!(
                        value,
                        crate::inference_profiles::ReasoningLevel::None
                            | crate::inference_profiles::ReasoningLevel::Off
                    )
                }) {
                    return Err(InferenceError::UnsupportedOption("reasoning"));
                }
                let worker = self.local.as_ref().ok_or(InferenceError::Configuration)?;
                let image = match input {
                    InferenceInput::Image { bytes, mime_type } => {
                        runner::validate_image(bytes, mime_type)?;
                        Some(bytes)
                    }
                    InferenceInput::Text => {
                        runner::validate_text_prompt(prompt)?;
                        None
                    }
                };
                let mut limits = local::Limits {
                    context: std::num::NonZeroU32::new(16384).unwrap(),
                    output_tokens: 4096,
                    ..Default::default()
                };
                if let Some(tokens) = selection.options.max_output_tokens {
                    limits.output_tokens = tokens;
                }
                if let Some(seconds) = selection.options.request_timeout_seconds {
                    limits.timeout_ms = u64::from(seconds) * 1000;
                }
                return worker
                    .text_with_progress(
                        &selection.model,
                        prompt,
                        image,
                        limits,
                        cancellation,
                        progress,
                    )
                    .await;
            }
            #[cfg(not(feature = "local-inference"))]
            return Err(InferenceError::Configuration);
        }
        if let Some(progress) = progress {
            progress(InferenceProgress::Processing);
        }
        selection.validate_credential_reference()?;
        // `None` is reachable only for a keyless provider; every other backend
        // was rejected above without a reference.
        let credential = selection
            .credential_ref
            .as_ref()
            .map(|reference| {
                self.credentials
                    .as_ref()
                    .map(|store| (store, reference))
                    .ok_or(InferenceError::MissingCredential)
            })
            .transpose()?;

        let api_key = match credential {
            Some((credentials, credential_ref)) => {
                let key = credentials
                    .get(ManagedSecretKind::ApiKey, credential_ref)
                    .map_err(|error| match error {
                        crate::managed_credentials::ManagedCredentialError::Missing => {
                            InferenceError::MissingCredential
                        }
                        _ => InferenceError::AuthenticationRequired,
                    })?;
                String::from_utf8(key).map_err(|_| InferenceError::InvalidApiKey)?
            }
            None => String::new(),
        };
        let provider = match selection.backend {
            InferenceBackend::OpenaiApi => Provider::OpenAi,
            InferenceBackend::AnthropicApi => Provider::Anthropic,
            InferenceBackend::GeminiApi => Provider::Gemini,
            InferenceBackend::OpenaiCompatible => Provider::OpenAiCompatible,
            InferenceBackend::Local => unreachable!(),
        };
        let mut config = InferenceConfig::new(provider, selection.model.clone(), api_key);
        config = config.with_options(selection.options.clone());
        if let Some(endpoint) = selection.endpoint.as_deref() {
            config = config.with_base_url(endpoint);
        }
        self.api
            .infer_with_prompt(&config, prompt, input, cancellation)
            .await
    }
}

impl ProductInference for InferenceService {
    fn observed(&self, progress: ProgressCallback) -> Option<Box<dyn ProductInference + '_>> {
        Some(Box::new(ObservedInference {
            service: self,
            progress,
        }))
    }
    fn extract<'a>(
        &'a self,
        selection: &'a FrozenInference,
        prompt: &'a str,
        image: &'a [u8],
        mime_type: &'a str,
        cancellation: tokio_util::sync::CancellationToken,
    ) -> InferenceFuture<'a> {
        Box::pin(self.infer_inner(
            selection,
            prompt,
            InferenceInput::Image {
                bytes: image,
                mime_type,
            },
            cancellation,
            None,
        ))
    }

    fn process_text<'a>(
        &'a self,
        selection: &'a FrozenInference,
        prompt: &'a str,
        cancellation: tokio_util::sync::CancellationToken,
    ) -> InferenceFuture<'a> {
        Box::pin(self.infer_inner(selection, prompt, InferenceInput::Text, cancellation, None))
    }
}

struct ObservedInference<'a> {
    service: &'a InferenceService,
    progress: ProgressCallback,
}
impl ProductInference for ObservedInference<'_> {
    fn extract<'a>(
        &'a self,
        selection: &'a FrozenInference,
        prompt: &'a str,
        image: &'a [u8],
        mime_type: &'a str,
        cancellation: CancellationToken,
    ) -> InferenceFuture<'a> {
        Box::pin(self.service.infer_inner(
            selection,
            prompt,
            InferenceInput::Image {
                bytes: image,
                mime_type,
            },
            cancellation,
            Some(self.progress.clone()),
        ))
    }
    fn process_text<'a>(
        &'a self,
        selection: &'a FrozenInference,
        prompt: &'a str,
        cancellation: CancellationToken,
    ) -> InferenceFuture<'a> {
        Box::pin(self.service.infer_inner(
            selection,
            prompt,
            InferenceInput::Text,
            cancellation,
            Some(self.progress.clone()),
        ))
    }
}

use runner::{InferenceInput, InferenceRunner};
use target::{api_target, validate_api_config};

#[cfg(test)]
use target::validate_api_config as validate_config;
use tokio_util::sync::CancellationToken;

/// Provider-neutral streaming inference.
pub struct ApiInference {
    runner: InferenceRunner,
}

impl ApiInference {
    pub fn new() -> Result<Self, InferenceError> {
        Ok(Self {
            runner: InferenceRunner::new()?,
        })
    }

    /// Extract HTML from one validated PNG or JPEG image after a completed
    /// provider response.
    pub async fn extract(
        &self,
        config: &InferenceConfig,
        image: &[u8],
        mime_type: &str,
        cancellation: CancellationToken,
    ) -> Result<String, InferenceError> {
        self.extract_with_prompt(
            config,
            default_generic_conversion_prompt(),
            image,
            mime_type,
            cancellation,
        )
        .await
    }

    pub(crate) async fn extract_with_prompt(
        &self,
        config: &InferenceConfig,
        prompt: &str,
        image: &[u8],
        mime_type: &str,
        cancellation: CancellationToken,
    ) -> Result<String, InferenceError> {
        self.infer_with_prompt(
            config,
            prompt,
            InferenceInput::Image {
                bytes: image,
                mime_type,
            },
            cancellation,
        )
        .await
    }

    /// Send the supplied prompt literally through the provider's text route.
    pub async fn process_text(
        &self,
        config: &InferenceConfig,
        prompt: &str,
        cancellation: CancellationToken,
    ) -> Result<String, InferenceError> {
        self.infer_with_prompt(config, prompt, InferenceInput::Text, cancellation)
            .await
    }

    async fn infer_with_prompt(
        &self,
        config: &InferenceConfig,
        prompt: &str,
        input: InferenceInput<'_>,
        cancellation: CancellationToken,
    ) -> Result<String, InferenceError> {
        validate_api_config(config)?;
        let request = input.request(prompt)?;
        if cancellation.is_cancelled() {
            return Err(InferenceError::Cancelled);
        }

        let target = api_target(&self.runner.client, config, None).await?;
        let options = runner::chat_options(config.provider, &config.model, &config.options)?;
        self.runner
            .extract(
                target,
                request,
                options,
                runner::request_timeout(&config.options),
                cancellation,
            )
            .await
    }
}

#[cfg(test)]
mod tests;
