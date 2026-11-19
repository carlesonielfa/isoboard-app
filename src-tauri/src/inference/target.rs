//! Explicit target construction for API requests.

use genai::adapter::AdapterKind;
use genai::resolver::{AuthData, Endpoint};
use genai::{Client, ModelIden, ServiceTarget};
use reqwest::Url;
use std::net::IpAddr;

use super::error::InferenceError;
use super::runner::MAX_MODEL_BYTES;
use crate::inference_profiles::InferenceOptions;

const MAX_API_KEY_BYTES: usize = 16 * 1024;
const MAX_ENDPOINT_BYTES: usize = 2 * 1024;

/// Chat route that `genai` appends to OpenAI-compatible base URLs.
const OPENAI_CHAT_ROUTE: &str = "chat/completions";

/// Supported API wire protocols.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    OpenAi,
    Anthropic,
    Gemini,
    OpenAiCompatible,
}

impl Provider {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "openai" => Some(Self::OpenAi),
            "anthropic" => Some(Self::Anthropic),
            "gemini" => Some(Self::Gemini),
            "openai-compatible" | "openai_compatible" | "compatible" => {
                Some(Self::OpenAiCompatible)
            }
            _ => None,
        }
    }

    pub const fn id(self) -> &'static str {
        match self {
            Self::OpenAi => "openai",
            Self::Anthropic => "anthropic",
            Self::Gemini => "gemini",
            Self::OpenAiCompatible => "openai-compatible",
        }
    }
}

/// Explicit per-request provider configuration.
#[derive(Clone)]
pub struct InferenceConfig {
    pub provider: Provider,
    pub model: String,
    pub api_key: String,
    /// Required only for [`Provider::OpenAiCompatible`]. This is a base URL;
    /// `genai` appends the provider route.
    pub base_url: Option<String>,
    pub options: InferenceOptions,
}

impl InferenceConfig {
    pub fn new(provider: Provider, model: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            provider,
            model: model.into(),
            api_key: api_key.into(),
            base_url: None,
            options: InferenceOptions::default(),
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    pub fn with_options(mut self, options: InferenceOptions) -> Self {
        self.options = options;
        self
    }
}

pub(crate) fn validate_api_config(config: &InferenceConfig) -> Result<(), InferenceError> {
    config
        .options
        .validate()
        .map_err(|_| InferenceError::UnsupportedOption("invalid inference option"))?;
    if config.model.trim().is_empty() || config.model.len() > MAX_MODEL_BYTES {
        return Err(InferenceError::InvalidModel);
    }
    let keyless = config.provider == Provider::OpenAiCompatible && config.api_key.is_empty();
    if (!keyless && config.api_key.trim().is_empty())
        || config.api_key.len() > MAX_API_KEY_BYTES
        || config.api_key.chars().any(char::is_control)
    {
        return Err(InferenceError::InvalidApiKey);
    }
    if config.provider != Provider::OpenAiCompatible && config.base_url.is_some() {
        return Err(InferenceError::EndpointNotAllowed);
    }
    match &config.base_url {
        Some(base_url) => validate_endpoint(base_url)?,
        None if config.provider == Provider::OpenAiCompatible => {
            return Err(InferenceError::InvalidEndpoint)
        }
        None => {}
    }
    Ok(())
}

pub(crate) async fn api_target(
    client: &Client,
    config: &InferenceConfig,
    endpoint_override: Option<&str>,
) -> Result<ServiceTarget, InferenceError> {
    let adapter = api_adapter(config.provider);
    let model = ModelIden::new(adapter, config.model.clone());
    let mut target = client
        .resolve_service_target(model)
        .await
        .map_err(|_| InferenceError::Configuration)?;
    target.auth = AuthData::from_single(config.api_key.clone());
    if let Some(endpoint) = endpoint_override.or(config.base_url.as_deref()) {
        let endpoint = normalize_endpoint(endpoint);
        if config.api_key.is_empty() {
            // The OpenAI adapter always writes `Authorization: Bearer <key>`.
            // A keyless server receives the same chat route with no auth header.
            let url = Url::parse(&endpoint)
                .and_then(|base| base.join(OPENAI_CHAT_ROUTE))
                .map_err(|_| InferenceError::InvalidEndpoint)?;
            target.auth = AuthData::RequestOverride {
                url: url.into(),
                headers: genai::Headers::default(),
            };
        }
        target.endpoint = Endpoint::from_owned(endpoint);
    }
    Ok(target)
}

fn normalize_endpoint(endpoint: &str) -> String {
    if endpoint.ends_with('/') {
        endpoint.to_owned()
    } else {
        format!("{endpoint}/")
    }
}

fn api_adapter(provider: Provider) -> AdapterKind {
    match provider {
        Provider::OpenAi | Provider::OpenAiCompatible => AdapterKind::OpenAI,
        Provider::Anthropic => AdapterKind::Anthropic,
        Provider::Gemini => AdapterKind::Gemini,
    }
}

fn validate_endpoint(value: &str) -> Result<(), InferenceError> {
    if value.is_empty() || value.len() > MAX_ENDPOINT_BYTES {
        return Err(InferenceError::InvalidEndpoint);
    }
    let url = Url::parse(value).map_err(|_| InferenceError::InvalidEndpoint)?;
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(InferenceError::InvalidEndpoint);
    }
    let host = url.host_str().ok_or(InferenceError::InvalidEndpoint)?;
    let is_loopback = host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<IpAddr>()
            .map(|ip| ip.is_loopback())
            .unwrap_or(false);
    match url.scheme() {
        "https" => {}
        "http" if is_loopback => {}
        _ => return Err(InferenceError::InvalidEndpoint),
    }
    Ok(())
}
