//! Model identity and downloaded assets. Download transport is a separate concern.

use std::path::{Path, PathBuf};

pub mod commands;
mod download;
mod service;
pub use service::{LocalModels, ModelUse, OperationGuard, RuntimePhase};

#[derive(Clone, serde::Serialize, serde::Deserialize, specta::Type, tauri_specta::Event)]
#[tauri_specta(event_name = "local-models-changed")]
pub struct LocalModelsChangedEvent {
    pub revision: u64,
    pub models: Vec<LocalModelSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelAssets {
    pub model: PathBuf,
    pub projector: PathBuf,
}

mod catalog;
pub use catalog::{DownloadBundle, DownloadFile, ModelDefinition, MODELS};

pub fn supported() -> bool {
    cfg!(feature = "local-inference") && cfg!(target_pointer_width = "64")
}

pub fn definition(id: &str) -> Option<&'static ModelDefinition> {
    MODELS.iter().find(|model| model.id == id)
}

#[derive(Clone)]
pub struct ModelStore {
    root: PathBuf,
}

impl ModelStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
    pub fn for_config(path: &Path) -> Self {
        Self::new(path.parent().unwrap_or(Path::new(".")).join("models"))
    }
    pub fn resolve(&self, id: &str) -> Option<ModelAssets> {
        if !supported() {
            return None;
        }
        let definition = definition(id)?;
        let directory = self
            .root
            .join(&definition.id)
            .join(&definition.download.revision);
        let assets = ModelAssets {
            model: directory.join(&definition.download.files[0].name),
            projector: directory.join(&definition.download.files[1].name),
        };
        [&assets.model, &assets.projector]
            .iter()
            .enumerate()
            .all(|(index, file)| {
                file.metadata().is_ok_and(|metadata| {
                    metadata.is_file() && metadata.len() == definition.download.files[index].bytes
                })
            })
            .then_some(assets)
    }
}

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type,
)]
#[serde(rename_all = "camelCase")]
pub enum DownloadStatus {
    #[default]
    NotDownloaded,
    Downloading,
    Verifying,
    Paused,
    Downloaded,
    Failed,
    Removing,
}

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type,
)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeStatus {
    #[default]
    Sleeping,
    Loading,
    Ready,
    Processing,
    Failed,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LocalModelSnapshot {
    pub supports_text: bool,
    pub(crate) minimum_ui_mode: crate::config::UiMode,
    pub id: String,
    pub name: String,
    pub downloaded: bool,
    pub repository_url: String,
    // The license file at the pinned revision, or the repository page when the
    // catalog names no license file.
    pub license_url: String,
    pub download_status: DownloadStatus,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub runtime_status: RuntimeStatus,
    pub in_use: bool,
    pub error: Option<String>,
}

impl ModelStore {
    pub fn available_on_demand(&self, id: &str) -> bool {
        supported() && definition(id).is_some()
    }
    pub(crate) fn partial_directory(&self, id: &str, bundle: &DownloadBundle) -> PathBuf {
        self.root
            .join(id)
            .join(format!(".partial-{}", bundle.revision))
    }
    pub fn partial_bytes(&self, id: &str) -> u64 {
        definition(id)
            .map(|model| &model.download)
            .map_or(0, |bundle| {
                let stage = self.partial_directory(id, bundle);
                bundle
                    .files
                    .iter()
                    .map(|file| {
                        let complete = stage
                            .join(&file.name)
                            .metadata()
                            .map_or(0, |metadata| metadata.len().min(file.bytes));
                        if complete > 0 {
                            complete
                        } else {
                            stage
                                .join(format!("{}.part", file.name))
                                .metadata()
                                .map_or(0, |metadata| metadata.len().min(file.bytes))
                        }
                    })
                    .sum()
            })
    }
    pub(crate) async fn remove(&self, id: &str) -> Result<(), String> {
        let model = definition(id).ok_or("Unknown local model.")?;
        let directory = self.root.join(&model.id);
        match tokio::fs::symlink_metadata(&directory).await {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                tokio::fs::remove_file(directory).await
            }
            Ok(_) => tokio::fs::remove_dir_all(directory).await,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(format!("Could not remove the model download: {error}")),
        }
        .map_err(|error| format!("Could not remove the model download: {error}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_complete_pinned_bundles_are_resolved() {
        if !supported() {
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let store = ModelStore::new(temp.path().into());
        for model in MODELS.iter() {
            let base = temp.path().join(&model.id);
            std::fs::create_dir_all(&base).unwrap();
            for file in &model.download.files {
                std::fs::File::create(base.join(&file.name))
                    .unwrap()
                    .set_len(file.bytes)
                    .unwrap();
            }
            assert!(store.resolve(&model.id).is_none());
            let pinned = base.join(&model.download.revision);
            std::fs::create_dir_all(&pinned).unwrap();
            let model_file = &model.download.files[0];
            std::fs::rename(base.join(&model_file.name), pinned.join(&model_file.name)).unwrap();
            assert!(store.resolve(&model.id).is_none());
            let projector = &model.download.files[1];
            std::fs::rename(base.join(&projector.name), pinned.join(&projector.name)).unwrap();
            assert!(store.resolve(&model.id).is_some());
            std::fs::OpenOptions::new()
                .write(true)
                .open(pinned.join(&projector.name))
                .unwrap()
                .set_len(projector.bytes - 1)
                .unwrap();
            assert!(store.resolve(&model.id).is_none());
        }
    }
    #[tokio::test]
    async fn removal_is_scoped_to_the_curated_model_and_rejects_paths() {
        let temp = tempfile::tempdir().unwrap();
        let store = ModelStore::new(temp.path().into());
        let model = temp.path().join("isocr1-0.8b");
        std::fs::create_dir_all(model.join("partial")).unwrap();
        std::fs::write(model.join("partial/file.part"), b"partial").unwrap();
        std::fs::write(temp.path().join("keep"), b"other assets").unwrap();
        assert!(store.remove("../keep").await.is_err());
        store.remove("isocr1-0.8b").await.unwrap();
        assert!(!model.exists());
        assert!(temp.path().join("keep").exists());
    }
}
