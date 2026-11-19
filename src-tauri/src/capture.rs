use std::fmt;
use std::io::Cursor;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::managed_credentials::ManagedCredentialReadiness;
use crate::profiles::{ConversionSettings, ConversionSettingsApplicationMode};
use crate::secret_store::ManagedSecretKind;
use base64::Engine as _;
use image::{GenericImageView, ImageFormat, ImageReader, Limits};
use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime, State};
use tauri_plugin_dialog::{DialogExt, FilePath};
use tauri_specta::Event;
use tokio_util::sync::CancellationToken;

mod backend;

const MAX_IMAGE_BYTES: usize = 20 * 1024 * 1024;
const MIN_REGION_CAPTURE_PIXELS: u64 = 500;
const MAX_IMAGE_PIXELS: u64 = 100_000_000;
const MAX_IMAGE_DIMENSION: u32 = 10_000;
const MAX_DECODE_BYTES: u64 = 100 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Serialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum CaptureSource {
    Region,
    File,
}

#[derive(Clone, Copy, Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum CaptureFailureCode {
    Busy,
    PermissionDenied,
    Unsupported,
    InvalidImage,
    TooLarge,
    Io,
    Inference,
    Clipboard,
}

/// The profile and binding selected when one capture starts.
///
/// This value is copied before native capture begins, so changes to the
/// configuration cannot retarget an in-flight run.
#[derive(Clone, Debug, PartialEq)]
pub struct CaptureContext {
    pub profile_id: String,
    pub profile_name: String,
    pub binding_id: Option<String>,
    pub inference: Option<crate::inference::FrozenInference>,
    pub postprocessing_inference: Option<crate::inference::FrozenInference>,
    pub conversion: ConversionSettings,
    pub application_mode: ConversionSettingsApplicationMode,
    pub additional_rules: String,
    pub additional_rules_separate_pass: bool,
    pub conversion_prompt_template: Option<String>,
    pub postprocessing_prompt_template: Option<String>,
    /// Whether History keeps this capture's original image.
    pub keep_history_original: bool,
}

impl CaptureContext {
    #[cfg(test)]
    pub fn direct(profile_id: String) -> Self {
        Self {
            profile_name: profile_id.clone(),
            profile_id,
            binding_id: None,
            inference: None,
            postprocessing_inference: None,
            conversion: ConversionSettings::default(),
            application_mode: ConversionSettingsApplicationMode::PromptAndHtmlCleanup,
            additional_rules: String::new(),
            additional_rules_separate_pass: false,
            conversion_prompt_template: None,
            postprocessing_prompt_template: None,
            keep_history_original: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ImagePayload {
    pub mime_type: String,
    pub data_base64: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Serialize, specta::Type, tauri_specta::Event)]
#[tauri_specta(event_name = "capture-status")]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CaptureStatus {
    Idle {
        sequence: u64,
    },
    Started {
        sequence: u64,
        source: CaptureSource,
        profile_id: String,
        binding_id: Option<String>,
    },
    ImageReady {
        sequence: u64,
        source: CaptureSource,
        profile_id: String,
        binding_id: Option<String>,
        image: ImagePayload,
    },
    Processing {
        revision: u64,
        sequence: u64,
        source: CaptureSource,
        profile_id: String,
        binding_id: Option<String>,
    },
    DownloadingModel {
        sequence: u64,
        revision: u64,
        source: CaptureSource,
        profile_id: String,
        binding_id: Option<String>,
        model: String,
        downloaded_bytes: u64,
        total_bytes: u64,
    },
    LoadingModel {
        sequence: u64,
        revision: u64,
        source: CaptureSource,
        profile_id: String,
        binding_id: Option<String>,
        model: String,
    },
    Completed {
        sequence: u64,
        source: CaptureSource,
        profile_id: String,
        binding_id: Option<String>,
        html: String,
    },
    Cancelled {
        sequence: u64,
        source: CaptureSource,
        profile_id: String,
        binding_id: Option<String>,
    },
    Failed {
        sequence: u64,
        source: CaptureSource,
        profile_id: String,
        binding_id: Option<String>,
        code: CaptureFailureCode,
        message: String,
    },
}

#[derive(Clone, Default)]
pub struct CaptureCoordinator {
    inner: Arc<CaptureCoordinatorInner>,
}

struct CaptureCoordinatorInner {
    in_flight: AtomicBool,
    sequence: AtomicU64,
    snapshot: Mutex<CaptureStatus>,
}

impl Default for CaptureCoordinatorInner {
    fn default() -> Self {
        Self {
            in_flight: AtomicBool::new(false),
            sequence: AtomicU64::new(0),
            snapshot: Mutex::new(CaptureStatus::Idle { sequence: 0 }),
        }
    }
}

impl CaptureCoordinator {
    #[cfg(test)]
    fn begin(&self, source: CaptureSource) -> Result<CaptureGuard, CaptureFailureCode> {
        self.begin_with_context(source, &CaptureContext::direct("test-profile".into()))
    }

    fn begin_with_context(
        &self,
        source: CaptureSource,
        context: &CaptureContext,
    ) -> Result<CaptureGuard, CaptureFailureCode> {
        if self
            .inner
            .in_flight
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(CaptureFailureCode::Busy);
        }

        let sequence = self.inner.sequence.fetch_add(1, Ordering::AcqRel) + 1;
        let status = CaptureStatus::Started {
            sequence,
            source,
            profile_id: context.profile_id.clone(),
            binding_id: context.binding_id.clone(),
        };
        *self
            .inner
            .snapshot
            .lock()
            .expect("capture snapshot lock poisoned") = status;

        Ok(CaptureGuard {
            inner: Arc::clone(&self.inner),
        })
    }

    fn publish<R: Runtime>(&self, app: &AppHandle<R>, status: CaptureStatus) {
        *self
            .inner
            .snapshot
            .lock()
            .expect("capture snapshot lock poisoned") = status.clone();
        crate::tray_activity::update(app, &status);
        crate::capture_feedback::update(app, &status);
        crate::capture_overlay::update(app, &status);
        crate::app_sounds::update(app, &status);
        if let Err(error) = status.emit(app) {
            eprintln!("isoboard: could not emit capture status: {error}");
        }
    }

    pub(crate) fn is_busy(&self) -> bool {
        self.inner.in_flight.load(Ordering::Acquire)
    }

    pub(crate) fn snapshot(&self) -> CaptureStatus {
        self.inner
            .snapshot
            .lock()
            .expect("capture snapshot lock poisoned")
            .clone()
    }
}

struct CaptureGuard {
    inner: Arc<CaptureCoordinatorInner>,
}

impl Drop for CaptureGuard {
    fn drop(&mut self) {
        self.inner.in_flight.store(false, Ordering::Release);
    }
}

#[derive(Debug)]
#[allow(dead_code)]
enum CaptureFailure {
    Cancelled,
    PermissionDenied,
    Unsupported,
    InvalidImage(String),
    TooLarge,
    Io(String),
    Inference(crate::inference::InferenceError),
    InvalidPromptTemplate(String),
    Clipboard(crate::clipboard::ClipboardError, Option<String>),
}

impl CaptureFailure {
    fn code(&self) -> CaptureFailureCode {
        match self {
            Self::Cancelled => unreachable!("cancelled is represented by CaptureStatus"),
            Self::PermissionDenied => CaptureFailureCode::PermissionDenied,
            Self::Unsupported => CaptureFailureCode::Unsupported,
            Self::InvalidImage(_) => CaptureFailureCode::InvalidImage,
            Self::TooLarge => CaptureFailureCode::TooLarge,
            Self::Io(_) => CaptureFailureCode::Io,
            Self::Inference(_) => CaptureFailureCode::Inference,
            Self::InvalidPromptTemplate(_) => CaptureFailureCode::Inference,
            Self::Clipboard(..) => CaptureFailureCode::Clipboard,
        }
    }

    fn raw_inference_html(&self) -> Option<&str> {
        match self {
            Self::Clipboard(_, raw) => raw.as_deref(),
            _ => None,
        }
    }

    fn diagnostic_details(&self) -> Option<String> {
        match self {
            Self::Clipboard(crate::clipboard::ClipboardError::Preparation(error), _) => {
                Some(format!("HTML preparation failed ({error:?}): {error}."))
            }
            Self::Inference(error) => {
                let mut details = format!("Inference failed ({:?}): {error}.", error.kind());
                if let Some(response) = error.provider_response() {
                    details.push_str("\nProvider response: ");
                    details.push_str(&response);
                }
                Some(details)
            }
            _ => None,
        }
    }

