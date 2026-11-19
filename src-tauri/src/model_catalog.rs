//! Cached model metadata from models.dev.
//!
//! The endpoint and decoder are kept behind this module so an upstream schema
//! change does not leak into the settings UI or the persisted configuration.

use crate::application_config::SERVICES;
use futures_util::StreamExt;
use reqwest::header::{HeaderValue, ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED};
use serde::{de::Deserializer, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use tauri::{AppHandle, Runtime, State};
use tauri_specta::Event;

const CACHE_FILE_NAME: &str = "models.dev.catalog.json";
const MAX_BODY_BYTES: usize = 16 * 1024 * 1024;

fn deserialize_required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

#[derive(
    Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize, specta::Type, tauri_specta::Event,
)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "inference-model-catalog-changed")]
pub struct ModelCatalogSnapshot {
    pub revision: u64,
    pub providers: Vec<ModelCatalogProvider>,
    pub freshness: CatalogFreshness,
    pub loading: bool,
    pub error: Option<String>,
    pub source_hash: Option<String>,
    pub last_refresh_at: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum CatalogFreshness {
    #[default]
    Unavailable,
    Stale,
    Fresh,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct ModelCatalogProvider {
    pub id: String,
    pub name: String,
    pub models: Vec<ModelCatalogEntry>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct ModelCatalogEntry {
    pub id: String,
    pub name: String,
    #[serde(deserialize_with = "deserialize_required_option")]
    #[specta(type = Option<String>)]
    pub family: Option<String>,
    #[serde(deserialize_with = "deserialize_required_option")]
    #[specta(type = Option<bool>)]
    pub reasoning: Option<bool>,
    #[serde(deserialize_with = "deserialize_required_option")]
    #[specta(type = Option<bool>)]
    pub temperature: Option<bool>,
    #[serde(deserialize_with = "deserialize_required_option")]
    #[specta(type = Option<Vec<ModelReasoningOption>>)]
    pub reasoning_options: Option<Vec<ModelReasoningOption>>,
    #[serde(deserialize_with = "deserialize_required_option")]
    #[specta(type = Option<String>)]
    pub release_date: Option<String>,
    #[serde(deserialize_with = "deserialize_required_option")]
    #[specta(type = Option<String>)]
    pub last_updated: Option<String>,
    pub input_modalities: Vec<String>,
    pub output_modalities: Vec<String>,
    #[serde(deserialize_with = "deserialize_required_option")]
    #[specta(type = Option<String>)]
    pub status: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, specta::Type)]
#[serde(tag = "type", rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum ModelReasoningOption {
    Toggle,
    Effort {
        values: Vec<Option<String>>,
    },
    BudgetTokens {
        #[serde(deserialize_with = "deserialize_required_option")]
        #[specta(type = Option<i64>)]
        min: Option<i64>,
        #[serde(deserialize_with = "deserialize_required_option")]
        #[specta(type = Option<i64>)]
        max: Option<i64>,
    },
    Unknown {
        name: String,
    },
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct CatalogData {
    providers: Vec<ModelCatalogProvider>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CacheFile {
    hash: String,
    #[serde(deserialize_with = "deserialize_required_option")]
    etag: Option<String>,
    #[serde(deserialize_with = "deserialize_required_option")]
    last_modified: Option<String>,
    #[serde(deserialize_with = "deserialize_required_option")]
    last_refresh_at: Option<String>,
    catalog: CatalogData,
}

#[derive(Clone, Debug, Default)]
struct CatalogState {
    catalog: CatalogData,
    revision: u64,
    source_hash: Option<String>,
    etag: Option<String>,
    last_modified: Option<String>,
    last_refresh_at: Option<String>,
    freshness: CatalogFreshness,
    loading: bool,
    error: Option<String>,
}

#[derive(Clone, Debug, Default)]
struct RefreshHeaders {
    etag: Option<String>,
    last_modified: Option<String>,
}

pub struct ModelCatalogService {
    state: RwLock<CatalogState>,
    cache_path: Option<PathBuf>,
    refresh_in_flight: Mutex<bool>,
}

impl ModelCatalogService {
    pub fn new(cache_path: Option<PathBuf>) -> Self {
        Self {
            state: RwLock::new(CatalogState::default()),
            cache_path,
            refresh_in_flight: Mutex::new(false),
        }
    }

    pub fn default_cache_path(cache_directory: &Path) -> PathBuf {
        cache_directory.join(CACHE_FILE_NAME)
    }

    pub fn snapshot(&self) -> ModelCatalogSnapshot {
        let state = self.state.read().expect("model catalog lock poisoned");
        let mut providers = state.catalog.providers.clone();
        if crate::local_models::supported() {
            providers.push(ModelCatalogProvider {
                id: "local".into(),
                name: "Local".into(),
                models: crate::local_models::MODELS
                    .iter()
                    .map(|model| ModelCatalogEntry {
                        id: model.id.clone(),
                        name: model.name.clone(),
                        reasoning: Some(false),
                        temperature: Some(false),
                        input_modalities: vec!["text".into(), "image".into()],
                        output_modalities: vec!["text".into()],
                        ..Default::default()
                    })
                    .collect(),
            });
        }
        ModelCatalogSnapshot {
            revision: state.revision,
            providers,
            freshness: state.freshness.clone(),
            loading: state.loading,
            error: state.error.clone(),
            source_hash: state.source_hash.clone(),
            last_refresh_at: state.last_refresh_at.clone(),
        }
    }

    /// Load the last-known-good catalog before starting network work. A cache
    /// failure is advisory and never removes an already loaded in-memory value.
    pub fn load_cache(&self) {
        let Some(path) = self.cache_path.as_deref() else {
            return;
        };
        let result = fs::read(path)
            .map_err(|error| error.to_string())
            .and_then(|bytes| {
                serde_json::from_slice::<CacheFile>(&bytes).map_err(|error| error.to_string())
            })
            .and_then(validate_cache_file);

        let mut state = self.state.write().expect("model catalog lock poisoned");
        match result {
            Ok(cache) => {
                state.catalog = cache.catalog;
                state.source_hash = Some(cache.hash);
                state.etag = cache.etag;
                state.last_modified = cache.last_modified;
                state.last_refresh_at = cache.last_refresh_at;
                state.freshness = CatalogFreshness::Stale;
                state.error = None;
                state.revision = state.revision.saturating_add(1);
            }
            Err(error) if path.exists() => {
                let cleanup_error = fs::remove_file(path)
                    .err()
                    .filter(|cleanup| cleanup.kind() != std::io::ErrorKind::NotFound);
                state.error = Some(match cleanup_error {
                    Some(cleanup) => format!(
                        "could not read model catalog cache: {error}; could not clear invalid cache: {cleanup}"
                    ),
                    None => format!("could not read model catalog cache: {error}"),
                });
                state.revision = state.revision.saturating_add(1);
            }
            Err(_) => {}
        }
    }

    pub fn emit_changed<R: Runtime>(&self, app: &AppHandle<R>) {
        if let Err(error) = self.snapshot().emit(app) {
            eprintln!("isoboard: could not emit model catalog change: {error}");
        }
    }

    pub fn start_refresh<R>(self: Arc<Self>, app: AppHandle<R>)
    where
        R: Runtime + 'static,
    {
        if !self.begin_refresh() {
            return;
        }
        tauri::async_runtime::spawn(async move {
            self.refresh(app).await;
            self.end_refresh();
        });
    }

    async fn refresh<R: Runtime + 'static>(&self, app: AppHandle<R>) {
        self.set_loading(true);
        self.emit_changed(&app);

        let headers = {
            let state = self.state.read().expect("model catalog lock poisoned");
            RefreshHeaders {
                etag: state.etag.clone(),
                last_modified: state.last_modified.clone(),
            }
        };

        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(
                SERVICES.models_dev.request_timeout_seconds,
            ))
            .build()
        {
            Ok(client) => client,
            Err(error) => {
                self.finish_error(
                    &app,
                    format!("could not initialize model catalog request: {error}"),
                );
                return;
            }
        };
        let mut request = client.get(SERVICES.models_dev.url.as_str());
        if let Some(etag) = headers.etag.as_deref().and_then(valid_header_value) {
            request = request.header(IF_NONE_MATCH, etag);
        }
        if let Some(last_modified) = headers
            .last_modified
            .as_deref()
            .and_then(valid_header_value)
        {
            request = request.header(IF_MODIFIED_SINCE, last_modified);
        }

        let response = match request.send().await {
            Ok(response) => response,
            Err(error) => {
                self.finish_error(&app, format!("could not refresh model catalog: {error}"));
                return;
            }
        };

        if response.status() == reqwest::StatusCode::NOT_MODIFIED {
            if self
                .state
                .read()
                .expect("model catalog lock poisoned")
                .source_hash
                .is_none()
            {
                self.finish_error(&app, "model catalog returned 304 without a cache".into());
                return;
            }
            let response_headers = RefreshHeaders {
                etag: header_string(response.headers(), ETAG),
                last_modified: header_string(response.headers(), LAST_MODIFIED),
            };
            self.finish_success(&app, self.merge_headers(response_headers), true)
                .await;
            return;
        }
        if response.status() != reqwest::StatusCode::OK {
            self.finish_error(
                &app,
                format!("model catalog request returned HTTP {}", response.status()),
            );
            return;
        }

        let response_headers = self.merge_headers(RefreshHeaders {
            etag: header_string(response.headers(), ETAG),
            last_modified: header_string(response.headers(), LAST_MODIFIED),
        });
        let body = match read_bounded_body(response).await {
            Ok(body) => body,
            Err(error) => {
                self.finish_error(&app, error);
                return;
            }
        };
        let hash = sha256_hex(&body);
        let catalog = match decode_models_dev_payload(&body) {
            Ok(catalog) => catalog,
            Err(error) => {
                self.finish_error(&app, format!("could not decode model catalog: {error}"));
                return;
            }
        };
        let same_body = self
            .state
            .read()
            .expect("model catalog lock poisoned")
            .source_hash
            .as_deref()
            == Some(hash.as_str());

        if let Err(error) = self.persist_cache(
            &hash,
            response_headers.clone(),
            if same_body {
                None
            } else {
                Some(catalog.clone())
            },
        ) {
            self.finish_error(&app, format!("could not cache model catalog: {error}"));
            return;
        }

        self.apply_success(hash, response_headers, Some(catalog), same_body);
        self.emit_changed(&app);
    }

    async fn finish_success<R: Runtime + 'static>(
        &self,
        app: &AppHandle<R>,
        headers: RefreshHeaders,
        persist: bool,
    ) {
        let hash = self
            .state
            .read()
            .expect("model catalog lock poisoned")
            .source_hash
            .clone();
        if persist {
            if let Some(hash) = hash.as_deref() {
                if let Err(error) = self.persist_cache(hash, headers.clone(), None) {
                    self.finish_error(app, format!("could not cache model catalog: {error}"));
                    return;
                }
            }
        }
        self.apply_success(hash.unwrap_or_default(), headers, None, true);
        self.emit_changed(app);
    }

