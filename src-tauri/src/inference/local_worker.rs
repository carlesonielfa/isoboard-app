//! A dedicated native thread serializes model loading, generation, and unloading.

use super::{
    local::{Limits, LocalRuntime, Response, StopReason},
    InferenceError, InferenceProgress, ProgressCallback,
};
use crate::local_models::{LocalModels, ModelAssets, ModelStore, ModelUse};
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc, Arc, Mutex,
    },
    time::Duration,
};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

pub use crate::local_models::RuntimePhase as Phase;

struct Request {
    _usage: ModelUse,
    id: String,
    assets: ModelAssets,
    prompt: String,
    image: Option<Vec<u8>>,
    limits: Limits,
    progress: Option<ProgressCallback>,
    cancellation: CancellationToken,
    reply: oneshot::Sender<Result<Response, InferenceError>>,
}
enum Command {
    Infer(Request),
    Unload(Option<String>, oneshot::Sender<()>),
    Shutdown,
}

#[derive(Default)]
struct WorkerControl {
    stopping: bool,
    active: Option<CancellationToken>,
}

struct WorkerOwner {
    sender: mpsc::SyncSender<Command>,
    thread: Mutex<Option<std::thread::JoinHandle<()>>>,
    control: Arc<Mutex<WorkerControl>>,
}

impl WorkerOwner {
    fn shutdown(&self) {
        let mut thread = self.thread.lock().expect("local thread lock");
        if let Some(handle) = thread.take() {
            {
                let mut control = self.control.lock().expect("local control lock");
                control.stopping = true;
                if let Some(active) = &control.active {
                    active.cancel();
                }
            }
            let _ = self.sender.send(Command::Shutdown);
            let _ = handle.join();
        }
    }
}