    fn message(&self) -> String {
        match self {
            Self::Cancelled => String::new(),
            Self::PermissionDenied => {
                "Screen Recording permission is required. Enable isoboard in System Settings > Privacy & Security > Screen Recording.".into()
            }
            Self::Unsupported => "Screen capture is only available on macOS in this build.".into(),
            Self::InvalidImage(error) => format!("The selected file is not a valid PNG or JPEG image: {error}"),
            Self::TooLarge => format!(
                "The image exceeds the supported size or decode memory limits ({MAX_IMAGE_BYTES} input bytes, {MAX_IMAGE_PIXELS} pixels)."
            ),
            Self::Io(_) => "The image could not be read.".into(),
            Self::Inference(error) => error.to_string(),
            Self::InvalidPromptTemplate(error) => format!("The prompt template is invalid: {error}"),
            Self::Clipboard(crate::clipboard::ClipboardError::Preparation(_), _) => {
                "The inference result could not be prepared for the clipboard.".into()
            }
            Self::Clipboard(crate::clipboard::ClipboardError::Write, _) => {
                "The inference result could not be written to the clipboard.".into()
            }
        }
    }
}

impl From<backend::CaptureError> for CaptureFailure {
    fn from(error: backend::CaptureError) -> Self {
        match error {
            backend::CaptureError::Cancelled => Self::Cancelled,
            backend::CaptureError::PermissionDenied => Self::PermissionDenied,
            backend::CaptureError::Unsupported => Self::Unsupported,
            backend::CaptureError::Io(message) => Self::Io(message),
        }
    }
}

impl fmt::Display for CaptureFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message())
    }
}

fn emit_started<R: Runtime>(app: &AppHandle<R>, status: CaptureStatus) {
    crate::app_sounds::update(app, &status);
    crate::tray_activity::update(app, &status);
    crate::capture_feedback::update(app, &status);
    crate::capture_overlay::update(app, &status);
    if let Err(error) = status.emit(app) {
        eprintln!("isoboard: could not emit capture start status: {error}");
    }
}

#[cfg(test)]
fn terminal_status_with_context(
    sequence: u64,
    source: CaptureSource,
    context: &CaptureContext,
    result: Result<Vec<u8>, CaptureFailure>,
) -> CaptureStatus {
    let profile_id = context.profile_id.clone();
    let binding_id = context.binding_id.clone();
    match result.and_then(|bytes| capture_image_payload(source, bytes)) {
        Ok(image) => CaptureStatus::ImageReady {
            sequence,
            source,
            profile_id,
            binding_id,
            image,
        },
        Err(CaptureFailure::Cancelled) => CaptureStatus::Cancelled {
            sequence,
            source,
            profile_id,
            binding_id,
        },
        Err(error) => CaptureStatus::Failed {
            sequence,
            source,
            profile_id,
            binding_id,
            code: error.code(),
            message: error.message(),
        },
    }
}

fn capture_image_payload(
    source: CaptureSource,
    bytes: Vec<u8>,
) -> Result<ImagePayload, CaptureFailure> {
    let payload = image_payload(bytes)?;
    // Tiny region selections are usually accidental drags. File imports are deliberate.
    if matches!(source, CaptureSource::Region)
        && u64::from(payload.width) * u64::from(payload.height) < MIN_REGION_CAPTURE_PIXELS
    {
        return Err(CaptureFailure::Cancelled);
    }
    Ok(payload)
}

fn image_payload(bytes: Vec<u8>) -> Result<ImagePayload, CaptureFailure> {
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(CaptureFailure::TooLarge);
    }

    let reader = ImageReader::new(Cursor::new(&bytes))
        .with_guessed_format()
        .map_err(|error| CaptureFailure::InvalidImage(error.to_string()))?;
    let format = reader
        .format()
        .ok_or_else(|| CaptureFailure::InvalidImage("unknown image format".into()))?;
    let mime_type = match format {
        ImageFormat::Png => "image/png",
        ImageFormat::Jpeg => "image/jpeg",
        _ => {
            return Err(CaptureFailure::InvalidImage(
                "only PNG and JPEG are supported".into(),
            ));
        }
    };

    let mut reader = reader;
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_DIMENSION);
    limits.max_image_height = Some(MAX_IMAGE_DIMENSION);
    limits.max_alloc = Some(MAX_DECODE_BYTES);
    reader.limits(limits);
    let decoded = reader.decode().map_err(|error| match error {
        image::ImageError::Limits(_) => CaptureFailure::TooLarge,
        error => CaptureFailure::InvalidImage(error.to_string()),
    })?;
    let (width, height) = decoded.dimensions();
    if u64::from(width) * u64::from(height) > MAX_IMAGE_PIXELS {
        return Err(CaptureFailure::TooLarge);
    }

    let data_base64 = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(ImagePayload {
        mime_type: mime_type.into(),
        data_base64,
        width,
        height,
    })
}

pub fn start_region_capture_with_context(
    app: AppHandle,
    coordinator: CaptureCoordinator,
    context: CaptureContext,
    inference: std::sync::Arc<dyn crate::inference::ProductInference>,
    clipboard: std::sync::Arc<dyn crate::clipboard::ClipboardWriter>,
) -> Result<(), String> {
    if !region_capture_ready(&app, &context) {
        return Err("Capture setup is incomplete.".into());
    }
    let guard = coordinator
        .begin_with_context(CaptureSource::Region, &context)
        .map_err(|code| format!("capture is busy ({code:?})"))?;
    let sequence = coordinator.sequence();
    emit_started(
        &app,
        CaptureStatus::Started {
            sequence,
            source: CaptureSource::Region,
            profile_id: context.profile_id.clone(),
            binding_id: context.binding_id.clone(),
        },
    );

    std::thread::spawn(move || {
        let backend = backend::system();
        match backend
            .capture(backend::CaptureRequest::Region)
            .map(|outcome| outcome.bytes)
            .map_err(CaptureFailure::from)
        {
            Ok(bytes) => {
                tauri::async_runtime::spawn(process_capture(CaptureWork {
                    app,
                    coordinator,
                    guard,
                    sequence,
                    source: CaptureSource::Region,
                    context,
                    bytes,
                    inference,
                    clipboard,
                }));
            }
            Err(error) => {
                coordinator.publish(
                    &app,
                    failure_status(sequence, CaptureSource::Region, &context, error),
                );
                drop(guard);
            }
        }
    });

    Ok(())
}

fn configured_region_capture_ready(
    context: &CaptureContext,
    credential_ready: &impl Fn(&crate::inference::FrozenInference) -> bool,
) -> bool {
    if ensure_capture_ready(context).is_err() {
        return false;
    }
    let Some(inference) = context.inference.as_ref() else {
        return false;
    };
    if inference.model.trim().is_empty() || !credential_ready(inference) {
        return false;
    }
    !uses_separate_pass(context)
        || context
            .postprocessing_inference
            .as_ref()
            .is_some_and(|selection| {
                !selection.model.trim().is_empty()
                    && credential_ready(selection)
                    && (selection.backend != crate::inference_profiles::InferenceBackend::Local
                        || crate::local_models::definition(&selection.model)
                            .is_some_and(|model| model.supports_text))
            })
}

fn region_capture_ready<R: Runtime>(app: &AppHandle<R>, context: &CaptureContext) -> bool {
    if !crate::screen_capture_permission::granted() {
        return false;
    }
    let registered = {
        let config = app.state::<crate::config::SharedConfig>();
        let config = config.lock().expect("config lock poisoned");
        config.snapshot().registered
    };
    if !registered {
        return false;
    }
    let credentials = app.state::<crate::config::ManagedCredentialsState>();
    let store = credentials.0.as_ref();
    let config_path = app
        .state::<crate::config::SharedConfig>()
        .lock()
        .expect("config lock poisoned")
        .config_path()
        .to_path_buf();
    let models = crate::local_models::ModelStore::for_config(&config_path);
    let credential_ready = |selection: &crate::inference::FrozenInference| {
        if selection.backend == crate::inference_profiles::InferenceBackend::Local {
            return models.available_on_demand(&selection.model);
        }
        let Some(reference) = selection.credential_ref.as_ref() else {
            return selection.keyless();
        };
        let Some(store) = store else {
            return false;
        };
        let kind = match selection.backend.credential_kind() {
            crate::inference_profiles::CredentialKind::None => return false,
            crate::inference_profiles::CredentialKind::ApiKey => ManagedSecretKind::ApiKey,
        };
        // Passive menu checks never unlock credentials. An explicit capture retries
        // access after a denial; known missing or corrupt credentials need replacement.
        !matches!(
            store.readiness(kind, reference),
            ManagedCredentialReadiness::Missing | ManagedCredentialReadiness::Corrupt
        )
    };
    configured_region_capture_ready(context, &credential_ready)
}

pub(crate) fn active_region_capture_ready<R: Runtime>(app: &AppHandle<R>) -> bool {
    let context = {
        let config = app.state::<crate::config::SharedConfig>();
        let config = config.lock().expect("config lock poisoned");
        config.resolve_capture_context(&config.active_profile_context(), None)
    };
    context.is_some_and(|context| region_capture_ready(app, &context))
}