    fn persist_cache(
        &self,
        hash: &str,
        headers: RefreshHeaders,
        catalog: Option<CatalogData>,
    ) -> Result<(), String> {
        let Some(path) = self.cache_path.as_deref() else {
            return Ok(());
        };
        let catalog = catalog.unwrap_or_else(|| {
            self.state
                .read()
                .expect("model catalog lock poisoned")
                .catalog
                .clone()
        });
        write_cache(
            path,
            &CacheFile {
                hash: hash.to_string(),
                etag: headers.etag,
                last_modified: headers.last_modified,
                last_refresh_at: Some(now_timestamp()),
                catalog,
            },
        )
    }

    fn apply_success(
        &self,
        hash: String,
        headers: RefreshHeaders,
        catalog: Option<CatalogData>,
        same_body: bool,
    ) {
        let mut state = self.state.write().expect("model catalog lock poisoned");
        if !same_body {
            if let Some(catalog) = catalog {
                state.catalog = catalog;
            }
        }
        state.source_hash = (!hash.is_empty()).then_some(hash);
        state.etag = headers.etag;
        state.last_modified = headers.last_modified;
        state.last_refresh_at = Some(now_timestamp());
        state.freshness = CatalogFreshness::Fresh;
        state.loading = false;
        state.error = None;
        state.revision = state.revision.saturating_add(1);
    }