impl Drop for WorkerOwner {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[derive(Clone)]
pub struct LocalWorker {
    owner: Arc<WorkerOwner>,
    models: Arc<LocalModels>,
    loads: Arc<AtomicU64>,
}

impl LocalWorker {
    pub fn new(store: ModelStore) -> Result<Self, InferenceError> {
        Self::with_idle_timeout(store, Duration::from_secs(300))
    }
    pub fn with_idle_timeout(store: ModelStore, idle: Duration) -> Result<Self, InferenceError> {
        let (sender, receiver) = mpsc::sync_channel(8);
        let models = LocalModels::new(store);
        let state = models.clone();
        let loads = Arc::new(AtomicU64::new(0));
        let load_count = loads.clone();
        let control = Arc::new(Mutex::new(WorkerControl::default()));
        let worker_control = control.clone();
        let thread = std::thread::Builder::new()
            .name("isoboard-local-inference".into())
            .spawn(move || {
                let mut loaded: Option<(String, ModelAssets, LocalRuntime)> = None;
                loop {
                    let command = match receiver.recv_timeout(idle) {
                        Ok(command) => command,
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            loaded = None;
                            state.set_phase(Phase::Sleeping);
                            continue;
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    };
                    match command {
                        Command::Shutdown => {
                            drop(loaded.take());
                            state.set_phase(Phase::Sleeping);
                            break;
                        }
                        Command::Unload(model, reply) => {
                            if model.as_ref().is_none_or(|model| {
                                loaded.as_ref().is_some_and(|(id, _, _)| id == model)
                                    || matches!(state.phase(), Phase::Failed(id) if id == *model)
                            }) {
                                loaded = None;
                                state.set_phase(Phase::Sleeping);
                            }
                            let _ = reply.send(());
                        }
                        Command::Infer(request) => {
                            {
                                let mut control =
                                    worker_control.lock().expect("local control lock");
                                if control.stopping {
                                    break;
                                }
                                control.active = Some(request.cancellation.clone());
                            }
                            if request.cancellation.is_cancelled() || request.reply.is_closed() {
                                let _ = request.reply.send(Err(InferenceError::Cancelled));
                                continue;
                            }
                            if !loaded.as_ref().is_some_and(|(id, assets, _)| {
                                id == &request.id && assets == &request.assets
                            }) {
                                loaded = None;
                                state.set_phase(Phase::Loading(request.id.clone()));
                                if let Some(progress) = &request.progress {
                                    progress(InferenceProgress::LoadingModel {
                                        model: request.id.clone(),
                                    });
                                }
                                match LocalRuntime::load(
                                    &request.assets.model,
                                    &request.assets.projector,
                                ) {
                                    Ok(runtime) => {
                                        load_count.fetch_add(1, Ordering::Relaxed);
                                        loaded = Some((
                                            request.id.clone(),
                                            request.assets.clone(),
                                            runtime,
                                        ))
                                    }
                                    Err(error) => {
                                        eprintln!("isoboard local model load failed: {error}");
                                        state.set_phase(Phase::Failed(request.id));
                                        let _ =
                                            request.reply.send(Err(InferenceError::LocalModelLoad));
                                        continue;
                                    }
                                }
                            }
                            state.set_phase(Phase::Processing(request.id.clone()));
                            if let Some(progress) = &request.progress {
                                progress(InferenceProgress::Processing);
                            }
                            let result = loaded
                                .as_mut()
                                .expect("loaded runtime")
                                .2
                                .infer(
                                    &request.prompt,
                                    request.image.as_deref(),
                                    request.limits,
                                    &request.cancellation,
                                )
                                .map_err(|error| {
                                    eprintln!("isoboard local inference failed: {error}");
                                    InferenceError::CompletionFailed
                                });
                            state.set_phase(Phase::Ready(request.id));
                            worker_control.lock().expect("local control lock").active = None;
                            let _ = request.reply.send(result);
                        }
                    }
                }
            })
            .map_err(|_| InferenceError::Configuration)?;
        Ok(Self {
            owner: Arc::new(WorkerOwner {
                sender,
                thread: Mutex::new(Some(thread)),
                control,
            }),
            models,
            loads,
        })
    }
    pub fn shutdown(&self) {
        self.models.cancel_all();
        self.models.clear_publisher();
        self.owner.shutdown();
    }
    pub fn load_count(&self) -> u64 {
        self.loads.load(Ordering::Relaxed)
    }
    pub fn phase(&self) -> Phase {
        self.models.phase()
    }
    pub fn downloaded(&self, id: &str) -> bool {
        self.models.store.resolve(id).is_some()
    }
    /// Exercise vision, prefill, and decode without publishing a capture result.
    pub async fn warm_up(&self, id: &str) -> Result<(), InferenceError> {
        if !self.downloaded(id) {
            return Err(InferenceError::LocalModelMissing);
        }
        let mut image = std::io::Cursor::new(Vec::new());
        image::RgbImage::from_pixel(64, 64, image::Rgb([255, 255, 255]))
            .write_to(&mut image, image::ImageFormat::Png)
            .map_err(|_| InferenceError::CompletionFailed)?;
        let response = self
            .infer(
                id,
                "Extract the text from this image.",
                Some(image.get_ref()),
                Limits {
                    output_tokens: 1,
                    ..Limits::default()
                },
                CancellationToken::new(),
            )
            .await?;
        match response.stop_reason {
            StopReason::EndOfGeneration | StopReason::OutputTokens => Ok(()),
            StopReason::Cancelled => Err(InferenceError::Cancelled),
            StopReason::Timeout => Err(InferenceError::TimedOut),
            StopReason::OutputBytes => Err(InferenceError::OutputTooLarge),
        }
    }
    pub async fn infer(
        &self,
        id: &str,
        prompt: &str,
        image: Option<&[u8]>,
        limits: Limits,
        cancellation: CancellationToken,
    ) -> Result<Response, InferenceError> {
        self.infer_with_progress(id, prompt, image, limits, cancellation, None)
            .await
    }
    #[allow(clippy::too_many_arguments)]
    async fn infer_with_progress(
        &self,
        id: &str,
        prompt: &str,
        image: Option<&[u8]>,
        limits: Limits,
        cancellation: CancellationToken,
        progress: Option<ProgressCallback>,
    ) -> Result<Response, InferenceError> {
        let usage = self.models.reserve(id)?;
        let assets = self
            .models
            .ensure_downloaded(id, &cancellation, progress.as_ref())
            .await?;
        if image.is_none()
            && !crate::local_models::definition(id).is_some_and(|model| model.supports_text)
        {
            return Err(InferenceError::UnsupportedOption("text processing"));
        }
        let (reply, receiver) = oneshot::channel();
        self.owner
            .sender
            .try_send(Command::Infer(Request {
                _usage: usage,
                id: id.into(),
                assets,
                prompt: prompt.into(),
                image: image.map(<[u8]>::to_vec),
                limits,
                progress,
                cancellation,
                reply,
            }))
            .map_err(|_| InferenceError::Transport)?;
        receiver.await.map_err(|_| InferenceError::Transport)?
    }
    pub fn models(&self) -> &Arc<LocalModels> {
        &self.models
    }
    pub async fn unload_model(&self, model: Option<String>) -> Result<(), InferenceError> {
        let (reply, receiver) = oneshot::channel();
        self.owner
            .sender
            .try_send(Command::Unload(model, reply))
            .map_err(|_| InferenceError::Transport)?;
        receiver.await.map_err(|_| InferenceError::Transport)
    }
    pub async fn unload(&self) -> Result<(), InferenceError> {
        self.unload_model(None).await
    }
    pub async fn text(
        &self,
        id: &str,
        prompt: &str,
        image: Option<&[u8]>,
        limits: Limits,
        cancellation: CancellationToken,
    ) -> Result<String, InferenceError> {
        self.text_with_progress(id, prompt, image, limits, cancellation, None)
            .await
    }
    #[allow(clippy::too_many_arguments)]
    pub async fn text_with_progress(
        &self,
        id: &str,
        prompt: &str,
        image: Option<&[u8]>,
        limits: Limits,
        cancellation: CancellationToken,
        progress: Option<ProgressCallback>,
    ) -> Result<String, InferenceError> {
        let response = self
            .infer_with_progress(id, prompt, image, limits, cancellation, progress)
            .await?;
        match response.stop_reason {
            StopReason::EndOfGeneration if !response.text.trim().is_empty() => Ok(response.text),
            StopReason::EndOfGeneration => Err(InferenceError::NoContent),
            StopReason::Cancelled => Err(InferenceError::Cancelled),
            StopReason::Timeout => Err(InferenceError::TimedOut),
            StopReason::OutputBytes | StopReason::OutputTokens => {
                Err(InferenceError::OutputTooLarge)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn warm_up_requires_an_existing_bundle() {
        let directory = tempfile::tempdir().unwrap();
        let worker = LocalWorker::new(ModelStore::new(directory.path().into())).unwrap();
        assert!(matches!(
            worker.warm_up("isocr1-0.8b").await,
            Err(InferenceError::LocalModelMissing)
        ));
        assert_eq!(worker.load_count(), 0);
        assert_eq!(worker.phase(), Phase::Sleeping);
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    }

    #[tokio::test]
    #[ignore = "requires downloaded isOCR1 and a native GPU runtime"]
    async fn live_setup_warm_up_reuses_model_for_capture() {
        let root = std::env::var("ISOBOARD_LOCAL_MODELS_ROOT").expect("set model root");
        let worker = LocalWorker::new(ModelStore::new(root.into())).unwrap();
        let started = std::time::Instant::now();
        worker.warm_up("isocr1-0.8b").await.unwrap();
        eprintln!("Setup warm-up: {:?}", started.elapsed());
        assert_eq!(worker.phase(), Phase::Ready("isocr1-0.8b".into()));
        assert_eq!(worker.load_count(), 1);
        let started = std::time::Instant::now();
        let response = worker
            .infer(
                "isocr1-0.8b",
                "Extract the text from this image.",
                Some(include_bytes!("../../fixtures/local-probe.png")),
                Limits {
                    output_tokens: 1,
                    ..Limits::default()
                },
                CancellationToken::new(),
            )
            .await
            .unwrap();
        eprintln!(
            "First capture inference after warm-up: {:?}",
            started.elapsed()
        );
        assert!(matches!(
            response.stop_reason,
            StopReason::EndOfGeneration | StopReason::OutputTokens
        ));
        assert_eq!(
            worker.load_count(),
            1,
            "capture must reuse the warmed weights"
        );
    }
}