pub(crate) fn profile_region_capture_ready<R: Runtime>(
    app: &AppHandle<R>,
    profile_id: &str,
) -> bool {
    let context = {
        let config = app.state::<crate::config::SharedConfig>();
        let config = config.lock().expect("config lock poisoned");
        config.resolve_capture_context(profile_id, None)
    };
    context.is_some_and(|context| region_capture_ready(app, &context))
}

pub(crate) fn start_active_region_capture(app: AppHandle) -> Result<(), String> {
    let context = {
        let config = app.state::<crate::config::SharedConfig>();
        let config = config.lock().expect("config lock poisoned");
        config.resolve_capture_context(&config.active_profile_context(), None)
    }
    .ok_or("Active Profile is unavailable.")?;
    start_region_capture_with_app_services(app, context)
}

/// Start a region capture with one Profile without changing the active Profile.
pub(crate) fn start_profile_region_capture(app: AppHandle, profile_id: &str) -> Result<(), String> {
    let context = {
        let config = app.state::<crate::config::SharedConfig>();
        let config = config.lock().expect("config lock poisoned");
        config.resolve_capture_context(profile_id, None)
    }
    .ok_or("Profile is unavailable.")?;
    start_region_capture_with_app_services(app, context)
}

fn start_region_capture_with_app_services(
    app: AppHandle,
    context: CaptureContext,
) -> Result<(), String> {
    let coordinator = app.state::<CaptureCoordinator>().inner().clone();
    let inference = app
        .state::<std::sync::Arc<crate::inference::InferenceService>>()
        .inner()
        .clone();
    let clipboard = app
        .state::<std::sync::Arc<dyn crate::clipboard::ClipboardWriter>>()
        .inner()
        .clone();
    start_region_capture_with_context(app, coordinator, context, inference, clipboard)
}

pub fn start_file_open_with_context(
    app: AppHandle,
    coordinator: CaptureCoordinator,
    context: CaptureContext,
    inference: std::sync::Arc<dyn crate::inference::ProductInference>,
    clipboard: std::sync::Arc<dyn crate::clipboard::ClipboardWriter>,
) -> Result<(), String> {
    let guard = coordinator
        .begin_with_context(CaptureSource::File, &context)
        .map_err(|code| format!("file open is busy ({code:?})"))?;
    let sequence = coordinator.sequence();
    emit_started(
        &app,
        CaptureStatus::Started {
            sequence,
            source: CaptureSource::File,
            profile_id: context.profile_id.clone(),
            binding_id: context.binding_id.clone(),
        },
    );

    let callback_app = app.clone();
    let callback_coordinator = coordinator.clone();
    app.dialog()
        .file()
        .add_filter("Images", &["png", "jpg", "jpeg"])
        .pick_file(move |file_path| match file_path {
            None => {
                callback_coordinator.publish(
                    &callback_app,
                    CaptureStatus::Cancelled {
                        sequence,
                        source: CaptureSource::File,
                        profile_id: context.profile_id.clone(),
                        binding_id: context.binding_id.clone(),
                    },
                );
                drop(guard);
            }
            Some(file_path) => match read_selected_file(file_path) {
                Ok(bytes) => {
                    tauri::async_runtime::spawn(process_capture(CaptureWork {
                        app: callback_app,
                        coordinator: callback_coordinator,
                        guard,
                        sequence,
                        source: CaptureSource::File,
                        context,
                        bytes,
                        inference,
                        clipboard,
                    }));
                }
                Err(error) => {
                    callback_coordinator.publish(
                        &callback_app,
                        failure_status(sequence, CaptureSource::File, &context, error),
                    );
                    drop(guard);
                }
            },
        });

    Ok(())
}

fn read_selected_file(file_path: FilePath) -> Result<Vec<u8>, CaptureFailure> {
    let path = file_path
        .into_path()
        .map_err(|error| CaptureFailure::Io(error.to_string()))?;
    let metadata =
        std::fs::metadata(&path).map_err(|error| CaptureFailure::Io(error.to_string()))?;
    if metadata.len() > MAX_IMAGE_BYTES as u64 {
        return Err(CaptureFailure::TooLarge);
    }
    std::fs::read(path).map_err(|error| CaptureFailure::Io(error.to_string()))
}

fn failure_status(
    sequence: u64,
    source: CaptureSource,
    context: &CaptureContext,
    error: CaptureFailure,
) -> CaptureStatus {
    match error {
        CaptureFailure::Cancelled => CaptureStatus::Cancelled {
            sequence,
            source,
            profile_id: context.profile_id.clone(),
            binding_id: context.binding_id.clone(),
        },
        error => CaptureStatus::Failed {
            sequence,
            source,
            profile_id: context.profile_id.clone(),
            binding_id: context.binding_id.clone(),
            code: error.code(),
            message: error.message(),
        },
    }
}

#[cfg(test)]
fn capture_status_with_backend_and_context(
    backend: &dyn backend::CaptureBackend,
    sequence: u64,
    source: CaptureSource,
    context: &CaptureContext,
) -> CaptureStatus {
    let result = backend
        .capture(backend::CaptureRequest::Region)
        .map(|outcome| outcome.bytes)
        .map_err(CaptureFailure::from);
    terminal_status_with_context(sequence, source, context, result)
}

struct CaptureWork {
    app: AppHandle,
    coordinator: CaptureCoordinator,
    guard: CaptureGuard,
    sequence: u64,
    source: CaptureSource,
    context: CaptureContext,
    bytes: Vec<u8>,
    inference: std::sync::Arc<dyn crate::inference::ProductInference>,
    clipboard: std::sync::Arc<dyn crate::clipboard::ClipboardWriter>,
}

async fn process_capture(work: CaptureWork) {
    let CaptureWork {
        app,
        coordinator,
        guard,
        sequence,
        source,
        context,
        bytes,
        inference,
        clipboard,
    } = work;
    if let Err(error) = ensure_capture_ready(&context) {
        coordinator.publish(&app, failure_status(sequence, source, &context, error));
        drop(guard);
        return;
    }
    let payload = match capture_image_payload(source, bytes.clone()) {
        Ok(payload) => payload,
        Err(error) => {
            coordinator.publish(&app, failure_status(sequence, source, &context, error));
            drop(guard);
            return;
        }
    };
    let settings_snapshot = history_settings_snapshot(&context);
    crate::history::begin_pending(
        &app,
        crate::history::PendingHistoryInput {
            sequence,
            source: match source {
                CaptureSource::Region => "region",
                CaptureSource::File => "file",
            },
            profile_id: &context.profile_id,
            profile_name: &context.profile_name,
            settings_snapshot: &settings_snapshot,
            screenshot: &bytes,
            width: payload.width,
            height: payload.height,
            keep_original: context.keep_history_original,
        },
    );
    coordinator.publish(
        &app,
        CaptureStatus::ImageReady {
            sequence,
            source,
            profile_id: context.profile_id.clone(),
            binding_id: context.binding_id.clone(),
            image: payload.clone(),
        },
    );
    coordinator.publish(
        &app,
        CaptureStatus::Processing {
            revision: 0,
            sequence,
            source,
            profile_id: context.profile_id.clone(),
            binding_id: context.binding_id.clone(),
        },
    );

    let observer = {
        let app = app.clone();
        let coordinator = coordinator.clone();
        let profile_id = context.profile_id.clone();
        let binding_id = context.binding_id.clone();
        let revision = AtomicU64::new(0);
        std::sync::Arc::new(move |progress| {
            let revision = revision.fetch_add(1, Ordering::AcqRel) + 1;
            let status = match progress {
                crate::inference::InferenceProgress::DownloadingModel {
                    model,
                    downloaded_bytes,
                    total_bytes,
                } => CaptureStatus::DownloadingModel {
                    sequence,
                    revision,
                    source,
                    profile_id: profile_id.clone(),
                    binding_id: binding_id.clone(),
                    model,
                    downloaded_bytes,
                    total_bytes,
                },
                crate::inference::InferenceProgress::LoadingModel { model } => {
                    CaptureStatus::LoadingModel {
                        sequence,
                        revision,
                        source,
                        profile_id: profile_id.clone(),
                        binding_id: binding_id.clone(),
                        model,
                    }
                }
                crate::inference::InferenceProgress::Processing => CaptureStatus::Processing {
                    sequence,
                    revision,
                    source,
                    profile_id: profile_id.clone(),
                    binding_id: binding_id.clone(),
                },
            };
            coordinator.publish(&app, status);
        }) as crate::inference::ProgressCallback
    };
    let observed = inference.observed(observer);
    let result = prepare_capture_result(
        observed.as_deref().unwrap_or(inference.as_ref()),
        clipboard.as_ref(),
        &context,
        &bytes,
        &payload,
    )
    .await;
    let diagnostic_details = result
        .as_ref()
        .err()
        .and_then(CaptureFailure::diagnostic_details);
    let raw_inference_html = result
        .as_ref()
        .err()
        .and_then(CaptureFailure::raw_inference_html)
        .map(str::to_owned);
    let status = match result {
        Ok(prepared) => {
            if let Some(history) =
                app.try_state::<std::sync::Arc<dyn crate::history::HistoryStore>>()
            {
                let pending_identity = crate::history::pending_identity(&app, sequence);
                let input = crate::history::HistoryEntryInput {
                    id: pending_identity.as_ref().map(|pending| pending.id.clone()),
                    keep_original: pending_identity
                        .as_ref()
                        .is_some_and(|pending| pending.keep_original),
                    timestamp: pending_identity.map_or_else(
                        || chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                        |pending| pending.timestamp,
                    ),
                    capture_source: match source {
                        CaptureSource::Region => "region".into(),
                        CaptureSource::File => "file".into(),
                    },
                    processing_profile_id: context.profile_id.clone(),
                    processing_profile_name: context.profile_name.clone(),
                    settings_snapshot,
                    screenshot_bytes: bytes.clone(),
                    raw_inference_html: prepared.raw_inference_html,
                    first_inference_html: prepared.first_inference_html,
                    second_inference_html: prepared.second_inference_html,
                    formatting_body_html: prepared.formatting_body_html,
                    clean_document_html: prepared.document_html.clone(),
                    clipboard_body_html: prepared.payload.html,
                    plain_text: prepared.payload.plain_text,
                };
                match history.insert(input) {
                    Ok(_) => {
                        if let Err(error) = crate::history::refresh_tray_menu(&app) {
                            eprintln!("isoboard: could not refresh history menu: {error}");
                        }
                    }
                    Err(error) => {
                        eprintln!("isoboard: could not persist clipboard history: {error}");
                        crate::history::emit_persistence_error(&app);
                    }
                }
            }
            CaptureStatus::Completed {
                sequence,
                source,
                profile_id: context.profile_id.clone(),
                binding_id: context.binding_id.clone(),
                html: prepared.document_html,
            }
        }
        Err(error) => failure_status(sequence, source, &context, error),
    };
    match &status {
        CaptureStatus::Completed { .. } => crate::history::complete_pending(&app, sequence),
        CaptureStatus::Failed { code, message, .. } => {
            crate::history::fail_pending(
                &app,
                sequence,
                &format!("{code:?}"),
                message,
                diagnostic_details.as_deref(),
                raw_inference_html.as_deref(),
            );
        }
        CaptureStatus::Cancelled { .. } => crate::history::cancel_pending(&app, sequence),
        _ => {}
    }
    coordinator.publish(&app, status);
    drop(guard);
}