    fn set_loading(&self, loading: bool) {
        let mut state = self.state.write().expect("model catalog lock poisoned");
        if state.loading != loading {
            state.loading = loading;
            state.revision = state.revision.saturating_add(1);
        }
    }

    fn merge_headers(&self, replacement: RefreshHeaders) -> RefreshHeaders {
        let state = self.state.read().expect("model catalog lock poisoned");
        RefreshHeaders {
            etag: replacement.etag.or_else(|| state.etag.clone()),
            last_modified: replacement
                .last_modified
                .or_else(|| state.last_modified.clone()),
        }
    }

    fn begin_refresh(&self) -> bool {
        let mut in_flight = self
            .refresh_in_flight
            .lock()
            .expect("model catalog refresh lock poisoned");
        if *in_flight {
            return false;
        }
        *in_flight = true;
        true
    }

    fn end_refresh(&self) {
        *self
            .refresh_in_flight
            .lock()
            .expect("model catalog refresh lock poisoned") = false;
    }

    fn finish_error<R: Runtime>(&self, app: &AppHandle<R>, error: String) {
        {
            let mut state = self.state.write().expect("model catalog lock poisoned");
            state.loading = false;
            state.error = Some(error);
            if state.source_hash.is_some() {
                state.freshness = CatalogFreshness::Stale;
            }
            state.revision = state.revision.saturating_add(1);
        }
        self.emit_changed(app);
    }
}

#[tauri::command]
#[specta::specta]
pub fn get_inference_model_catalog(
    state: State<'_, Arc<ModelCatalogService>>,
) -> ModelCatalogSnapshot {
    state.snapshot()
}

fn validate_cache_file(mut cache: CacheFile) -> Result<CacheFile, String> {
    if cache.hash.len() != 64 || !cache.hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("cache hash is invalid".into());
    }
    normalize_display_names(&mut cache.catalog)?;
    validate_catalog(&cache.catalog)?;
    Ok(cache)
}

fn normalize_display_names(catalog: &mut CatalogData) -> Result<(), String> {
    for provider in &mut catalog.providers {
        provider.name = normalize_display_name(&provider.name, &provider.id);
        for model in &mut provider.models {
            model.name = normalize_display_name(&model.name, &model.id);
            if let Some(family) = model.family.as_mut() {
                *family = family.trim().to_string();
                validate_catalog_string("model family", family)?;
            }
        }
    }
    Ok(())
}

