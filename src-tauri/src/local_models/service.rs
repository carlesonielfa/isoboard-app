use super::*;
use crate::application_config::SERVICES;
use crate::inference::{InferenceError, InferenceProgress, ProgressCallback};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimePhase {
    Sleeping,
    Loading(String),
    Ready(String),
    Processing(String),
    Failed(String),
}

struct DownloadRecord {
    status: DownloadStatus,
    bytes: u64,
    error: Option<String>,
    cancellation: Option<CancellationToken>,
}
struct Inner {
    revision: u64,
    downloads: BTreeMap<String, DownloadRecord>,
    uses: BTreeMap<String, usize>,
    operations: BTreeMap<String, bool>,
    runtime: RuntimePhase,
}
type Publisher = Arc<dyn Fn(LocalModelsChangedEvent) + Send + Sync>;
type DownloadCompletionHandler = Arc<dyn Fn(&str) + Send + Sync>;

pub struct LocalModels {
    pub store: ModelStore,
    inner: Mutex<Inner>,
    changes: watch::Sender<LocalModelsChangedEvent>,
    publisher: Mutex<Option<Publisher>>,
    download_completion_handler: Mutex<Option<DownloadCompletionHandler>>,
}
impl LocalModels {
    pub fn new(store: ModelStore) -> Arc<Self> {
        let (changes, _) = watch::channel(LocalModelsChangedEvent {
            revision: 0,
            models: Vec::new(),
        });
        Arc::new(Self {
            store,
            inner: Mutex::new(Inner {
                revision: 0,
                downloads: BTreeMap::new(),
                uses: BTreeMap::new(),
                operations: BTreeMap::new(),
                runtime: RuntimePhase::Sleeping,
            }),
            changes,
            publisher: Mutex::new(None),
            download_completion_handler: Mutex::new(None),
        })
    }
    pub fn set_publisher(&self, publisher: Publisher) {
        *self.publisher.lock().expect("model publisher lock") = Some(publisher);
    }
    pub fn set_download_completion_handler(&self, handler: DownloadCompletionHandler) {
        *self
            .download_completion_handler
            .lock()
            .expect("download completion lock") = Some(handler);
    }
    pub fn clear_publisher(&self) {
        *self.publisher.lock().expect("model publisher lock") = None;
    }
    pub fn subscribe(&self) -> watch::Receiver<LocalModelsChangedEvent> {
        self.changes.subscribe()
    }
    fn snapshot_locked(&self, inner: &Inner) -> LocalModelsChangedEvent {
        let models = if supported() {
            MODELS
                .iter()
                .map(|model| {
                    let assets = self.store.resolve(&model.id);
                    let downloaded = assets.is_some();
                    let total = model.download.files.iter().map(|file| file.bytes).sum();
                    let saved = self.store.partial_bytes(&model.id);
                    let record = inner.downloads.get(&model.id);
                    let status = if inner.operations.get(&model.id) == Some(&true) {
                        DownloadStatus::Removing
                    } else if downloaded {
                        DownloadStatus::Downloaded
                    } else {
                        record.map_or(
                            if saved > 0 {
                                DownloadStatus::Paused
                            } else {
                                DownloadStatus::NotDownloaded
                            },
                            |record| record.status,
                        )
                    };
                    let runtime_status = match &inner.runtime {
                        RuntimePhase::Loading(id) if id == &model.id => RuntimeStatus::Loading,
                        RuntimePhase::Ready(id) if id == &model.id => RuntimeStatus::Ready,
                        RuntimePhase::Processing(id) if id == &model.id => {
                            RuntimeStatus::Processing
                        }
                        RuntimePhase::Failed(id) if id == &model.id => RuntimeStatus::Failed,
                        _ => RuntimeStatus::Sleeping,
                    };
                    LocalModelSnapshot {
                        supports_text: model.supports_text,
                        minimum_ui_mode: model.minimum_ui_mode,
                        id: model.id.clone(),
                        name: model.name.clone(),
                        downloaded,
                        repository_url: format!(
                            "{}/{}",
                            SERVICES.hugging_face.base_url, model.download.repo
                        ),
                        license_url: match &model.download.license_file {
                            Some(file) => format!(
                                "{}/{}/blob/{}/{file}",
                                SERVICES.hugging_face.base_url,
                                model.download.repo,
                                model.download.revision
                            ),
                            None => format!(
                                "{}/{}",
                                SERVICES.hugging_face.base_url, model.download.repo
                            ),
                        },
                        download_status: status,
                        downloaded_bytes: if downloaded {
                            total
                        } else {
                            record
                                .filter(|record| record.cancellation.is_some())
                                .map_or(saved, |record| record.bytes)
                        },
                        total_bytes: total,
                        runtime_status,
                        in_use: inner.uses.values().any(|uses| *uses > 0)
                            || !inner.operations.is_empty()
                            || inner
                                .downloads
                                .values()
                                .any(|record| record.cancellation.is_some()),
                        error: record.and_then(|record| record.error.clone()),
                    }
                })
                .collect()
        } else {
            Vec::new()
        };
        LocalModelsChangedEvent {
            revision: inner.revision,
            models,
        }
    }
    pub fn snapshot(&self) -> LocalModelsChangedEvent {
        self.snapshot_locked(&self.inner.lock().expect("local models lock"))
    }
    fn publish_locked(&self, inner: &mut Inner) -> LocalModelsChangedEvent {
        inner.revision += 1;
        self.snapshot_locked(inner)
    }
    fn emit(&self, snapshot: LocalModelsChangedEvent) {
        let changed = self.changes.send_if_modified(|current| {
            if current.revision >= snapshot.revision {
                return false;
            }
            *current = snapshot.clone();
            true
        });
        if changed {
            if let Some(publisher) = self.publisher.lock().expect("model publisher lock").clone() {
                publisher(snapshot);
            }
        }
    }
    pub fn phase(&self) -> RuntimePhase {
        self.inner
            .lock()
            .expect("local models lock")
            .runtime
            .clone()
    }
    pub fn set_phase(&self, phase: RuntimePhase) {
        let snapshot = {
            let mut inner = self.inner.lock().expect("local models lock");
            if inner.runtime == phase {
                return;
            }
            inner.runtime = phase;
            self.publish_locked(&mut inner)
        };
        self.emit(snapshot);
    }
    pub fn reserve(self: &Arc<Self>, id: &str) -> Result<ModelUse, InferenceError> {
        if definition(id).is_none() {
            return Err(InferenceError::LocalModelMissing);
        }
        let snapshot = {
            let mut inner = self.inner.lock().expect("local models lock");
            if !inner.operations.is_empty() {
                return Err(InferenceError::UnsupportedOption(
                    "model resource changes during capture",
                ));
            }
            *inner.uses.entry(id.into()).or_default() += 1;
            self.publish_locked(&mut inner)
        };
        self.emit(snapshot);
        Ok(ModelUse {
            models: self.clone(),
            id: id.into(),
        })
    }
    pub fn begin_operation(
        self: &Arc<Self>,
        id: &str,
        removing: bool,
    ) -> Result<OperationGuard, String> {
        if definition(id).is_none() {
            return Err("Unknown local model.".into());
        }
        let snapshot = {
            let mut inner = self.inner.lock().expect("local models lock");
            if !inner.operations.is_empty()
                || inner.uses.values().any(|uses| *uses > 0)
                || matches!(
                    inner.runtime,
                    RuntimePhase::Loading(_) | RuntimePhase::Processing(_)
                )
                || inner
                    .downloads
                    .values()
                    .any(|record| record.cancellation.is_some())
            {
                return Err("Wait until capture and model downloads finish.".into());
            }
            inner.operations.insert(id.into(), removing);
            self.publish_locked(&mut inner)
        };
        self.emit(snapshot);
        Ok(OperationGuard {
            models: self.clone(),
            id: id.into(),
        })
    }
    pub fn start_download(self: &Arc<Self>, id: &str) -> Result<(), String> {
        let model = definition(id)
            .filter(|_| supported())
            .ok_or("Unknown or unsupported local model.")?;
        let bundle = &model.download;
        if self.store.resolve(id).is_some() {
            return Ok(());
        }
        let token = CancellationToken::new();
        let snapshot = {
            let mut inner = self.inner.lock().expect("local models lock");
            if !inner.operations.is_empty() {
                return Err("Wait until model resource changes finish.".into());
            }
            if inner
                .downloads
                .get(id)
                .is_some_and(|record| record.cancellation.is_some())
            {
                return Ok(());
            }
            inner.downloads.insert(
                id.into(),
                DownloadRecord {
                    status: DownloadStatus::Downloading,
                    bytes: self.store.partial_bytes(id),
                    error: None,
                    cancellation: Some(token.clone()),
                },
            );
            self.publish_locked(&mut inner)
        };
        self.emit(snapshot);
        let models = self.clone();
        tokio::spawn(async move {
            let reporter = {
                let models = models.clone();
                move |status, bytes, _total| {
                    models.update_download(&model.id, status, bytes, None, false);
                }
            };
            let result = models
                .store
                .download_bundle(&model.id, bundle, &token, &reporter)
                .await;
            let (status, error) = match result {
                Ok(()) => (DownloadStatus::Downloaded, None),
                Err(_) if token.is_cancelled() => (DownloadStatus::Paused, None),
                Err(error) => {
                    eprintln!("isoboard model download failed: {error}");
                    (
                        DownloadStatus::Failed,
                        Some(
                            "Download failed. Check your connection and disk space, then retry."
                                .into(),
                        ),
                    )
                }
            };
            models.update_download(
                &model.id,
                status,
                models.store.partial_bytes(&model.id),
                error,
                true,
            );
        });
        Ok(())
    }
    fn update_download(
        &self,
        id: &str,
        status: DownloadStatus,
        bytes: u64,
        error: Option<String>,
        finished: bool,
    ) {
        let (snapshot, completed) = {
            let mut inner = self.inner.lock().expect("local models lock");
            let Some(record) = inner.downloads.get_mut(id) else {
                return;
            };
            let completed =
                finished && status == DownloadStatus::Downloaded && record.cancellation.is_some();
            record.status = status;
            record.bytes = bytes;
            record.error = error;
            if finished {
                record.cancellation = None;
            }
            (self.publish_locked(&mut inner), completed)
        };
        self.emit(snapshot);
        if completed {
            let handler = self
                .download_completion_handler
                .lock()
                .expect("download completion lock")
                .clone();
            if let Some(handler) = handler {
                handler(id);
            }
        }
    }
    pub async fn cancel_download(&self, id: &str) -> Result<(), String> {
        let mut changes = self.subscribe();
        {
            let inner = self.inner.lock().expect("local models lock");
            if let Some(token) = inner
                .downloads
                .get(id)
                .and_then(|record| record.cancellation.as_ref())
            {
                token.cancel();
            } else {
                return Ok(());
            }
        }
        loop {
            if !self
                .inner
                .lock()
                .expect("local models lock")
                .downloads
                .get(id)
                .is_some_and(|record| record.cancellation.is_some())
            {
                return Ok(());
            }
            changes
                .changed()
                .await
                .map_err(|_| "Model download service stopped.")?;
        }
    }
    pub fn cancel_all(&self) {
        for record in self
            .inner
            .lock()
            .expect("local models lock")
            .downloads
            .values()
        {
            if let Some(token) = &record.cancellation {
                token.cancel();
            }
        }
    }
    pub async fn ensure_downloaded(
        self: &Arc<Self>,
        id: &str,
        cancellation: &CancellationToken,
        progress: Option<&ProgressCallback>,
    ) -> Result<ModelAssets, InferenceError> {
        if let Some(assets) = self.store.resolve(id) {
            return Ok(assets);
        }
        let mut changes = self.subscribe();
        self.start_download(id)
            .map_err(|_| InferenceError::LocalModelMissing)?;
        loop {
            if cancellation.is_cancelled() {
                return Err(InferenceError::Cancelled);
            }
            let snapshot = self.snapshot();
            let model = snapshot
                .models
                .iter()
                .find(|model| model.id == id)
                .ok_or(InferenceError::LocalModelMissing)?;
            if let Some(progress) = progress {
                progress(InferenceProgress::DownloadingModel {
                    model: id.into(),
                    downloaded_bytes: model.downloaded_bytes,
                    total_bytes: model.total_bytes,
                });
            }
            match model.download_status {
                DownloadStatus::Downloaded => {
                    return self
                        .store
                        .resolve(id)
                        .ok_or(InferenceError::LocalModelMissing)
                }
                DownloadStatus::Paused => return Err(InferenceError::Cancelled),
                DownloadStatus::Failed => return Err(InferenceError::LocalModelDownload),
                _ => {}
            }
            tokio::select! { _ = cancellation.cancelled() => return Err(InferenceError::Cancelled), result = changes.changed() => result.map_err(|_| InferenceError::Transport)? }
        }
    }
    pub async fn remove_download(&self, id: &str) -> Result<(), String> {
        self.store.remove(id).await?;
        let snapshot = {
            let mut inner = self.inner.lock().expect("local models lock");
            inner.downloads.remove(id);
            self.publish_locked(&mut inner)
        };
        self.emit(snapshot);
        Ok(())
    }
}