fn ensure_capture_ready(context: &CaptureContext) -> Result<(), CaptureFailure> {
    if context.inference.is_none()
        || (uses_separate_pass(context) && context.postprocessing_inference.is_none())
    {
        return Err(CaptureFailure::Inference(
            crate::inference::InferenceError::MissingAssignment,
        ));
    }
    if let Some(selection) = &context.inference {
        if selection.backend == crate::inference_profiles::InferenceBackend::Local
            && crate::local_models::definition(&selection.model)
                .is_some_and(|model| model.extraction_prompt().is_some())
            && context.conversion_prompt_template.is_none()
        {
            if !uses_separate_pass(context) && !context.additional_rules.trim().is_empty() {
                return Err(inference_failure(
                    crate::inference::InferenceError::UnsupportedOption(
                        "inline additional rules; use a separate processing pass",
                    ),
                ));
            }
            if context.application_mode
                == crate::profiles::ConversionSettingsApplicationMode::Prompt
                && (!context.conversion.tables
                    || !context.conversion.colors
                    || !context.conversion.images)
            {
                return Err(inference_failure(
                    crate::inference::InferenceError::UnsupportedOption(
                        "conversion options require cleanup with this model",
                    ),
                ));
            }
        }
    }
    if uses_separate_pass(context) {
        for selection in [
            context.inference.as_ref(),
            context.postprocessing_inference.as_ref(),
        ] {
            selection
                .expect("checked assignment")
                .validate_credential_reference()
                .map_err(inference_failure)?;
        }
    }
    Ok(())
}

fn uses_separate_pass(context: &CaptureContext) -> bool {
    context.additional_rules_separate_pass && !context.additional_rules.trim().is_empty()
}

fn history_settings_snapshot(context: &CaptureContext) -> crate::history::HistorySettingsSnapshot {
    fn model(
        selection: &crate::inference::FrozenInference,
    ) -> crate::history::HistoryModelSnapshot {
        crate::history::HistoryModelSnapshot {
            provider: selection.backend.id().to_owned(),
            model: selection.model.clone(),
        }
    }

    let separate_pass_used = uses_separate_pass(context);
    crate::history::HistorySettingsSnapshot {
        first_model: context.inference.as_ref().map(model),
        second_model: context
            .postprocessing_inference
            .as_ref()
            .filter(|_| separate_pass_used)
            .map(model),
        tables: context.conversion.tables,
        colors: context.conversion.colors,
        images: context.conversion.images,
        application_mode: context.application_mode,
        additional_rules_applied: !context.additional_rules.trim().is_empty(),
        separate_pass_used,
    }
}

async fn prepare_capture_result(
    inference: &dyn crate::inference::ProductInference,
    clipboard: &dyn crate::clipboard::ClipboardWriter,
    context: &CaptureContext,
    bytes: &[u8],
    payload: &ImagePayload,
) -> Result<crate::clipboard::PreparedClipboard, CaptureFailure> {
    // The resolved stage chains are checked before the first service call.
    ensure_capture_ready(context)?;
    let cancellation = CancellationToken::new();
    let raw_html = extract_capture_html(
        inference,
        context,
        bytes,
        &payload.mime_type,
        cancellation.clone(),
    )
    .await?;
    if uses_separate_pass(context) {
        let prepared: crate::rendering::PreparedConversion = crate::rendering::prepare_conversion(
            &raw_html,
            crate::rendering::SourceImage {
                bytes,
                width: payload.width,
                height: payload.height,
            },
        )
        .map_err(|error| {
            CaptureFailure::Clipboard(
                crate::clipboard::ClipboardError::Preparation(error),
                Some(raw_html.clone()),
            )
        })?;
        let prompt = crate::inference::params::postprocessing_prompt_with_template(
            &context.additional_rules,
            prepared.intermediate_html(),
            context.postprocessing_prompt_template.as_deref(),
        )
        .map_err(prompt_template_failure)?;
        let second_raw = inference
            .process_text(
                context
                    .postprocessing_inference
                    .as_ref()
                    .expect("checked assignment"),
                &prompt,
                cancellation.clone(),
            )
            .await
            .map_err(inference_failure)?;
        if cancellation.is_cancelled() {
            return Err(CaptureFailure::Cancelled);
        }
        let normalized = crate::rendering::finalize_postprocessing(
            prepared,
            &second_raw,
            context.conversion.clone(),
            context.application_mode,
        )
        .map_err(|error| {
            CaptureFailure::Clipboard(
                crate::clipboard::ClipboardError::Preparation(error),
                Some(second_raw.clone()),
            )
        })?;
        if cancellation.is_cancelled() {
            return Err(CaptureFailure::Cancelled);
        }
        return crate::clipboard::write_normalized_with_stages(
            clipboard,
            &raw_html,
            &second_raw,
            normalized,
        )
        .await
        .map_err(|error| CaptureFailure::Clipboard(error, Some(second_raw)));
    }
    if cancellation.is_cancelled() {
        return Err(CaptureFailure::Cancelled);
    }
    crate::clipboard::prepare_and_write_with_conversion(
        clipboard,
        &raw_html,
        bytes,
        payload.width,
        payload.height,
        context.conversion.clone(),
        context.application_mode,
    )
    .await
    .map_err(|error| CaptureFailure::Clipboard(error, Some(raw_html)))
}

async fn extract_capture_html(
    inference: &dyn crate::inference::ProductInference,
    context: &CaptureContext,
    bytes: &[u8],
    mime_type: &str,
    cancellation: CancellationToken,
) -> Result<String, CaptureFailure> {
    // Readiness precedes the service boundary, including credential lookup.
    ensure_capture_ready(context)?;
    let selection = context.inference.as_ref().ok_or(CaptureFailure::Inference(
        crate::inference::InferenceError::MissingAssignment,
    ))?;
    let rules = if uses_separate_pass(context) {
        ""
    } else {
        &context.additional_rules
    };
    let template = if selection.backend == crate::inference_profiles::InferenceBackend::Local {
        crate::local_models::definition(&selection.model)
            .and_then(|model| model.extraction_prompt())
    } else {
        None
    };
    if template.is_some() && !rules.trim().is_empty() {
        return Err(CaptureFailure::Inference(
            crate::inference::InferenceError::UnsupportedOption(
                "inline additional rules; use a separate processing pass",
            ),
        ));
    }
    let prompt = crate::inference::params::conversion_prompt_with_template(
        &context.conversion,
        context.application_mode,
        rules,
        context.conversion_prompt_template.as_deref().or(template),
    )
    .map_err(prompt_template_failure)?;
    inference
        .extract(selection, &prompt, bytes, mime_type, cancellation)
        .await
        .map_err(inference_failure)
}