fn normalize_display_name(value: &str, fallback: &str) -> String {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        fallback.to_string()
    } else {
        normalized
    }
}

fn validate_catalog(catalog: &CatalogData) -> Result<(), String> {
    if catalog.providers.is_empty() {
        return Err("model catalog must contain at least one provider".into());
    }
    let mut provider_ids = std::collections::HashSet::new();
    let mut model_count = 0;
    for provider in &catalog.providers {
        validate_catalog_string("provider ID", &provider.id)?;
        validate_catalog_string("provider name", &provider.name)?;
        if !provider_ids.insert(&provider.id) {
            return Err(format!("duplicate provider ID {:?}", provider.id));
        }

        let mut model_ids = std::collections::HashSet::new();
        for model in &provider.models {
            model_count += 1;
            validate_catalog_string("model ID", &model.id)?;
            validate_catalog_string("model name", &model.name)?;
            if let Some(family) = model.family.as_deref() {
                validate_catalog_string("model family", family)?;
            }
            if !model_ids.insert(&model.id) {
                return Err(format!(
                    "duplicate model ID {:?} for provider {:?}",
                    model.id, provider.id
                ));
            }
            for (label, date) in [
                ("release date", model.release_date.as_deref()),
                ("last updated", model.last_updated.as_deref()),
            ] {
                if let Some(date) = date {
                    validate_catalog_string(label, date)?;
                    if !is_sensible_date(date) {
                        return Err(format!("{label} {:?} is invalid", date));
                    }
                }
            }
            if let Some(status) = model.status.as_deref() {
                validate_catalog_string("model status", status)?;
                if !matches!(status, "alpha" | "beta" | "deprecated") {
                    return Err(format!("model status {:?} is invalid", status));
                }
            }
            for modality in model
                .input_modalities
                .iter()
                .chain(model.output_modalities.iter())
            {
                validate_catalog_string("model modality", modality)?;
            }
        }
    }
    if model_count == 0 {
        return Err("model catalog must contain at least one model".into());
    }
    Ok(())
}

fn validate_catalog_string(label: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{label} must not be empty"));
    }
    if value.chars().any(char::is_control) {
        return Err(format!("{label} contains a control character"));
    }
    Ok(())
}

fn is_sensible_date(value: &str) -> bool {
    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok()
        || chrono::NaiveDate::parse_from_str(&format!("{value}-01"), "%Y-%m-%d").is_ok()
}

fn valid_header_value(value: &str) -> Option<HeaderValue> {
    HeaderValue::from_str(value).ok()
}

fn header_string(
    headers: &reqwest::header::HeaderMap,
    name: reqwest::header::HeaderName,
) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
}

async fn read_bounded_body(response: reqwest::Response) -> Result<Vec<u8>, String> {
    if response
        .content_length()
        .is_some_and(|length| length > MAX_BODY_BYTES as u64)
    {
        return Err(format!(
            "model catalog response exceeds {MAX_BODY_BYTES} bytes"
        ));
    }
    let mut body = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk =
            chunk.map_err(|error| format!("could not read model catalog response: {error}"))?;
        if body.len().saturating_add(chunk.len()) > MAX_BODY_BYTES {
            return Err(format!(
                "model catalog response exceeds {MAX_BODY_BYTES} bytes"
            ));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

fn write_cache(path: &Path, cache: &CacheFile) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "model catalog cache path has no parent".to_string())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let bytes = serde_json::to_vec_pretty(cache).map_err(|error| error.to_string())?;
    let mut temporary =
        tempfile::NamedTempFile::new_in(parent).map_err(|error| error.to_string())?;
    temporary
        .write_all(&bytes)
        .map_err(|error| error.to_string())?;
    temporary.flush().map_err(|error| error.to_string())?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|error| error.to_string())?;
    temporary
        .persist(path)
        .map_err(|error| error.error.to_string())?;
    Ok(())
}

