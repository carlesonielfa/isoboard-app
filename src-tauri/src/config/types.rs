//! Configuration schema, validation errors, and frontend snapshots.

use super::bindings::{validate_bindings, ShortcutBinding};
use crate::inference_profiles::{InferenceConfig, InferenceSnapshot};
use crate::profiles::{
    default_processing_config, normalize_additional_rules, validate_profile_id,
    validate_profile_name as validate_profile_name_value, ProcessingConfig, ProcessingSnapshot,
};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

pub const CONFIG_VERSION: u32 = 1;
pub const DEFAULT_CAPTURE_SHORTCUT: &str = "CommandOrControl+Shift+2";
/// A layout-stable, one-handed shortcut for revealing the isoboard window.
pub const DEFAULT_OPEN_SHORTCUT: &str = "CommandOrControl+Alt+KeyZ";

/// Prevents the capture callback from running while the active shortcut is
/// temporarily unregistered for recording.
#[derive(Default)]
pub struct ShortcutRecordingState(AtomicBool);

impl ShortcutRecordingState {
    pub fn set_recording(&self, recording: bool) {
        self.0.store(recording, Ordering::Release);
    }

    pub(super) fn is_recording(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(default)]
pub(crate) struct AppConfig {
    #[serde(default = "default_config_version")]
    pub(crate) version: u32,
    pub(crate) capture: CaptureConfig,
    pub(crate) shortcuts: ShortcutConfig,
    pub(crate) ui: UiConfig,
    pub(crate) updates: UpdatesConfig,
    pub(crate) history: HistoryConfig,
    pub(crate) processing: ProcessingConfig,
    pub(crate) inference: InferenceConfig,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub(crate) enum UiMode {
    #[default]
    Basic,
    Advanced,
    Debug,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(default)]
pub(crate) struct UiConfig {
    pub(crate) mode: UiMode,
    pub(crate) sounds_enabled: bool,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            mode: UiMode::default(),
            sounds_enabled: true,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(default)]
pub(crate) struct UpdatesConfig {
    pub(crate) check_automatically: bool,
}

impl Default for UpdatesConfig {
    fn default() -> Self {
        Self {
            check_automatically: true,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(default)]
pub(crate) struct HistoryConfig {
    /// Keeps full-resolution captures of recent entries for reprocessing.
    pub(crate) keep_originals: bool,
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            keep_originals: true,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(default)]
pub(crate) struct ShortcutConfig {
    #[serde(default = "default_open_shortcut")]
    pub(crate) open: String,
}

impl Default for ShortcutConfig {
    fn default() -> Self {
        Self {
            open: default_open_shortcut(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(default)]
pub(crate) struct CaptureConfig {
    #[serde(default = "default_bindings")]
    pub(crate) bindings: Vec<ShortcutBinding>,
    pub(crate) overlay_enabled: bool,
    pub(crate) overlay_position: OverlayPosition,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub(crate) enum OverlayPosition {
    TopLeft,
    TopCenter,
    TopRight,
    MiddleLeft,
    Center,
    MiddleRight,
    BottomLeft,
    #[default]
    BottomCenter,
    BottomRight,
}

fn default_config_version() -> u32 {
    CONFIG_VERSION
}

fn default_open_shortcut() -> String {
    DEFAULT_OPEN_SHORTCUT.to_string()
}

fn default_bindings() -> Vec<ShortcutBinding> {
    vec![super::bindings::default_binding()]
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            bindings: default_bindings(),
            overlay_enabled: true,
            overlay_position: OverlayPosition::default(),
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            capture: CaptureConfig::default(),
            shortcuts: ShortcutConfig {
                open: default_open_shortcut(),
            },
            ui: UiConfig::default(),
            updates: UpdatesConfig::default(),
            history: HistoryConfig::default(),
            processing: default_processing_config(),
            inference: InferenceConfig::default(),
        }
    }
}

impl AppConfig {
    pub(super) fn validate(&self, path: &Path) -> Result<(), ConfigError> {
        if self.version != CONFIG_VERSION {
            return Err(ConfigError::validation(
                path,
                format!(
                    "unsupported config version {}; expected {CONFIG_VERSION}",
                    self.version
                ),
                None,
            ));
        }

        if self.processing.profiles.is_empty() {
            return Err(ConfigError::validation(
                path,
                "processing must contain at least one profile",
                None,
            ));
        }
        if !self
            .processing
            .profiles
            .contains_key(&self.processing.active_profile)
        {
            return Err(ConfigError::validation(
                path,
                format!(
                    "active profile {:?} does not exist",
                    self.processing.active_profile
                ),
                None,
            ));
        }

        let mut names = std::collections::HashSet::new();
        for (id, profile) in &self.processing.profiles {
            validate_profile_id(id)
                .map_err(|message| ConfigError::validation(path, message, None))?;
            validate_profile_name(path, &profile.name)?;
            normalize_additional_rules(&profile.additional_rules)
                .map_err(|message| ConfigError::validation(path, message, None))?;
            if !names.insert(profile.name.trim().to_lowercase()) {
                return Err(ConfigError::validation(
                    path,
                    format!("duplicate profile name {:?}", profile.name),
                    None,
                ));
            }
        }

        validate_bindings(self, path)?;

        let open_id =
            super::bindings::native_shortcut_id(&self.shortcuts.open).map_err(|error| {
                ConfigError::validation(
                    path,
                    format!("invalid open shortcut {:?}: {error}", self.shortcuts.open),
                    Some(self.shortcuts.open.clone()),
                )
            })?;
        if self
            .capture
            .bindings
            .iter()
            .filter_map(|binding| super::bindings::native_shortcut_id(&binding.shortcut).ok())
            .any(|capture_id| capture_id == open_id)
        {
            return Err(ConfigError::validation(
                path,
                format!(
                    "open shortcut {:?} conflicts with a capture shortcut",
                    self.shortcuts.open
                ),
                Some(self.shortcuts.open.clone()),
            ));
        }

        self.inference
            .validate(path)
            .map_err(|message| ConfigError::validation(path, message, None))?;
        for profile in self.processing.profiles.values() {
            for settings in [&profile.model_settings, &profile.rules_model_settings]
                .into_iter()
                .flatten()
            {
                settings
                    .validate()
                    .map_err(|message| ConfigError::validation(path, message, None))?;
            }
        }

        Ok(())
    }
}

pub(super) fn validate_profile_name(path: &Path, name: &str) -> Result<(), ConfigError> {
    validate_profile_name_value(name)
        .map(|_| ())
        .map_err(|message| ConfigError::validation(path, message, None))
}

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "config-changed")]
pub(crate) struct ConfigSnapshot {
    pub(crate) version: u32,
    pub(crate) capture_bindings: Vec<CaptureBindingSnapshot>,
    pub(crate) open_shortcut: String,
    pub(crate) ui_mode: UiMode,
    pub(crate) sounds_enabled: bool,
    pub(crate) keep_history_originals: bool,
    pub(crate) automatic_update_checks: bool,
    pub(crate) overlay_enabled: bool,
    pub(crate) overlay_position: OverlayPosition,
    pub(crate) config_path: String,
    pub(crate) registered: bool,
    pub(crate) registration_error: Option<String>,
    pub(crate) last_error: Option<ConfigError>,
    pub(crate) revision: u64,
    pub(crate) processing: ProcessingSnapshot,
    pub(crate) inference: InferenceSnapshot,
}

impl ConfigSnapshot {
    pub(crate) fn needs_initial_setup(&self) -> bool {
        let Some(active) = self
            .processing
            .profiles
            .iter()
            .find(|profile| profile.id == self.processing.active_profile_id)
        else {
            return true;
        };
        let rules = if active.additional_rules_separate_pass
            && !active.additional_rules.trim().is_empty()
        {
            match active.rules_model_mode {
                crate::profiles::RulesModelMode::Same => active.model_settings.as_ref(),
                crate::profiles::RulesModelMode::Independent => {
                    active.rules_model_settings.as_ref()
                }
            }
        } else {
            active.model_settings.as_ref()
        };
        [active.model_settings.as_ref(), rules]
            .into_iter()
            .any(|settings| {
                settings.is_none_or(|settings| {
                    settings.provider == crate::inference_profiles::InferenceBackend::Local
                        && settings.local_model_downloaded != Some(true)
                })
            })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CaptureBindingSnapshot {
    pub(crate) id: String,
    pub(crate) shortcut: String,
    pub(crate) profile_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ConfigErrorKind {
    Io,
    Parse,
    Validation,
    Conflict,
    Runtime,
}

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "config-error")]
pub(crate) struct ConfigError {
    pub(crate) kind: ConfigErrorKind,
    pub(crate) message: Box<str>,
    pub(crate) config_path: Box<str>,
    pub(crate) shortcut: Option<Box<str>>,
    pub(crate) binding_id: Option<Box<str>>,
    pub(crate) current_bindings: Box<[CaptureBindingSnapshot]>,
    pub(crate) current_open_shortcut: Box<str>,
    pub(crate) registered: bool,
    pub(crate) revision: u64,
}

impl Display for ConfigError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ConfigError {}

impl ConfigError {
    fn path_string(path: &Path) -> Box<str> {
        path.display().to_string().into_boxed_str()
    }

    pub(super) fn io(path: &Path, message: impl Into<String>) -> Self {
        Self {
            kind: ConfigErrorKind::Io,
            message: message.into().into_boxed_str(),
            config_path: Self::path_string(path),
            shortcut: None,
            binding_id: None,
            current_bindings: Vec::new().into_boxed_slice(),
            current_open_shortcut: DEFAULT_OPEN_SHORTCUT.into(),
            registered: false,
            revision: 0,
        }
    }

    pub(super) fn parse(path: &Path, message: impl Into<String>) -> Self {
        Self {
            kind: ConfigErrorKind::Parse,
            message: message.into().into_boxed_str(),
            config_path: Self::path_string(path),
            shortcut: None,
            binding_id: None,
            current_bindings: Vec::new().into_boxed_slice(),
            current_open_shortcut: DEFAULT_OPEN_SHORTCUT.into(),
            registered: false,
            revision: 0,
        }
    }

    pub(super) fn validation(
        path: &Path,
        message: impl Into<String>,
        shortcut: Option<String>,
    ) -> Self {
        Self {
            kind: ConfigErrorKind::Validation,
            message: message.into().into_boxed_str(),
            config_path: Self::path_string(path),
            shortcut: shortcut.map(String::into_boxed_str),
            binding_id: None,
            current_bindings: Vec::new().into_boxed_slice(),
            current_open_shortcut: DEFAULT_OPEN_SHORTCUT.into(),
            registered: false,
            revision: 0,
        }
    }

    pub(super) fn conflict(
        path: &Path,
        message: impl Into<String>,
        shortcut: Option<String>,
    ) -> Self {
        Self {
            kind: ConfigErrorKind::Conflict,
            message: message.into().into_boxed_str(),
            config_path: Self::path_string(path),
            shortcut: shortcut.map(String::into_boxed_str),
            binding_id: None,
            current_bindings: Vec::new().into_boxed_slice(),
            current_open_shortcut: DEFAULT_OPEN_SHORTCUT.into(),
            registered: false,
            revision: 0,
        }
    }

    pub(super) fn runtime(
        path: &Path,
        message: impl Into<String>,
        shortcut: Option<String>,
    ) -> Self {
        Self {
            kind: ConfigErrorKind::Runtime,
            message: message.into().into_boxed_str(),
            config_path: Self::path_string(path),
            shortcut: shortcut.map(String::into_boxed_str),
            binding_id: None,
            current_bindings: Vec::new().into_boxed_slice(),
            current_open_shortcut: DEFAULT_OPEN_SHORTCUT.into(),
            registered: false,
            revision: 0,
        }
    }
}

impl AppConfig {
    /// Seed each connected provider once; the TOML ledger survives profile deletion.
    pub(super) fn provision_profiles(&mut self) -> bool {
        let mut changed = false;
        for backend in crate::inference_profiles::InferenceBackend::ALL {
            if !self
                .inference
                .providers
                .get(backend.id())
                .is_some_and(|p| p.credential_ref.is_some())
                || self.inference.provisioned_providers.contains(&backend)
            {
                continue;
            }
            self.inference.provisioned_providers.push(backend);
            changed = true;
            for seed in crate::builtin_inference_profiles::profiles()
                .iter()
                .filter(|s| s.backend == backend)
            {
                let id = crate::profiles::next_profile_id(&self.processing);
                let mut name = seed.name.to_owned();
                let mut suffix = 2;
                while self
                    .processing
                    .profiles
                    .values()
                    .any(|p| p.name.eq_ignore_ascii_case(&name))
                {
                    name = format!("{} {}", seed.name, suffix);
                    suffix += 1;
                }
                let mut profile = crate::profiles::default_processing_config()
                    .profiles
                    .remove(crate::profiles::DEFAULT_PROFILE_ID)
                    .expect("default profile");
                profile.name = name;
                profile.model_settings = Some(crate::inference_profiles::ModelSettings {
                    provider: backend,
                    model: seed.model.to_owned(),
                    options: None,
                });
                self.processing.profiles.insert(id, profile);
            }
        }
        changed
    }
}
