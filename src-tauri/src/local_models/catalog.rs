use std::{collections::HashSet, sync::LazyLock};

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelDefinition {
    pub id: String,
    pub name: String,
    pub supports_text: bool,
    pub(crate) minimum_ui_mode: crate::config::UiMode,
    prompt: PromptPreset,
    pub download: DownloadBundle,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum PromptPreset {
    Standard,
    Isocr,
}

impl ModelDefinition {
    pub fn extraction_prompt(&self) -> Option<&'static str> {
        match self.prompt {
            PromptPreset::Standard => None,
            PromptPreset::Isocr => Some(include_str!("../inference/isocr.jinja")),
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DownloadFile {
    pub name: String,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DownloadBundle {
    pub repo: String,
    pub revision: String,
    // Repository file that holds the model's license, linked before download.
    #[serde(default)]
    pub license_file: Option<String>,
    // The model comes first, followed by its matching vision projector.
    pub files: [DownloadFile; 2],
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalog {
    version: u32,
    models: Vec<ModelDefinition>,
}

fn component(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c))
}

fn hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

fn parse(source: &str) -> Result<Vec<ModelDefinition>, String> {
    let catalog: Catalog = serde_json::from_str(source).map_err(|error| error.to_string())?;
    if catalog.version != 1 || catalog.models.is_empty() {
        return Err("Expected a nonempty version-one model catalog.".into());
    }
    let mut ids = HashSet::new();
    for model in &catalog.models {
        let bundle = &model.download;
        let repo: Vec<_> = bundle.repo.split('/').collect();
        if !component(&model.id)
            || !ids.insert(&model.id)
            || model.name.trim().is_empty()
            || repo.len() != 2
            || !repo.iter().all(|part| component(part))
            || !hex(&bundle.revision, 40)
            || bundle
                .license_file
                .as_deref()
                .is_some_and(|file| !component(file))
            || bundle.files[0].name == bundle.files[1].name
            || bundle.files.iter().any(|file| {
                !component(&file.name)
                    || !file.name.ends_with(".gguf")
                    || file.bytes == 0
                    || !hex(&file.sha256, 64)
            })
            || bundle.files[0]
                .bytes
                .checked_add(bundle.files[1].bytes)
                .is_none()
        {
            return Err(format!("Invalid model catalog entry: {}", model.id));
        }
    }
    Ok(catalog.models)
}

pub static MODELS: LazyLock<Vec<ModelDefinition>> = LazyLock::new(|| {
    parse(include_str!("catalog.json")).expect("Bundled local model catalog must be valid")
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_preserves_identity_and_rejects_unsafe_or_unpinned_entries() {
        let source = include_str!("catalog.json");
        let models = parse(source).unwrap();
        let isocr = models
            .iter()
            .find(|model| model.id == "isocr1-0.8b")
            .unwrap();
        assert_eq!(
            isocr.extraction_prompt(),
            Some(include_str!("../inference/isocr.jinja"))
        );
        assert!(!isocr.supports_text);
        assert_eq!(isocr.minimum_ui_mode, crate::config::UiMode::Basic);
        assert_eq!(
            models
                .iter()
                .find(|model| model.id == "qwen3.5-2b")
                .unwrap()
                .minimum_ui_mode,
            crate::config::UiMode::Debug
        );
        let original: serde_json::Value = serde_json::from_str(source).unwrap();
        for (pointer, value) in [
            ("/version", serde_json::json!(2)),
            ("/models/0/minimum_ui_mode", serde_json::json!("unknown")),
            ("/models/0/id", serde_json::json!("../escape")),
            ("/models/0/download/revision", serde_json::json!("main")),
            (
                "/models/0/download/files/0/name",
                serde_json::json!("../model.gguf"),
            ),
            ("/models/0/download/files/0/bytes", serde_json::json!(0)),
            (
                "/models/0/download/files/0/sha256",
                serde_json::json!("missing"),
            ),
            ("/models/0/prompt", serde_json::json!("unknown")),
            (
                "/models/0/download/license_file",
                serde_json::json!("../LICENSE.md"),
            ),
        ] {
            let mut invalid = original.clone();
            *invalid.pointer_mut(pointer).unwrap() = value;
            assert!(parse(&invalid.to_string()).is_err(), "{pointer}");
        }
        let mut duplicate = original.clone();
        duplicate["models"]
            .as_array_mut()
            .unwrap()
            .push(original["models"][0].clone());
        assert!(parse(&duplicate.to_string()).is_err());
    }
}