fn sha256_hex(body: &[u8]) -> String {
    let digest = Sha256::digest(body);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn now_timestamp() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RawPayload {
    Wrapped {
        providers: std::collections::BTreeMap<String, RawProvider>,
    },
    Plain(std::collections::BTreeMap<String, RawProvider>),
}

#[derive(Default, Deserialize)]
struct RawProvider {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    models: std::collections::BTreeMap<String, RawModel>,
}

#[derive(Default, Deserialize)]
struct RawModel {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    family: Option<String>,
    #[serde(default)]
    reasoning: Option<bool>,
    #[serde(default)]
    temperature: Option<bool>,
    #[serde(default)]
    reasoning_options: Option<serde_json::Value>,
    #[serde(default)]
    release_date: Option<String>,
    #[serde(default)]
    last_updated: Option<String>,
    #[serde(default)]
    modalities: Option<RawModalities>,
    #[serde(default)]
    status: Option<String>,
}

#[derive(Default, Deserialize)]
struct RawModalities {
    #[serde(default)]
    input: Vec<String>,
    #[serde(default)]
    output: Vec<String>,
}

fn decode_reasoning_options(value: Option<serde_json::Value>) -> Option<Vec<ModelReasoningOption>> {
    let value = value?;
    let values = value.as_array()?;
    Some(values.iter().map(decode_reasoning_option).collect())
}

fn decode_reasoning_option(value: &serde_json::Value) -> ModelReasoningOption {
    let Some(object) = value.as_object() else {
        return ModelReasoningOption::Unknown {
            name: "invalid".into(),
        };
    };
    let Some(kind) = object.get("type").and_then(serde_json::Value::as_str) else {
        return ModelReasoningOption::Unknown {
            name: "unknown".into(),
        };
    };

    match kind {
        "toggle" => ModelReasoningOption::Toggle,
        "effort" => {
            let Some(values) = object.get("values").and_then(serde_json::Value::as_array) else {
                return ModelReasoningOption::Unknown { name: kind.into() };
            };
            let source_len = values.len();
            let values: Vec<Option<String>> = values
                .iter()
                .filter_map(|value| match value {
                    serde_json::Value::Null => Some(None),
                    serde_json::Value::String(value)
                        if !value.is_empty() && !value.chars().any(char::is_control) =>
                    {
                        Some(Some(value.clone()))
                    }
                    _ => None,
                })
                .collect();
            if source_len > 0 && values.is_empty() {
                return ModelReasoningOption::Unknown { name: kind.into() };
            }
            ModelReasoningOption::Effort { values }
        }
        "budget_tokens" => {
            let min = decode_budget_bound(object.get("min"));
            let max = decode_budget_bound(object.get("max"));
            let min_present = object.get("min").is_some_and(|value| !value.is_null());
            let max_present = object.get("max").is_some_and(|value| !value.is_null());
            if (min_present && min.is_none()) || (max_present && max.is_none()) {
                return ModelReasoningOption::Unknown { name: kind.into() };
            }
            if min.zip(max).is_some_and(|(min, max)| min > max) {
                return ModelReasoningOption::Unknown { name: kind.into() };
            }
            ModelReasoningOption::BudgetTokens { min, max }
        }
        _ => ModelReasoningOption::Unknown { name: kind.into() },
    }
}

fn decode_budget_bound(value: Option<&serde_json::Value>) -> Option<i64> {
    value.and_then(|value| value.as_i64())
}

fn decode_models_dev_payload(body: &[u8]) -> Result<CatalogData, String> {
    let payload = serde_json::from_slice::<RawPayload>(body).map_err(|error| error.to_string())?;
    let providers = match payload {
        RawPayload::Wrapped { providers } | RawPayload::Plain(providers) => providers,
    };
    let mut providers = providers
        .into_iter()
        .map(|(provider_id, provider)| {
            let mut models = provider
                .models
                .into_iter()
                .map(|(model_id, model)| {
                    let modalities = model.modalities.unwrap_or_default();
                    ModelCatalogEntry {
                        id: model_id.clone(),
                        name: normalize_display_name(
                            model.name.as_deref().unwrap_or(""),
                            &model_id,
                        ),
                        family: model.family,
                        reasoning: model.reasoning,
                        temperature: model.temperature,
                        reasoning_options: decode_reasoning_options(model.reasoning_options),
                        release_date: model.release_date,
                        last_updated: model.last_updated,
                        input_modalities: modalities.input,
                        output_modalities: modalities.output,
                        status: model.status,
                    }
                })
                .collect::<Vec<_>>();
            models.sort_by(|left, right| {
                right
                    .release_date
                    .as_deref()
                    .unwrap_or("")
                    .cmp(left.release_date.as_deref().unwrap_or(""))
                    .then_with(|| {
                        right
                            .last_updated
                            .as_deref()
                            .unwrap_or("")
                            .cmp(left.last_updated.as_deref().unwrap_or(""))
                    })
                    .then_with(|| left.id.cmp(&right.id))
            });
            ModelCatalogProvider {
                id: provider_id.clone(),
                name: normalize_display_name(provider.name.as_deref().unwrap_or(""), &provider_id),
                models,
            }
        })
        .collect::<Vec<_>>();
    providers.sort_by(|left, right| left.id.cmp(&right.id));
    let mut catalog = CatalogData { providers };
    normalize_display_names(&mut catalog)?;
    validate_catalog(&catalog)?;
    Ok(catalog)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn decoder_keeps_opaque_ids_and_sorts_newest_first() {
        let body = br#"
        {
          "openai": {
            "name": "OpenAI",
            "models": {
              "legacy/model:v1": {"name":"Legacy", "release_date":"2024-01-01", "modalities":{"input":["text"],"output":["text"]}},
              "new/model@2026": {"name":"Newest", "release_date":"2026-02-03", "last_updated":"2026-02-04", "modalities":{"input":["text","image"],"output":["text"]}, "future": true}
            },
            "future_provider_field": true
          }
        }
        "#;
        let catalog = decode_models_dev_payload(body).expect("valid catalog");
        assert_eq!(catalog.providers[0].name, "OpenAI");
        assert_eq!(catalog.providers[0].models[0].id, "new/model@2026");
        assert_eq!(
            catalog.providers[0].models[0].input_modalities,
            ["text", "image"]
        );
        assert_eq!(catalog.providers[0].models[1].id, "legacy/model:v1");
    }

    #[test]
    fn decoder_uses_ids_when_names_are_missing() {
        let catalog = decode_models_dev_payload(br#"{"vendor":{"models":{"opaque/id":{}}}}"#)
            .expect("valid catalog");
        assert_eq!(catalog.providers[0].name, "vendor");
        assert_eq!(catalog.providers[0].models[0].name, "opaque/id");
    }

    #[test]
    fn decoder_preserves_and_trims_model_family() {
        let body = br#"{
          "google": {
            "models": {
              "gemini-embedding-2": {"name":"Gemini Embedding 2", "family":"  embedding  ", "modalities":{"input":["text"],"output":["embedding"]}}
            }
          }
        }"#;
        let catalog = decode_models_dev_payload(body).expect("valid catalog");
        assert_eq!(
            catalog.providers[0].models[0].family.as_deref(),
            Some("embedding")
        );
    }

    #[test]
    fn decoder_preserves_reasoning_metadata_and_unknown_option_kinds() {
        let body = br#"{
          "openai": {
            "models": {
              "gpt-reasoning": {
                "reasoning": true,
                "temperature": false,
                "reasoning_options": [
                  {"type":"toggle"},
                  {"type":"effort", "values":[null,"none","minimal","low","medium","high","xhigh","max","default"]},
                  {"type":"budget_tokens", "min":1024, "max":32768},
                  {"type":"future_reasoning_mode"}
                ]
              },
              "gpt-empty-options": {"reasoning_options":[]},
              "gpt-null-options": {"reasoning_options":null},
              "gpt-absent-options": {}
            }
          }
        }"#;
        let catalog = decode_models_dev_payload(body).expect("valid catalog");
        let models = &catalog.providers[0].models;
        let reasoning = models
            .iter()
            .find(|model| model.id == "gpt-reasoning")
            .expect("reasoning model");
        assert_eq!(reasoning.reasoning, Some(true));
        assert_eq!(reasoning.temperature, Some(false));
        assert_eq!(
            reasoning.reasoning_options,
            Some(vec![
                ModelReasoningOption::Toggle,
                ModelReasoningOption::Effort {
                    values: vec![
                        None,
                        Some("none".into()),
                        Some("minimal".into()),
                        Some("low".into()),
                        Some("medium".into()),
                        Some("high".into()),
                        Some("xhigh".into()),
                        Some("max".into()),
                        Some("default".into()),
                    ],
                },
                ModelReasoningOption::BudgetTokens {
                    min: Some(1024),
                    max: Some(32768),
                },
                ModelReasoningOption::Unknown {
                    name: "future_reasoning_mode".into(),
                },
            ])
        );
        assert_eq!(
            models
                .iter()
                .find(|model| model.id == "gpt-empty-options")
                .and_then(|model| model.reasoning_options.as_ref())
                .map(Vec::len),
            Some(0)
        );
        assert_eq!(
            models
                .iter()
                .find(|model| model.id == "gpt-null-options")
                .and_then(|model| model.reasoning_options.as_ref()),
            None
        );
        assert_eq!(
            models
                .iter()
                .find(|model| model.id == "gpt-absent-options")
                .and_then(|model| model.reasoning_options.as_ref()),
            None
        );
    }

    #[test]
    fn decoder_rejects_empty_or_control_character_model_family() {
        for family in [" \t ", "language\nmodel"] {
            let body = serde_json::json!({
                "provider": {"models": {"model": {"family": family}}}
            });
            let body = serde_json::to_vec(&body).expect("family JSON");
            assert!(decode_models_dev_payload(&body).is_err());
        }
    }

    #[test]
    fn decoder_normalizes_display_name_whitespace_without_changing_ids() {
        let body = br#"{
          "provider/id": {
            "name": "  Provider\tName\n",
            "models": {
              "opaque/model:id": {"name":"  Nice\n\tModel  "},
              "opaque/blank": {"name":"\t\n"}
            }
          }
        }"#;
        let catalog = decode_models_dev_payload(body).expect("valid catalog");
        assert_eq!(catalog.providers[0].id, "provider/id");
        assert_eq!(catalog.providers[0].name, "Provider Name");
        assert_eq!(catalog.providers[0].models[0].id, "opaque/blank");
        assert_eq!(catalog.providers[0].models[0].name, "opaque/blank");
        assert_eq!(catalog.providers[0].models[1].id, "opaque/model:id");
        assert_eq!(catalog.providers[0].models[1].name, "Nice Model");
    }

    #[test]
    fn cache_writes_are_readable_and_hashes_are_validated() {
        let directory = tempdir().expect("temporary directory");
        let path = directory.path().join("nested/catalog.json");
        let catalog = CatalogData {
            providers: vec![ModelCatalogProvider {
                id: "provider".into(),
                name: "Provider".into(),
                models: vec![ModelCatalogEntry {
                    id: "model".into(),
                    name: "Model".into(),
                    family: Some("language".into()),
                    reasoning: Some(true),
                    temperature: Some(false),
                    reasoning_options: Some(vec![ModelReasoningOption::Effort {
                        values: vec![None, Some("default".into())],
                    }]),
                    ..Default::default()
                }],
            }],
        };
        let cache = CacheFile {
            hash: sha256_hex(b"body"),
            etag: Some("\"tag\"".into()),
            last_modified: None,
            last_refresh_at: None,
            catalog,
        };
        write_cache(&path, &cache).expect("atomic cache write");
        let decoded: CacheFile =
            serde_json::from_slice(&fs::read(path).expect("cache bytes")).expect("cache JSON");
        let decoded = validate_cache_file(decoded).expect("valid cache");
        assert_eq!(
            decoded.catalog.providers[0].models[0].family.as_deref(),
            Some("language")
        );
        assert_eq!(decoded.catalog.providers[0].models[0].reasoning, Some(true));
        assert_eq!(
            decoded.catalog.providers[0].models[0].temperature,
            Some(false)
        );
        assert_eq!(
            decoded.catalog.providers[0].models[0].reasoning_options,
            Some(vec![ModelReasoningOption::Effort {
                values: vec![None, Some("default".into())],
            }])
        );
    }

    #[test]
    fn decoder_rejects_empty_catalogs_and_empty_providers() {
        assert!(decode_models_dev_payload(br#"{}"#).is_err());
        assert!(decode_models_dev_payload(br#"{"provider":{"models":{}}}"#).is_err());
    }

    #[test]
    fn decoder_rejects_unknown_lifecycle_status() {
        let body = br#"{
          "provider": {
            "models": {
              "model": {"name":"Model", "status":"preview"}
            }
          }
        }"#;
        assert!(decode_models_dev_payload(body).is_err());
    }

    #[test]
    fn invalid_cache_does_not_become_stale() {
        let directory = tempdir().expect("temporary directory");
        let path = directory.path().join("catalog.json");
        let cache = CacheFile {
            hash: sha256_hex(b"body"),
            etag: None,
            last_modified: None,
            last_refresh_at: None,
            catalog: CatalogData {
                providers: vec![ModelCatalogProvider {
                    id: "provider".into(),
                    name: "Provider".into(),
                    models: vec![ModelCatalogEntry {
                        id: "model".into(),
                        name: "Model".into(),
                        release_date: Some("2024-01-01".into()),
                        ..Default::default()
                    }],
                }],
            },
        };
        write_cache(&path, &cache).expect("cache write");
        let mut invalid_cache = serde_json::to_value(&cache).expect("cache JSON");
        invalid_cache
            .as_object_mut()
            .expect("cache object")
            .insert("unknown_field".into(), serde_json::Value::Bool(true));
        fs::write(
            &path,
            serde_json::to_vec(&invalid_cache).expect("invalid cache JSON"),
        )
        .expect("invalid cache write");
        let service = ModelCatalogService::new(Some(path.clone()));
        service.load_cache();
        let snapshot = service.snapshot();
        assert_eq!(snapshot.freshness, CatalogFreshness::Unavailable);
        assert!(snapshot
            .providers
            .iter()
            .all(|provider| provider.id == "local"));
        assert!(snapshot.error.is_some());
        assert_eq!(snapshot.revision, 1);
        assert!(
            !path.exists(),
            "invalid cache should be cleared for refetch"
        );
    }

    #[test]
    fn cache_missing_nullable_field_is_rejected_and_cleared_for_refetch() {
        let directory = tempdir().expect("temporary directory");
        let path = directory.path().join("catalog.json");
        let cache = CacheFile {
            hash: sha256_hex(b"body"),
            etag: None,
            last_modified: None,
            last_refresh_at: None,
            catalog: CatalogData {
                providers: vec![ModelCatalogProvider {
                    id: "provider".into(),
                    name: "Provider".into(),
                    models: vec![ModelCatalogEntry {
                        id: "model".into(),
                        name: "Model".into(),
                        ..Default::default()
                    }],
                }],
            },
        };
        write_cache(&path, &cache).expect("cache write");
        let mut invalid_cache = serde_json::to_value(&cache).expect("cache JSON");
        invalid_cache["catalog"]["providers"][0]["models"][0]
            .as_object_mut()
            .expect("model object")
            .remove("family");
        fs::write(
            &path,
            serde_json::to_vec(&invalid_cache).expect("invalid cache JSON"),
        )
        .expect("invalid cache write");

        let service = ModelCatalogService::new(Some(path.clone()));
        service.load_cache();
        let snapshot = service.snapshot();
        assert_eq!(snapshot.freshness, CatalogFreshness::Unavailable);
        assert!(snapshot
            .providers
            .iter()
            .all(|provider| provider.id == "local"));
        assert!(snapshot.error.is_some());
        assert!(
            !path.exists(),
            "schema-invalid cache should be cleared for refetch"
        );
    }

    #[test]
    fn cached_display_names_are_normalized_before_becoming_stale() {
        let directory = tempdir().expect("temporary directory");
        let path = directory.path().join("catalog.json");
        let cache = CacheFile {
            hash: sha256_hex(b"body"),
            etag: None,
            last_modified: None,
            last_refresh_at: None,
            catalog: CatalogData {
                providers: vec![ModelCatalogProvider {
                    id: "provider".into(),
                    name: "\tProvider\nName".into(),
                    models: vec![ModelCatalogEntry {
                        id: "model".into(),
                        name: "\nModel\tName ".into(),
                        ..Default::default()
                    }],
                }],
            },
        };
        write_cache(&path, &cache).expect("cache write");
        let service = ModelCatalogService::new(Some(path));
        service.load_cache();
        let snapshot = service.snapshot();
        assert_eq!(snapshot.freshness, CatalogFreshness::Stale);
        assert_eq!(snapshot.providers[0].name, "Provider Name");
        assert_eq!(snapshot.providers[0].models[0].name, "Model Name");
    }

    #[test]
    fn successful_304_persistence_retains_catalog_and_updates_metadata() {
        let directory = tempdir().expect("temporary directory");
        let path = directory.path().join("catalog.json");
        let service = ModelCatalogService::new(Some(path.clone()));
        {
            let mut state = service.state.write().expect("catalog lock");
            state.source_hash = Some(sha256_hex(b"body"));
            state.etag = Some("\"old-tag\"".into());
            state.catalog = CatalogData {
                providers: vec![ModelCatalogProvider {
                    id: "provider".into(),
                    name: "Provider".into(),
                    models: vec![ModelCatalogEntry {
                        id: "model".into(),
                        name: "Model".into(),
                        ..Default::default()
                    }],
                }],
            };
        }
        service
            .persist_cache(
                &sha256_hex(b"body"),
                RefreshHeaders {
                    etag: Some("\"new-tag\"".into()),
                    last_modified: Some("Thu, 22 Oct 2015 07:28:00 GMT".into()),
                },
                None,
            )
            .expect("304 cache persistence");
        let persisted: CacheFile =
            serde_json::from_slice(&fs::read(path).expect("cache bytes")).expect("cache JSON");
        assert_eq!(persisted.hash, sha256_hex(b"body"));
        assert_eq!(persisted.etag.as_deref(), Some("\"new-tag\""));
        assert_eq!(
            persisted.last_modified.as_deref(),
            Some("Thu, 22 Oct 2015 07:28:00 GMT")
        );
        assert!(persisted.last_refresh_at.is_some());
        assert_eq!(persisted.catalog.providers[0].models[0].id, "model");
    }

    #[test]
    fn unchanged_body_does_not_replace_catalog_data() {
        let service = ModelCatalogService::new(None);
        {
            let mut state = service.state.write().expect("catalog lock");
            state.catalog.providers.push(ModelCatalogProvider {
                id: "provider".into(),
                name: "Original".into(),
                models: vec![],
            });
            state.source_hash = Some("a".into());
        }
        service.apply_success(
            "a".into(),
            RefreshHeaders::default(),
            Some(CatalogData {
                providers: vec![ModelCatalogProvider {
                    id: "provider".into(),
                    name: "Replacement".into(),
                    models: vec![],
                }],
            }),
            true,
        );
        assert_eq!(service.snapshot().providers[0].name, "Original");
    }

    #[test]
    fn response_headers_fall_back_to_cached_validators() {
        let service = ModelCatalogService::new(None);
        {
            let mut state = service.state.write().expect("catalog lock");
            state.etag = Some("\"old-tag\"".into());
            state.last_modified = Some("Wed, 21 Oct 2015 07:28:00 GMT".into());
        }
        let merged = service.merge_headers(RefreshHeaders {
            etag: None,
            last_modified: Some("Thu, 22 Oct 2015 07:28:00 GMT".into()),
        });
        assert_eq!(merged.etag.as_deref(), Some("\"old-tag\""));
        assert_eq!(
            merged.last_modified.as_deref(),
            Some("Thu, 22 Oct 2015 07:28:00 GMT")
        );
    }

    #[test]
    fn refresh_is_single_flight() {
        let service = ModelCatalogService::new(None);
        assert!(service.begin_refresh());
        assert!(!service.begin_refresh());
        service.end_refresh();
        assert!(service.begin_refresh());
    }

    #[test]
    fn sha256_is_lowercase_hex() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