fn inference_failure(error: crate::inference::InferenceError) -> CaptureFailure {
    match error {
        crate::inference::InferenceError::Cancelled => CaptureFailure::Cancelled,
        error => CaptureFailure::Inference(error),
    }
}

fn prompt_template_failure(error: crate::inference::params::PromptTemplateError) -> CaptureFailure {
    CaptureFailure::InvalidPromptTemplate(error.to_string())
}

impl CaptureCoordinator {
    fn sequence(&self) -> u64 {
        self.inner.sequence.load(Ordering::Acquire)
    }
}

#[tauri::command]
#[specta::specta]
pub fn start_region_capture_command(app: AppHandle) -> Result<(), String> {
    start_active_region_capture(app)
}

#[tauri::command]
#[specta::specta]
pub fn open_image_command(
    app: AppHandle,
    state: State<'_, CaptureCoordinator>,
    config: State<'_, crate::config::SharedConfig>,
    inference: State<'_, std::sync::Arc<crate::inference::InferenceService>>,
    clipboard: State<'_, std::sync::Arc<dyn crate::clipboard::ClipboardWriter>>,
) -> Result<(), String> {
    let context = {
        let config = config.lock().expect("config lock poisoned");
        let profile_id = config.active_profile_context();
        config
            .resolve_capture_context(&profile_id, None)
            .expect("active processing profile must exist")
    };
    start_file_open_with_context(
        app,
        state.inner().clone(),
        context,
        inference.inner().clone(),
        clipboard.inner().clone(),
    )
}