pub struct ModelUse {
    models: Arc<LocalModels>,
    id: String,
}
impl Drop for ModelUse {
    fn drop(&mut self) {
        let snapshot = {
            let mut inner = self.models.inner.lock().expect("local models lock");
            if let Some(uses) = inner.uses.get_mut(&self.id) {
                *uses = uses.saturating_sub(1);
            }
            self.models.publish_locked(&mut inner)
        };
        self.models.emit(snapshot);
    }
}
pub struct OperationGuard {
    models: Arc<LocalModels>,
    id: String,
}
impl Drop for OperationGuard {
    fn drop(&mut self) {
        let snapshot = {
            let mut inner = self.models.inner.lock().expect("local models lock");
            inner.operations.remove(&self.id);
            self.models.publish_locked(&mut inner)
        };
        self.models.emit(snapshot);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn completion_notification_only_follows_success_once_per_download() {
        let temp = tempfile::tempdir().unwrap();
        let models = LocalModels::new(ModelStore::new(temp.path().into()));
        let completed = Arc::new(Mutex::new(Vec::new()));
        let observed = completed.clone();
        models.set_download_completion_handler(Arc::new(move |id| {
            observed.lock().unwrap().push(id.to_owned());
        }));
        let id = "isocr1-0.8b";
        for status in [
            DownloadStatus::Paused,
            DownloadStatus::Failed,
            DownloadStatus::Downloaded,
        ] {
            models.inner.lock().unwrap().downloads.insert(
                id.into(),
                DownloadRecord {
                    status: DownloadStatus::Downloading,
                    bytes: 0,
                    error: None,
                    cancellation: Some(CancellationToken::new()),
                },
            );
            models.update_download(id, DownloadStatus::Verifying, 1, None, false);
            assert!(completed.lock().unwrap().is_empty());
            models.update_download(id, status, 2, None, true);
        }
        assert_eq!(*completed.lock().unwrap(), vec![id]);
        models.update_download(id, DownloadStatus::Downloaded, 2, None, true);
        models.set_phase(RuntimePhase::Ready(id.into()));
        assert_eq!(*completed.lock().unwrap(), vec![id]);
    }

    #[test]
    fn resource_changes_exclude_inference_and_each_other() {
        let temp = tempfile::tempdir().unwrap();
        let models = LocalModels::new(ModelStore::new(temp.path().into()));
        let usage = models.reserve("qwen3.5-2b").unwrap();
        assert!(models.begin_operation("isocr1-0.8b", true).is_err());
        drop(usage);
        let operation = models.begin_operation("qwen3.5-2b", true).unwrap();
        assert!(models.reserve("qwen3.5-2b").is_err());
        assert!(models.begin_operation("isocr1-0.8b", false).is_err());
        drop(operation);
        assert!(models.reserve("qwen3.5-2b").is_ok());
    }
    #[tokio::test]
    #[ignore = "downloads a pinned model bundle; set ISOBOARD_DOWNLOAD_TEST_ROOT"]
    async fn live_download_cancel_resume() {
        let root = std::env::var("ISOBOARD_DOWNLOAD_TEST_ROOT").expect("set isolated root");
        let id =
            std::env::var("ISOBOARD_DOWNLOAD_TEST_MODEL").unwrap_or_else(|_| "qwen3.5-2b".into());
        let models = LocalModels::new(ModelStore::new(root.into()));
        assert!(
            models.store.resolve(&id).is_none(),
            "use a fresh isolated test root"
        );
        models.start_download(&id).unwrap();
        let mut changes = models.subscribe();
        loop {
            let snapshot = models.snapshot();
            let model = snapshot.models.iter().find(|m| m.id == id).unwrap();
            assert_ne!(
                model.download_status,
                DownloadStatus::Failed,
                "{:?}",
                model.error
            );
            if model.downloaded_bytes >= 8 * 1024 * 1024 {
                break;
            }
            changes.changed().await.unwrap();
        }
        models.cancel_download(&id).await.unwrap();
        let paused = models.snapshot();
        let paused = paused.models.iter().find(|m| m.id == id).unwrap();
        assert_eq!(paused.download_status, DownloadStatus::Paused);
        assert!(paused.downloaded_bytes > 0);
        println!(
            "retained {} bytes after cancellation",
            paused.downloaded_bytes
        );
        // Reconstruct the service to exercise resume after an app restart.
        let restarted = LocalModels::new(models.store.clone());
        let assets = restarted
            .ensure_downloaded(&id, &CancellationToken::new(), None)
            .await
            .unwrap();
        println!(
            "verified model {} and projector {}",
            assets.model.display(),
            assets.projector.display()
        );
        assert_eq!(
            restarted
                .snapshot()
                .models
                .iter()
                .find(|m| m.id == id)
                .unwrap()
                .download_status,
            DownloadStatus::Downloaded
        );
    }
}