#[tauri::command]
#[specta::specta]
pub fn get_capture_snapshot(state: State<'_, CaptureCoordinator>) -> CaptureStatus {
    state.snapshot()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE_BY_ONE_PNG: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 248, 207, 192, 240,
        31, 0, 5, 0, 1, 255, 137, 153, 61, 29, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];

    #[derive(Default)]
    struct FakeInference {
        prompts: Mutex<Vec<String>>,
        calls: Mutex<Vec<(bool, crate::inference::FrozenInference, String)>>,
        fail: bool,
        first_raw: Option<String>,
        second_raw: Option<String>,
        second_error: Option<crate::inference::InferenceError>,
        cancel_after_second: bool,
        on_first: Option<Box<dyn Fn() + Send + Sync>>,
    }

    impl crate::inference::ProductInference for FakeInference {
        fn extract<'a>(
            &'a self,
            selection: &'a crate::inference::FrozenInference,
            prompt: &'a str,
            _image: &'a [u8],
            _mime_type: &'a str,
            _cancellation: CancellationToken,
        ) -> crate::inference::InferenceFuture<'a> {
            self.prompts.lock().unwrap().push(prompt.to_owned());
            self.calls
                .lock()
                .unwrap()
                .push((true, selection.clone(), prompt.into()));
            if let Some(action) = &self.on_first {
                action();
            }
            Box::pin(async {
                if self.fail {
                    Err(crate::inference::InferenceError::Transport)
                } else {
                    Ok(self
                        .first_raw
                        .clone()
                        .unwrap_or_else(|| "<div><p>Extracted text</p></div>".into()))
                }
            })
        }

        fn process_text<'a>(
            &'a self,
            selection: &'a crate::inference::FrozenInference,
            prompt: &'a str,
            cancellation: CancellationToken,
        ) -> crate::inference::InferenceFuture<'a> {
            self.calls
                .lock()
                .unwrap()
                .push((false, selection.clone(), prompt.into()));
            Box::pin(async move {
                if self.cancel_after_second {
                    cancellation.cancel();
                }
                if let Some(error) = self.second_error.clone() {
                    Err(error)
                } else {
                    let mut raw = self
                        .second_raw
                        .clone()
                        .unwrap_or_else(|| "<p>Final text</p>".into());
                    if raw.contains("{image-ref}") {
                        let start = prompt
                            .find("data-isoboard-image-ref=\"")
                            .expect("allocated image reference")
                            + "data-isoboard-image-ref=\"".len();
                        let reference = prompt[start..].split('"').next().unwrap();
                        raw = raw.replace("{image-ref}", reference);
                    }
                    Ok(raw)
                }
            })
        }
    }

    #[derive(Default)]
    struct FakeClipboard {
        writes: AtomicU64,
        published: Mutex<Vec<crate::clipboard::ClipboardPayload>>,
    }

    impl crate::clipboard::ClipboardWriter for FakeClipboard {
        fn write<'a>(
            &'a self,
            payload: crate::clipboard::ClipboardPayload,
        ) -> crate::clipboard::ClipboardFuture<'a> {
            self.writes.fetch_add(1, Ordering::Relaxed);
            self.published.lock().unwrap().push(payload);
            Box::pin(async { Ok(()) })
        }

        fn write_plain_text<'a>(&'a self, _text: String) -> crate::clipboard::ClipboardFuture<'a> {
            panic!("capture must publish the prepared rich clipboard payload")
        }
    }

    fn assigned_context() -> CaptureContext {
        let mut context = CaptureContext::direct("default".into());
        context.inference = Some(crate::inference::FrozenInference {
            processing_profile_id: "default".into(),
            backend: crate::inference_profiles::InferenceBackend::OpenaiApi,
            model: "model".into(),
            options: crate::inference_profiles::InferenceOptions::default(),
            endpoint: None,
            credential_ref: Some(
                crate::secret_store::ManagedSecretRef::parse("0123456789abcdef0123456789abcdef")
                    .unwrap(),
            ),
        });
        context
    }

    #[cfg(feature = "local-inference")]
    #[tokio::test]
    #[ignore = "requires downloaded isOCR and Qwen development models and native runtime"]
    async fn live_local_capture_lifecycle() {
        use crate::inference::{
            local::Limits,
            local_worker::{LocalWorker, Phase},
            ProductInference,
        };
        let root = std::env::var("ISOBOARD_LOCAL_MODELS_ROOT").expect("set model root");
        let worker = LocalWorker::with_idle_timeout(
            crate::local_models::ModelStore::new(root.into()),
            std::time::Duration::from_millis(600),
        )
        .unwrap();
        let service = crate::inference::InferenceService::new(None)
            .unwrap()
            .with_local_worker(worker.clone());
        let mut context = assigned_context();
        let selection = context.inference.as_mut().unwrap();
        selection.backend = crate::inference_profiles::InferenceBackend::Local;
        selection.model = "isocr1-0.8b".into();
        selection.credential_ref = None;
        context.application_mode = ConversionSettingsApplicationMode::HtmlCleanup;
        let image = include_bytes!("../fixtures/local-probe.png").to_vec();
        let payload = image_payload(image.clone()).unwrap();
        let clipboard = FakeClipboard::default();
        let progress = std::sync::Arc::new(Mutex::new(Vec::new()));
        let sink = progress.clone();
        let observed = service
            .observed(std::sync::Arc::new(move |update| {
                sink.lock().unwrap().push(update)
            }))
            .unwrap();
        for _ in 0..2 {
            prepare_capture_result(observed.as_ref(), &clipboard, &context, &image, &payload)
                .await
                .unwrap();
        }
        assert_eq!(
            progress.lock().unwrap().as_slice(),
            &[
                crate::inference::InferenceProgress::LoadingModel {
                    model: "isocr1-0.8b".into()
                },
                crate::inference::InferenceProgress::Processing,
                crate::inference::InferenceProgress::Processing,
            ]
        );
        drop(observed);
        assert_eq!(clipboard.writes.load(Ordering::Relaxed), 2);
        assert_eq!(worker.load_count(), 1, "second capture must reuse weights");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while worker.phase() != Phase::Sleeping && std::time::Instant::now() < deadline {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        assert_eq!(worker.phase(), Phase::Sleeping, "idle model must unload");
        prepare_capture_result(&service, &clipboard, &context, &image, &payload)
            .await
            .unwrap();
        assert_eq!(worker.load_count(), 2, "capture must wake the model");
        let mut text_selection = context.inference.clone().unwrap();
        assert!(service
            .process_text(&text_selection, "Return OK.", CancellationToken::new())
            .await
            .is_err());
        text_selection.model = "qwen3.5-2b".into();
        assert_eq!(
            service
                .process_text(
                    &text_selection,
                    "Return exactly ISOBOARD 123.",
                    CancellationToken::new()
                )
                .await
                .unwrap(),
            "ISOBOARD 123"
        );
        assert_eq!(
            worker.load_count(),
            3,
            "switch must replace resident weights"
        );
        let cancelled = CancellationToken::new();
        cancelled.cancel();
        assert!(worker
            .text(
                "qwen3.5-2b",
                "Return OK.",
                None,
                Limits::default(),
                cancelled
            )
            .await
            .is_err());
        worker.unload().await.unwrap();
        assert_eq!(worker.phase(), Phase::Sleeping);
        prepare_capture_result(&service, &clipboard, &context, &image, &payload)
            .await
            .unwrap();
        assert_eq!(worker.load_count(), 4);
        assert_eq!(clipboard.writes.load(Ordering::Relaxed), 4);
        eprintln!("Local capture: reuse, idle unload, wake, model switch, cancellation, explicit unload, and recovery passed");
    }

    fn split_context() -> CaptureContext {
        let mut context = assigned_context();
        context.postprocessing_inference = context.inference.clone().map(|mut chain| {
            chain.backend = crate::inference_profiles::InferenceBackend::GeminiApi;
            chain.model = "text-model".into();
            chain
        });
        context.additional_rules_separate_pass = true;
        context.additional_rules = "\n  Translate {{ literal }} & <tags>.\n".into();
        context
    }

    #[test]
    fn history_settings_snapshot_records_only_effective_capture_choices() {
        let mut context = split_context();
        context.conversion.colors = false;
        context.application_mode = ConversionSettingsApplicationMode::HtmlCleanup;
        let snapshot = history_settings_snapshot(&context);
        assert_eq!(snapshot.first_model.unwrap().model, "model");
        assert_eq!(snapshot.second_model.unwrap().model, "text-model");
        assert!(!snapshot.colors);
        assert_eq!(
            snapshot.application_mode,
            ConversionSettingsApplicationMode::HtmlCleanup
        );
        assert!(snapshot.additional_rules_applied);
        assert!(snapshot.separate_pass_used);

        context.additional_rules.clear();
        let without_rules = history_settings_snapshot(&context);
        assert!(!without_rules.additional_rules_applied);
        assert!(!without_rules.separate_pass_used);
        assert_eq!(without_rules.second_model, None);
    }

    #[test]
    fn region_capture_setup_gates_assignments_models_and_credentials() {
        let ready =
            |selection: &crate::inference::FrozenInference| selection.credential_ref.is_some();
        assert!(configured_region_capture_ready(&assigned_context(), &ready));
        assert!(!configured_region_capture_ready(
            &CaptureContext::direct("default".into()),
            &ready
        ));

        let mut local = assigned_context();
        local.inference.as_mut().unwrap().backend =
            crate::inference_profiles::InferenceBackend::Local;
        local.inference.as_mut().unwrap().model = "isocr1-0.8b".into();
        local.inference.as_mut().unwrap().credential_ref = None;
        assert!(configured_region_capture_ready(&local, &|_| true));
        assert!(!configured_region_capture_ready(&local, &|_| false));
        local.additional_rules = "Translate to Spanish".into();
        assert!(!configured_region_capture_ready(&local, &|_| true));
        local.additional_rules_separate_pass = true;
        local.postprocessing_inference = local.inference.clone();
        assert!(!configured_region_capture_ready(&local, &|_| true));
        local.postprocessing_inference.as_mut().unwrap().model = "qwen3.5-2b".into();
        assert!(configured_region_capture_ready(&local, &|_| true));

        let mut missing_model = assigned_context();
        missing_model.inference.as_mut().unwrap().model.clear();
        assert!(!configured_region_capture_ready(&missing_model, &ready));

        let mut missing_credential = assigned_context();
        missing_credential
            .inference
            .as_mut()
            .unwrap()
            .credential_ref = None;
        assert!(!configured_region_capture_ready(
            &missing_credential,
            &ready
        ));

        let mut split = split_context();
        split
            .postprocessing_inference
            .as_mut()
            .unwrap()
            .credential_ref = None;
        assert!(!configured_region_capture_ready(&split, &ready));
        split.additional_rules.clear();
        assert!(configured_region_capture_ready(&split, &ready));
    }

    fn crop_source() -> Vec<u8> {
        let mut bytes = Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(64, 64)
            .write_to(&mut bytes, ImageFormat::Png)
            .unwrap();
        bytes.into_inner()
    }

    #[tokio::test]
    async fn split_uses_ordered_frozen_chains_safe_intermediate_and_only_final_cleanup() {
        let bytes = crop_source();
        let payload = image_payload(bytes.clone()).unwrap();
        let first = "<p style=\"color:red\" onclick=\"unsafe()\">Original</p><table><tr><td>A</td><td>B</td></tr></table><img data-x1=\"0\" data-y1=\"0\" data-x2=\"1\" data-y2=\"1\" src=\"https://unsafe.test\"><script>secret_script</script>";
        let second = "<p style=\"color:blue\">Translated</p><table><tr><td>C</td><td>D</td></tr></table><img data-isoboard-image-ref=\"{image-ref}\">";
        for mode in [
            ConversionSettingsApplicationMode::Prompt,
            ConversionSettingsApplicationMode::HtmlCleanup,
            ConversionSettingsApplicationMode::PromptAndHtmlCleanup,
        ] {
            let mut context = split_context();
            context.application_mode = mode;
            context.conversion = ConversionSettings {
                tables: false,
                colors: false,
                images: false,
            };
            let source = Arc::new(Mutex::new(context.clone()));
            let changed_source = source.clone();
            let inference = FakeInference {
                first_raw: Some(first.into()),
                second_raw: Some(second.into()),
                on_first: Some(Box::new(move || {
                    let mut changed = changed_source.lock().unwrap();
                    changed.inference.as_mut().unwrap().model = "retargeted-image".into();
                    changed.postprocessing_inference.as_mut().unwrap().model =
                        "retargeted-text".into();
                    changed.additional_rules = "retargeted instructions".into();
                    changed.additional_rules_separate_pass = false;
                    changed.conversion = ConversionSettings::default();
                })),
                ..FakeInference::default()
            };
            let clipboard = FakeClipboard::default();
            let prepared =
                prepare_capture_result(&inference, &clipboard, &context, &bytes, &payload)
                    .await
                    .unwrap();
            let calls = inference.calls.lock().unwrap();
            assert_eq!(calls.len(), 2);
            assert!(calls[0].0);
            assert!(!calls[1].0);
            assert_eq!(calls[0].1, context.inference.clone().unwrap());
            assert_eq!(
                calls[1].1,
                context.postprocessing_inference.clone().unwrap()
            );
            assert_eq!(
                calls[0].2,
                crate::inference::params::conversion_prompt_with_template(
                    &context.conversion,
                    mode,
                    "",
                    None,
                )
                .unwrap()
            );
            assert!(!calls[0].2.contains(&context.additional_rules));
            assert!(calls[1].2.contains(&context.additional_rules));
            assert!(calls[1].2.contains("data-isoboard-image-ref=\""));
            assert!(calls[1].2.contains("<table>"));
            assert!(calls[1].2.contains("color: red"));
            for unsafe_input in [
                "data-x1=",
                "data:image/png;base64,",
                "https://unsafe.test",
                "onclick=",
                "secret_script",
                &payload.data_base64,
                "retargeted instructions",
            ] {
                assert!(
                    !calls[1].2.contains(unsafe_input),
                    "second-stage input leaked {unsafe_input}"
                );
            }
            assert_eq!(clipboard.writes.load(Ordering::Relaxed), 1);
            assert_eq!(
                *clipboard.published.lock().unwrap(),
                vec![prepared.payload.clone()]
            );
            assert!(prepared.payload.plain_text.contains("Translated"));
            assert!(!prepared.payload.plain_text.contains("Original"));
            assert!(!prepared.document_html.contains("data-isoboard-image-ref"));
            assert!(prepared
                .raw_inference_html
                .contains("data-isoboard-image-ref"));
            assert!(!prepared
                .raw_inference_html
                .contains("data:image/png;base64,"));
            if mode == ConversionSettingsApplicationMode::Prompt {
                assert!(prepared.document_html.contains("<table>"));
                assert!(prepared.document_html.contains("color: blue"));
                assert!(prepared.document_html.contains("data:image/png;base64,"));
            } else {
                assert!(!prepared.document_html.contains("<table>"));
                assert!(!prepared.document_html.contains("color:"));
                assert!(!prepared.document_html.contains("<img"));
                assert!(prepared.payload.plain_text.contains("C D"));
            }
        }
    }

    #[tokio::test]
    async fn split_second_stage_failure_cancellation_and_invalid_output_never_publish() {
        let payload = image_payload(ONE_BY_ONE_PNG.to_vec()).unwrap();
        let context = split_context();
        for inference in [
            FakeInference {
                second_error: Some(crate::inference::InferenceError::Transport),
                ..Default::default()
            },
            FakeInference {
                second_error: Some(crate::inference::InferenceError::Cancelled),
                ..Default::default()
            },
            FakeInference {
                cancel_after_second: true,
                ..Default::default()
            },
            FakeInference {
                second_raw: Some("<img data-isoboard-image-ref=\"unknown\">".into()),
                ..Default::default()
            },
            FakeInference {
                second_raw: Some("<script>unsafe</script>".into()),
                ..Default::default()
            },
        ] {
            let clipboard = FakeClipboard::default();
            let error =
                prepare_capture_result(&inference, &clipboard, &context, ONE_BY_ONE_PNG, &payload)
                    .await
                    .unwrap_err();
            if inference.cancel_after_second
                || inference.second_error == Some(crate::inference::InferenceError::Cancelled)
            {
                assert!(matches!(error, CaptureFailure::Cancelled));
            }
            assert_eq!(inference.calls.lock().unwrap().len(), 2);
            assert_eq!(clipboard.writes.load(Ordering::Relaxed), 0);
        }
    }

    #[tokio::test]
    async fn split_missing_credential_references_skip_both_transports() {
        let payload = image_payload(ONE_BY_ONE_PNG.to_vec()).unwrap();
        for missing_first in [true, false] {
            let mut context = split_context();
            if missing_first {
                context.inference.as_mut().unwrap().credential_ref = None;
            } else {
                context
                    .postprocessing_inference
                    .as_mut()
                    .unwrap()
                    .credential_ref = None;
            }
            let inference = FakeInference::default();
            let clipboard = FakeClipboard::default();
            let error =
                prepare_capture_result(&inference, &clipboard, &context, ONE_BY_ONE_PNG, &payload)
                    .await
                    .unwrap_err();
            assert!(matches!(
                error,
                CaptureFailure::Inference(crate::inference::InferenceError::MissingCredential)
            ));
            assert!(inference.calls.lock().unwrap().is_empty());
            assert_eq!(clipboard.writes.load(Ordering::Relaxed), 0);
        }
    }

    #[tokio::test]
    async fn split_first_preparation_failure_skips_text_transport_and_publication() {
        let payload = image_payload(ONE_BY_ONE_PNG.to_vec()).unwrap();
        let inference = FakeInference {
            first_raw: Some("<img data-x1=\"1\" data-y1=\"0\" data-x2=\"0\" data-y2=\"1\">".into()),
            ..Default::default()
        };
        let clipboard = FakeClipboard::default();
        let mut context = split_context();
        context.conversion.images = false;
        let error =
            prepare_capture_result(&inference, &clipboard, &context, ONE_BY_ONE_PNG, &payload)
                .await
                .unwrap_err();
        assert!(matches!(
            error,
            CaptureFailure::Clipboard(
                crate::clipboard::ClipboardError::Preparation(
                    crate::rendering::RenderingError::InvalidImageMarker
                ),
                _
            )
        ));
        assert_eq!(error.raw_inference_html(), inference.first_raw.as_deref());
        assert_eq!(
            error.message(),
            "The inference result could not be prepared for the clipboard."
        );
        assert_eq!(
            error.diagnostic_details().as_deref(),
            Some("HTML preparation failed (InvalidImageMarker): an image marker is invalid.")
        );
        assert_eq!(inference.calls.lock().unwrap().len(), 1);
        assert_eq!(clipboard.writes.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn missing_assignments_skip_all_calls_and_empty_rules_skip_the_separate_pass() {
        let inference = FakeInference::default();
        let clipboard = FakeClipboard::default();
        let payload = image_payload(ONE_BY_ONE_PNG.to_vec()).unwrap();
        let mut context = assigned_context();
        context.additional_rules_separate_pass = true;
        context.additional_rules = "Sensitive {{ rule }}".into();
        context.inference = None;
        context.postprocessing_inference = assigned_context().inference;
        let error =
            prepare_capture_result(&inference, &clipboard, &context, ONE_BY_ONE_PNG, &payload)
                .await
                .unwrap_err();
        assert!(matches!(
            error,
            CaptureFailure::Inference(crate::inference::InferenceError::MissingAssignment)
        ));
        assert!(matches!(error.code(), CaptureFailureCode::Inference));
        assert!(!error.message().contains("Sensitive"));
        assert!(inference.prompts.lock().unwrap().is_empty());
        assert!(inference.calls.lock().unwrap().is_empty());
        assert_eq!(clipboard.writes.load(Ordering::Relaxed), 0);

        context.inference = assigned_context().inference;
        // The controller resolves an omitted reference to this frozen first
        // chain before capture orchestration begins.
        context.postprocessing_inference = context.inference.clone();
        let fallback_inference = FakeInference::default();
        let fallback_clipboard = FakeClipboard::default();
        prepare_capture_result(
            &fallback_inference,
            &fallback_clipboard,
            &context,
            ONE_BY_ONE_PNG,
            &payload,
        )
        .await
        .unwrap();
        {
            let fallback_calls = fallback_inference.calls.lock().unwrap();
            assert_eq!(fallback_calls.len(), 2);
            assert!(fallback_calls[0].0 && !fallback_calls[1].0);
            assert_eq!(fallback_calls[0].1, fallback_calls[1].1);
        }
        assert_eq!(fallback_clipboard.writes.load(Ordering::Relaxed), 1);

        let inference = FakeInference::default();
        let clipboard = FakeClipboard::default();
        for empty in ["", " \n\t"] {
            context.additional_rules = empty.into();
            prepare_capture_result(&inference, &clipboard, &context, ONE_BY_ONE_PNG, &payload)
                .await
                .unwrap();
        }
        let prompts = inference.prompts.lock().unwrap();
        assert_eq!(prompts.len(), 2);
        assert!(prompts
            .iter()
            .all(|prompt| prompt == crate::inference::params::default_generic_conversion_prompt()));
        assert_eq!(clipboard.writes.load(Ordering::Relaxed), 2);
        assert!(inference
            .calls
            .lock()
            .unwrap()
            .iter()
            .all(|(image, _, _)| *image));
    }

    #[tokio::test]
    async fn invalid_frozen_prompt_template_fails_before_transport() {
        let inference = FakeInference::default();
        let clipboard = FakeClipboard::default();
        let payload = image_payload(ONE_BY_ONE_PNG.to_vec()).unwrap();
        let mut context = assigned_context();
        context.conversion_prompt_template = Some("{{ missing }}".into());
        let error =
            prepare_capture_result(&inference, &clipboard, &context, ONE_BY_ONE_PNG, &payload)
                .await
                .unwrap_err();
        assert!(matches!(error, CaptureFailure::InvalidPromptTemplate(_)));
        assert!(inference.calls.lock().unwrap().is_empty());
        assert_eq!(clipboard.writes.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn split_capture_uses_each_frozen_prompt_template() {
        let inference = FakeInference {
            first_raw: Some("<p>Original</p>".into()),
            second_raw: Some("<p>Final</p>".into()),
            ..Default::default()
        };
        let clipboard = FakeClipboard::default();
        let payload = image_payload(ONE_BY_ONE_PNG.to_vec()).unwrap();
        let mut context = split_context();
        context.additional_rules = "literal {{ tables }}".into();
        context.conversion_prompt_template = Some("first {{ tables }} {{ images }}".into());
        context.postprocessing_prompt_template =
            Some("second {{ additional_rules }}\n{{ intermediate_html }}".into());

        prepare_capture_result(&inference, &clipboard, &context, ONE_BY_ONE_PNG, &payload)
            .await
            .unwrap();
        let calls = inference.calls.lock().unwrap();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].2, "first True True");
        assert!(calls[1].2.starts_with("second literal {{ tables }}\n"));
        assert!(calls[1].2.contains("<p>Original</p>"));
    }

    #[tokio::test]
    async fn single_pass_rules_reach_the_only_inference_call_in_every_application_mode() {
        let payload = image_payload(ONE_BY_ONE_PNG.to_vec()).unwrap();
        let rules = "\n  Keep {{ literal }} & <tags>.\n";
        for mode in [
            ConversionSettingsApplicationMode::Prompt,
            ConversionSettingsApplicationMode::HtmlCleanup,
            ConversionSettingsApplicationMode::PromptAndHtmlCleanup,
        ] {
            let inference = FakeInference::default();
            let clipboard = FakeClipboard::default();
            let mut context = assigned_context();
            context.application_mode = mode;
            context.additional_rules = rules.into();
            context.conversion = ConversionSettings {
                tables: false,
                colors: false,
                images: false,
            };
            prepare_capture_result(&inference, &clipboard, &context, ONE_BY_ONE_PNG, &payload)
                .await
                .unwrap();
            let prompts = inference.prompts.lock().unwrap();
            assert_eq!(prompts.len(), 1);
            assert!(prompts[0].ends_with(rules));
            assert_eq!(clipboard.writes.load(Ordering::Relaxed), 1);
        }
    }

    #[tokio::test]
    async fn first_call_failure_is_terminal_without_clipboard_publication() {
        let inference = FakeInference {
            fail: true,
            ..FakeInference::default()
        };
        let clipboard = FakeClipboard::default();
        let payload = image_payload(ONE_BY_ONE_PNG.to_vec()).unwrap();
        let mut context = assigned_context();
        context.additional_rules = "Keep instructions".into();
        let error =
            prepare_capture_result(&inference, &clipboard, &context, ONE_BY_ONE_PNG, &payload)
                .await
                .unwrap_err();
        assert!(matches!(
            error,
            CaptureFailure::Inference(crate::inference::InferenceError::Transport)
        ));
        assert_eq!(inference.prompts.lock().unwrap().len(), 1);
        assert_eq!(clipboard.writes.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn validates_png_and_reports_dimensions() {
        let payload = image_payload(ONE_BY_ONE_PNG.to_vec()).unwrap();

        assert_eq!(payload.mime_type, "image/png");
        assert_eq!((payload.width, payload.height), (1, 1));
        assert!(!payload.data_base64.is_empty());
    }

    #[test]
    fn rejects_unknown_image_data() {
        let result = image_payload(b"not an image".to_vec());

        assert!(matches!(result, Err(CaptureFailure::InvalidImage(_))));
    }

    #[test]
    fn rejects_oversized_input_before_decoding() {
        let result = image_payload(vec![0; MAX_IMAGE_BYTES + 1]);

        assert!(matches!(result, Err(CaptureFailure::TooLarge)));
    }

    #[test]
    fn rejects_oversized_png_dimensions_before_allocating_pixels() {
        let mut png = ONE_BY_ONE_PNG.to_vec();
        png[16..20].copy_from_slice(&100_001_u32.to_be_bytes());
        let crc = png_crc(&png[12..29]);
        png[29..33].copy_from_slice(&crc.to_be_bytes());

        let result = image_payload(png);

        assert!(matches!(result, Err(CaptureFailure::TooLarge)));
    }

    fn png_crc(bytes: &[u8]) -> u32 {
        let mut crc = 0xffff_ffff_u32;
        for byte in bytes {
            crc ^= u32::from(*byte);
            for _ in 0..8 {
                crc = if crc & 1 == 1 {
                    (crc >> 1) ^ 0xedb8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }

    #[test]
    fn coordinator_rejects_a_second_capture_until_the_first_finishes() {
        let coordinator = CaptureCoordinator::default();
        let guard = coordinator.begin(CaptureSource::Region).unwrap();

        assert!(matches!(
            coordinator.begin(CaptureSource::File),
            Err(CaptureFailureCode::Busy)
        ));

        drop(guard);
        assert!(coordinator.begin(CaptureSource::File).is_ok());
    }

    struct FakeCaptureBackend {
        result: Result<Vec<u8>, backend::CaptureError>,
    }

    impl backend::CaptureBackend for FakeCaptureBackend {
        fn capture(
            &self,
            request: backend::CaptureRequest,
        ) -> Result<backend::CaptureOutcome, backend::CaptureError> {
            assert!(matches!(request, backend::CaptureRequest::Region));
            match &self.result {
                Ok(bytes) => Ok(backend::CaptureOutcome {
                    bytes: bytes.clone(),
                }),
                Err(backend::CaptureError::Cancelled) => Err(backend::CaptureError::Cancelled),
                Err(backend::CaptureError::PermissionDenied) => {
                    Err(backend::CaptureError::PermissionDenied)
                }
                Err(backend::CaptureError::Unsupported) => Err(backend::CaptureError::Unsupported),
                Err(backend::CaptureError::Io(message)) => {
                    Err(backend::CaptureError::Io(message.clone()))
                }
            }
        }
    }

    #[test]
    fn backend_bytes_follow_the_shared_capture_status_pipeline() {
        let context = CaptureContext::direct("test-profile".into());
        for (width, height, source, accepted) in [
            (1, 1, CaptureSource::Region, false),
            (19, 21, CaptureSource::Region, false),
            (20, 25, CaptureSource::Region, true),
            (501, 1, CaptureSource::Region, true),
            (10, 50, CaptureSource::Region, true),
            (1, 1, CaptureSource::File, true),
        ] {
            let backend = FakeCaptureBackend {
                result: Ok(png_with_dimensions(width, height)),
            };
            let status = capture_status_with_backend_and_context(&backend, 4, source, &context);
            if accepted {
                assert!(matches!(
                    status,
                    CaptureStatus::ImageReady {
                        sequence: 4,
                        image: ImagePayload { width: w, height: h, .. },
                        ..
                    } if w == width && h == height
                ));
            } else {
                assert!(matches!(
                    status,
                    CaptureStatus::Cancelled {
                        sequence: 4,
                        source: CaptureSource::Region,
                        ..
                    }
                ));
            }
        }
    }

    fn png_with_dimensions(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(width, height)
            .write_to(&mut bytes, ImageFormat::Png)
            .unwrap();
        bytes.into_inner()
    }

    #[test]
    fn backend_status_carries_the_frozen_capture_context() {
        let backend = FakeCaptureBackend {
            result: Ok(png_with_dimensions(20, 25)),
        };
        let context = CaptureContext {
            profile_id: "profile-2".into(),
            profile_name: "Profile 2".into(),
            binding_id: Some("binding-2".into()),
            inference: None,
            postprocessing_inference: None,
            conversion: ConversionSettings::default(),
            application_mode: ConversionSettingsApplicationMode::PromptAndHtmlCleanup,
            additional_rules: String::new(),
            additional_rules_separate_pass: false,
            conversion_prompt_template: None,
            postprocessing_prompt_template: None,
            keep_history_original: false,
        };

        let status =
            capture_status_with_backend_and_context(&backend, 8, CaptureSource::Region, &context);

        assert!(matches!(
            status,
            CaptureStatus::ImageReady {
                sequence: 8,
                source: CaptureSource::Region,
                ref profile_id,
                binding_id: Some(ref binding_id),
                ..
            } if profile_id == "profile-2" && binding_id == "binding-2"
        ));
    }

    #[test]
    fn backend_cancellation_stays_cancelled_in_the_shared_pipeline() {
        let backend = FakeCaptureBackend {
            result: Err(backend::CaptureError::Cancelled),
        };

        let context = CaptureContext::direct("test-profile".into());
        let status =
            capture_status_with_backend_and_context(&backend, 5, CaptureSource::Region, &context);

        assert!(matches!(
            status,
            CaptureStatus::Cancelled {
                sequence: 5,
                source: CaptureSource::Region,
                ..
            }
        ));
    }

    #[test]
    fn backend_permission_error_becomes_failed_without_an_image() {
        let backend = FakeCaptureBackend {
            result: Err(backend::CaptureError::PermissionDenied),
        };

        let context = CaptureContext::direct("test-profile".into());
        let status =
            capture_status_with_backend_and_context(&backend, 6, CaptureSource::Region, &context);

        assert!(matches!(
            status,
            CaptureStatus::Failed {
                sequence: 6,
                source: CaptureSource::Region,
                code: CaptureFailureCode::PermissionDenied,
                ..
            }
        ));
    }

    #[test]
    fn backend_unsupported_error_becomes_failed_without_an_image() {
        let backend = FakeCaptureBackend {
            result: Err(backend::CaptureError::Unsupported),
        };

        let context = CaptureContext::direct("test-profile".into());
        let status =
            capture_status_with_backend_and_context(&backend, 7, CaptureSource::Region, &context);

        assert!(matches!(
            status,
            CaptureStatus::Failed {
                sequence: 7,
                source: CaptureSource::Region,
                code: CaptureFailureCode::Unsupported,
                ..
            }
        ));
    }

    #[test]
    fn inference_failure_is_terminal_without_partial_html() {
        let context = CaptureContext::direct("default".into());
        let status = failure_status(
            9,
            CaptureSource::File,
            &context,
            CaptureFailure::Inference(crate::inference::InferenceError::MissingAssignment),
        );
        assert!(matches!(
            status,
            CaptureStatus::Failed {
                code: CaptureFailureCode::Inference,
                message,
                ..
            } if message.contains("needs a model")
        ));
    }

    #[test]
    fn io_failure_does_not_expose_filesystem_details() {
        let context = CaptureContext::direct("default".into());
        let status = failure_status(
            10,
            CaptureSource::File,
            &context,
            CaptureFailure::Io("private/path/image.png: permission denied".into()),
        );
        assert!(matches!(
            status,
            CaptureStatus::Failed { message, .. }
                if message == "The image could not be read."
                    && !message.contains("private/path")
                    && !message.contains("permission denied")
        ));
    }
}
