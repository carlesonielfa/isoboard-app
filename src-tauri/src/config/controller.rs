//! Transactional in-memory configuration controller.

use super::bindings::{native_shortcut_id, CaptureInvocation, ShortcutBinding, DEFAULT_BINDING_ID};
use super::storage::{parse_config, write_config, LoadedConfig};
use super::types::*;
use crate::inference::FrozenInference;
use crate::profiles::{
    next_profile_id, ConversionSettings, ConversionSettingsApplicationMode, ProcessingProfileDraft,
    ProcessingProfileSnapshot, ProcessingSnapshot, PromptTemplateDefaults,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub trait ShortcutRuntime: Send {
    fn register(&mut self, shortcut: &str) -> Result<(), String>;
    fn unregister(&mut self, shortcut: &str) -> Result<(), String>;
}

#[derive(Debug, PartialEq)]
pub enum ConfigTransition {
    Unchanged(ConfigSnapshot),
    Changed(ConfigSnapshot),
    Rejected(ConfigError),
}

pub struct ConfigController {
    pub(super) path: PathBuf,
    pub(super) current: AppConfig,
    /// Native IDs identify registrations; strings are retained for runtime calls.
    active_shortcuts: BTreeMap<u32, String>,
    registration_suspended: bool,
    last_error: Option<ConfigError>,
    revision: u64,
    runtime: Box<dyn ShortcutRuntime>,
}

pub type SharedConfig = std::sync::Arc<std::sync::Mutex<ConfigController>>;

impl ConfigController {
    pub fn load_or_default(
        path: impl Into<PathBuf>,
        runtime: Box<dyn ShortcutRuntime>,
    ) -> Result<(Self, Option<ConfigError>), ConfigError> {
        let path = path.into();
        let (mut current, startup_error) = match fs::read_to_string(&path) {
            Ok(raw) => match parse_config(&path, &raw) {
                Ok(loaded) => (loaded.config, None),
                Err(error) => (AppConfig::default(), Some(error)),
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let config = AppConfig::default();
                write_config(&path, &config)?;
                (config, None)
            }
            Err(error) => {
                return Err(ConfigError::io(
                    &path,
                    format!("could not read config {}: {error}", path.display()),
                ));
            }
        };
        let mut guided = false;
        for profile in current.processing.profiles.values_mut() {
            guided |= profile.normalize_models();
        }
        let provisioned = startup_error.is_none() && current.provision_profiles();
        if startup_error.is_none() && (guided || provisioned) {
            write_config(&path, &current)?;
        }
        Ok((
            Self {
                path,
                current,
                active_shortcuts: BTreeMap::new(),
                registration_suspended: false,
                last_error: None,
                revision: 0,
                runtime,
            },
            startup_error,
        ))
    }

    pub fn snapshot(&self) -> ConfigSnapshot {
        ConfigSnapshot {
            version: self.current.version,
            capture_bindings: self.capture_binding_snapshots(),
            open_shortcut: self.current.shortcuts.open.clone(),
            ui_mode: self.current.ui.mode,
            sounds_enabled: self.current.ui.sounds_enabled,
            keep_history_originals: self.current.history.keep_originals,
            automatic_update_checks: self.current.updates.check_automatically,
            overlay_position: self.current.capture.overlay_position,
            config_path: self.path.display().to_string(),
            registered: self.is_fully_registered(),
            registration_error: self
                .last_error
                .as_ref()
                .map(|error| error.message.to_string()),
            last_error: self.last_error.clone(),
            revision: self.revision,
            processing: ProcessingSnapshot {
                active_profile_id: self.current.processing.active_profile.clone(),
                profiles: self
                    .current
                    .processing
                    .profiles
                    .iter()
                    .map(|(id, profile)| ProcessingProfileSnapshot {
                        id: id.clone(),
                        name: profile.name.clone(),
                        model_settings: profile
                            .model_settings
                            .as_ref()
                            .map(|m| m.snapshot(&self.path)),
                        rules_model_settings: profile
                            .rules_model_settings
                            .as_ref()
                            .map(|m| m.snapshot(&self.path)),
                        rules_model_mode: profile.rules_model_mode,
                        conversion: profile.conversion.clone(),
                        additional_rules: profile.additional_rules.clone(),
                        additional_rules_separate_pass: profile.additional_rules_separate_pass,
                        application_mode_override: profile.application_mode_override,
                        conversion_prompt_template: profile.conversion_prompt_template.clone(),
                        postprocessing_prompt_template: profile
                            .postprocessing_prompt_template
                            .clone(),
                    })
                    .collect(),
                prompt_template_defaults: PromptTemplateDefaults {
                    conversion: crate::inference::params::default_conversion_prompt_template()
                        .to_owned(),
                    postprocessing:
                        crate::inference::params::default_postprocessing_prompt_template()
                            .to_owned(),
                },
            },
            inference: crate::inference_profiles::InferenceSnapshot::from(&self.current.inference),
        }
    }

    pub fn config_path(&self) -> &Path {
        &self.path
    }

    pub fn activate(&mut self) -> Result<ConfigSnapshot, ConfigError> {
        if self.registration_suspended {
            return Ok(self.snapshot());
        }
        let desired = self.desired_shortcuts(&self.current)?;
        if self.active_shortcuts != desired {
            self.sync_runtime(&desired)
                .map_err(|error| self.remember_error(error))?;
            self.clear_runtime_error();
            self.revision = self.revision.saturating_add(1);
        }
        Ok(self.snapshot())
    }

    pub fn resolve_capture_shortcut_id(&self, native_id: u32) -> Option<CaptureInvocation> {
        self.current
            .capture
            .bindings
            .iter()
            .find(|binding| native_shortcut_id(&binding.shortcut).ok() == Some(native_id))
            .map(|binding| CaptureInvocation {
                binding_id: binding.id.clone(),
                profile_id: binding
                    .profile_id
                    .clone()
                    .unwrap_or_else(|| self.current.processing.active_profile.clone()),
            })
    }

    pub fn is_open_shortcut_id(&self, native_id: u32) -> bool {
        native_shortcut_id(&self.current.shortcuts.open).ok() == Some(native_id)
    }

    pub fn active_profile_context(&self) -> String {
        self.current.processing.active_profile.clone()
    }

    /// Copy every run control under the caller's single configuration lock.
    pub(crate) fn resolve_capture_context(
        &self,
        profile_id: &str,
        binding_id: Option<String>,
    ) -> Option<crate::capture::CaptureContext> {
        let profile = self.current.processing.profiles.get(profile_id)?;
        let (conversion, application_mode) = self.resolve_conversion_for_processing(profile_id)?;
        let inference = self.resolve_inference_for_processing(profile_id);
        let postprocessing_inference = match profile.rules_model_mode {
            crate::profiles::RulesModelMode::Independent => {
                self.resolve_model_settings(profile_id, profile.rules_model_settings.as_ref())
            }
            crate::profiles::RulesModelMode::Same => inference.clone(),
        };
        Some(crate::capture::CaptureContext {
            profile_id: profile_id.to_owned(),
            profile_name: profile.name.clone(),
            binding_id,
            inference,
            // The persisted mode selects either the independent settings or
            // the first frozen chain; inactive settings remain remembered.
            postprocessing_inference,
            conversion,
            application_mode,
            additional_rules: profile.additional_rules.clone(),
            additional_rules_separate_pass: profile.additional_rules_separate_pass,
            conversion_prompt_template: profile.conversion_prompt_template.clone(),
            postprocessing_prompt_template: profile.postprocessing_prompt_template.clone(),
            keep_history_original: self.current.history.keep_originals,
        })
    }

    /// Resolve the profile-owned model and shared provider settings while
    /// the configuration lock is held. The returned selection is immutable
    /// for the lifetime of one capture and contains no secret bytes.
    pub(crate) fn resolve_inference_for_processing(
        &self,
        processing_profile_id: &str,
    ) -> Option<FrozenInference> {
        let processing = self
            .current
            .processing
            .profiles
            .get(processing_profile_id)?;
        self.resolve_model_settings(processing_profile_id, processing.model_settings.as_ref())
    }

    fn resolve_model_settings(
        &self,
        processing_profile_id: &str,
        settings: Option<&crate::inference_profiles::ModelSettings>,
    ) -> Option<FrozenInference> {
        let inference = settings?;
        let provider = self
            .current
            .inference
            .providers
            .get(inference.provider.id());
        let credential_ref = provider
            .and_then(|provider| provider.credential_ref.as_deref())
            .and_then(|reference| crate::secret_store::ManagedSecretRef::parse(reference).ok());
        Some(FrozenInference {
            processing_profile_id: processing_profile_id.to_owned(),
            backend: inference.provider,
            model: inference.model.clone(),
            options: inference.options.clone().unwrap_or_default(),
            endpoint: provider.and_then(|provider| provider.endpoint.clone()),
            credential_ref,
        })
    }

    /// Freeze conversion controls and the effective application mode while the
    /// configuration lock is held at capture start.
    pub(crate) fn resolve_conversion_for_processing(
        &self,
        processing_profile_id: &str,
    ) -> Option<(ConversionSettings, ConversionSettingsApplicationMode)> {
        let profile = self
            .current
            .processing
            .profiles
            .get(processing_profile_id)?;
        let backend = profile
            .model_settings
            .as_ref()
            .map(|m| m.provider)
            .unwrap_or(crate::inference_profiles::InferenceBackend::OpenaiApi);
        Some((
            profile.conversion.clone(),
            crate::inference::params::effective_application_mode(
                profile.application_mode_override,
                backend,
            ),
        ))
    }

    pub fn suspend_capture_shortcut(&mut self) -> Result<(), ConfigError> {
        if self.registration_suspended {
            return Ok(());
        }
        let active = std::mem::take(&mut self.active_shortcuts);
        let mut remaining = active.clone();
        for (native_id, shortcut) in active {
            if let Err(error) = self.runtime.unregister(&shortcut) {
                self.active_shortcuts = remaining;
                return Err(self.remember_error(ConfigError::runtime(
                    &self.path,
                    format!("could not pause shortcut {:?}: {error}", shortcut),
                    Some(shortcut),
                )));
            }
            remaining.remove(&native_id);
        }
        self.active_shortcuts = remaining;
        self.registration_suspended = true;
        Ok(())
    }

    pub fn resume_capture_shortcut(&mut self) -> Result<ConfigSnapshot, ConfigError> {
        if !self.registration_suspended && self.is_fully_registered() {
            return Ok(self.snapshot());
        }
        let desired = self.desired_shortcuts(&self.current)?;
        self.sync_runtime(&desired)
            .map_err(|error| self.remember_error(error))?;
        self.registration_suspended = false;
        self.clear_runtime_error();
        Ok(self.snapshot())
    }

    pub fn set_open_shortcut(
        &mut self,
        shortcut: String,
        expected_revision: u64,
    ) -> Result<ConfigSnapshot, ConfigError> {
        self.check_profile_revision(expected_revision)?;
        let mut candidate = self.current.clone();
        candidate.shortcuts.open = shortcut;
        self.apply_candidate(candidate, true)
    }

    pub(crate) fn sounds_enabled(&self) -> bool {
        self.current.ui.sounds_enabled
    }

    pub fn set_sounds_enabled(
        &mut self,
        enabled: bool,
        expected_revision: u64,
    ) -> Result<ConfigSnapshot, ConfigError> {
        self.check_profile_revision(expected_revision)?;
        let mut candidate = self.current.clone();
        candidate.ui.sounds_enabled = enabled;
        self.apply_candidate(candidate, true)
    }

    pub fn set_keep_history_originals(
        &mut self,
        enabled: bool,
        expected_revision: u64,
    ) -> Result<ConfigSnapshot, ConfigError> {
        self.check_profile_revision(expected_revision)?;
        let mut candidate = self.current.clone();
        candidate.history.keep_originals = enabled;
        self.apply_candidate(candidate, true)
    }

    pub(crate) fn automatic_update_checks(&self) -> bool {
        self.current.updates.check_automatically
    }

    pub fn set_automatic_update_checks(
        &mut self,
        enabled: bool,
        expected_revision: u64,
    ) -> Result<ConfigSnapshot, ConfigError> {
        self.check_profile_revision(expected_revision)?;
        let mut candidate = self.current.clone();
        candidate.updates.check_automatically = enabled;
        self.apply_candidate(candidate, true)
    }

    pub fn set_ui_mode(
        &mut self,
        mode: UiMode,
        expected_revision: u64,
    ) -> Result<ConfigSnapshot, ConfigError> {
        self.check_profile_revision(expected_revision)?;
        let mut candidate = self.current.clone();
        candidate.ui.mode = mode;
        self.apply_candidate(candidate, true)
    }

    pub fn set_overlay_position(
        &mut self,
        position: super::types::OverlayPosition,
        expected_revision: u64,
    ) -> Result<ConfigSnapshot, ConfigError> {
        self.check_profile_revision(expected_revision)?;
        let mut candidate = self.current.clone();
        candidate.capture.overlay_position = position;
        self.apply_candidate(candidate, true)
    }

    pub fn create_binding(
        &mut self,
        profile_id: String,
        shortcut: String,
        expected_revision: u64,
    ) -> Result<ConfigSnapshot, ConfigError> {
        self.check_profile_revision(expected_revision)?;
        if !self.current.processing.profiles.contains_key(&profile_id) {
            return Err(self.remember_error(ConfigError::validation(
                &self.path,
                format!("profile {:?} does not exist", profile_id),
                None,
            )));
        }
        let id = next_binding_id(&self.current);
        let mut candidate = self.current.clone();
        candidate.capture.bindings.push(ShortcutBinding {
            id,
            shortcut,
            profile_id: Some(profile_id),
        });
        self.apply_candidate(candidate, true)
    }

    pub fn set_binding(
        &mut self,
        id: String,
        shortcut: String,
        expected_revision: u64,
    ) -> Result<ConfigSnapshot, ConfigError> {
        self.check_profile_revision(expected_revision)?;
        let mut candidate = self.current.clone();
        let Some(binding) = candidate
            .capture
            .bindings
            .iter_mut()
            .find(|item| item.id == id)
        else {
            return Err(self.remember_error(ConfigError::validation(
                &self.path,
                format!("capture binding {:?} does not exist", id),
                Some(shortcut),
            )));
        };
        binding.shortcut = shortcut;
        self.apply_candidate(candidate, true)
    }

    pub fn set_binding_profile(
        &mut self,
        id: String,
        profile_id: String,
        expected_revision: u64,
    ) -> Result<ConfigSnapshot, ConfigError> {
        self.check_profile_revision(expected_revision)?;
        let mut candidate = self.current.clone();
        let Some(binding) = candidate
            .capture
            .bindings
            .iter_mut()
            .find(|item| item.id == id)
        else {
            return Err(self.remember_error(ConfigError::validation(
                &self.path,
                format!("capture binding {:?} does not exist", id),
                None,
            )));
        };
        if id == DEFAULT_BINDING_ID {
            return Err(self.remember_error(ConfigError::validation(
                &self.path,
                "the default capture binding always uses the active profile",
                None,
            )));
        }
        if !self.current.processing.profiles.contains_key(&profile_id) {
            return Err(self.remember_error(ConfigError::validation(
                &self.path,
                format!("profile {:?} does not exist", profile_id),
                None,
            )));
        }
        if binding.profile_id.is_none() {
            return Err(self.remember_error(ConfigError::validation(
                &self.path,
                format!("capture binding {:?} must target a fixed profile", id),
                None,
            )));
        }
        binding.profile_id = Some(profile_id);
        self.apply_candidate(candidate, true)
    }

    pub fn delete_binding(
        &mut self,
        id: String,
        expected_revision: u64,
    ) -> Result<ConfigSnapshot, ConfigError> {
        self.check_profile_revision(expected_revision)?;
        if id == DEFAULT_BINDING_ID {
            return Err(self.remember_error(ConfigError::validation(
                &self.path,
                "the default capture binding cannot be deleted",
                None,
            )));
        }
        let mut candidate = self.current.clone();
        let before = candidate.capture.bindings.len();
        candidate
            .capture
            .bindings
            .retain(|binding| binding.id != id);
        if candidate.capture.bindings.len() == before {
            return Err(self.remember_error(ConfigError::validation(
                &self.path,
                format!("capture binding {:?} does not exist", id),
                None,
            )));
        }
        self.apply_candidate(candidate, true)
    }

    #[cfg(test)]
    pub fn create_processing_profile(
        &mut self,
        name: String,
        expected_revision: u64,
    ) -> Result<ConfigSnapshot, ConfigError> {
        self.create_processing_profile_draft(
            ProcessingProfileDraft {
                name,
                model_settings: None,
                rules_model_settings: None,
                rules_model_mode: crate::profiles::RulesModelMode::Same,
                conversion: Default::default(),
                additional_rules: String::new(),
                additional_rules_separate_pass: false,
                application_mode_override: None,
                conversion_prompt_template: None,
                postprocessing_prompt_template: None,
            },
            expected_revision,
        )
    }

    pub fn create_processing_profile_draft(
        &mut self,
        draft: ProcessingProfileDraft,
        expected_revision: u64,
    ) -> Result<ConfigSnapshot, ConfigError> {
        self.check_profile_revision(expected_revision)?;
        let mut profile = draft.into_profile().map_err(|message| {
            self.remember_error(ConfigError::validation(&self.path, message, None))
        })?;
        profile.name = profile.name.trim().to_string();
        validate_profile_name(&self.path, &profile.name)
            .map_err(|error| self.remember_error(error))?;
        self.ensure_profile_name_available(&profile.name)?;
        let mut candidate = self.current.clone();
        let id = next_profile_id(&candidate.processing);
        candidate.processing.profiles.insert(id, profile);
        self.apply_candidate(candidate, true)
    }

    pub fn update_processing_profile(
        &mut self,
        id: String,
        draft: ProcessingProfileDraft,
        expected_revision: u64,
    ) -> Result<ConfigSnapshot, ConfigError> {
        self.check_profile_revision(expected_revision)?;
        let mut profile = draft.into_profile().map_err(|message| {
            self.remember_error(ConfigError::validation(&self.path, message, None))
        })?;
        profile.name = profile.name.trim().to_string();
        validate_profile_name(&self.path, &profile.name)
            .map_err(|error| self.remember_error(error))?;
        self.ensure_profile_name_available_except(&profile.name, &id)?;
        let mut candidate = self.current.clone();
        if !candidate.processing.profiles.contains_key(&id) {
            return Err(self.remember_error(ConfigError::validation(
                &self.path,
                format!("profile {:?} does not exist", id),
                None,
            )));
        }
        candidate.processing.profiles.insert(id, profile);
        self.apply_candidate(candidate, true)
    }

    pub fn rename_processing_profile(
        &mut self,
        id: String,
        name: String,
        expected_revision: u64,
    ) -> Result<ConfigSnapshot, ConfigError> {
        self.check_profile_revision(expected_revision)?;
        let name = name.trim().to_string();
        validate_profile_name(&self.path, &name).map_err(|error| self.remember_error(error))?;
        self.ensure_profile_name_available_except(&name, &id)?;
        let mut candidate = self.current.clone();
        let Some(profile) = candidate.processing.profiles.get_mut(&id) else {
            return Err(self.remember_error(ConfigError::validation(
                &self.path,
                format!("profile {:?} does not exist", id),
                None,
            )));
        };
        profile.name = name;
        self.apply_candidate(candidate, true)
    }

    pub fn delete_processing_profile(
        &mut self,
        id: String,
        expected_revision: u64,
    ) -> Result<ConfigSnapshot, ConfigError> {
        self.check_profile_revision(expected_revision)?;
        if self.current.processing.profiles.len() <= 1 {
            return Err(self.remember_error(ConfigError::validation(
                &self.path,
                "cannot delete the last profile",
                None,
            )));
        }
        let mut candidate = self.current.clone();
        if candidate.processing.profiles.remove(&id).is_none() {
            return Err(self.remember_error(ConfigError::validation(
                &self.path,
                format!("profile {:?} does not exist", id),
                None,
            )));
        }
        candidate
            .capture
            .bindings
            .retain(|binding| binding.profile_id.as_deref() != Some(id.as_str()));
        if candidate.processing.active_profile == id {
            candidate.processing.active_profile = candidate
                .processing
                .profiles
                .keys()
                .next()
                .expect("profile count checked before deletion")
                .clone();
        }
        self.apply_candidate(candidate, true)
    }

    pub fn select_processing_profile(
        &mut self,
        id: String,
        expected_revision: u64,
    ) -> Result<ConfigSnapshot, ConfigError> {
        self.check_profile_revision(expected_revision)?;
        if !self.current.processing.profiles.contains_key(&id) {
            return Err(self.remember_error(ConfigError::validation(
                &self.path,
                format!("profile {:?} does not exist", id),
                None,
            )));
        }
        let mut candidate = self.current.clone();
        candidate.processing.active_profile = id;
        self.apply_candidate(candidate, true)
    }

    pub fn reload_from_disk(&mut self) -> ConfigTransition {
        let loaded = match self.read_disk_config() {
            Ok(config) => config,
            Err(error) => return ConfigTransition::Rejected(self.remember_error(error)),
        };
        let candidate = loaded.config;
        if candidate == self.current {
            if self.last_error.is_some() {
                self.last_error = None;
                self.revision = self.revision.saturating_add(1);
                return ConfigTransition::Changed(self.snapshot());
            }
            return ConfigTransition::Unchanged(self.snapshot());
        }
        match self.apply_candidate(candidate, false) {
            Ok(snapshot) => ConfigTransition::Changed(snapshot),
            Err(error) => ConfigTransition::Rejected(error),
        }
    }

    pub(super) fn remember_error(&mut self, mut error: ConfigError) -> ConfigError {
        self.revision = self.revision.saturating_add(1);
        error.revision = self.revision;
        error.current_bindings = self.capture_binding_snapshots().into_boxed_slice();
        error.current_open_shortcut = self.current.shortcuts.open.clone().into_boxed_str();
        error.registered = self.is_fully_registered();
        self.last_error = Some(error.clone());
        error
    }

    pub(super) fn apply_candidate(
        &mut self,
        candidate: AppConfig,
        write: bool,
    ) -> Result<ConfigSnapshot, ConfigError> {
        let mut candidate = candidate;
        let mut guided = false;
        for profile in candidate.processing.profiles.values_mut() {
            guided |= profile.normalize_models();
        }
        let provisioned = candidate.provision_profiles();
        candidate
            .validate(&self.path)
            .map_err(|error| self.remember_error(error))?;
        let old_config = self.current.clone();
        let old_active = self.active_shortcuts.clone();
        if !self.registration_suspended {
            let desired = self.desired_shortcuts(&candidate)?;
            if let Err(error) = self.sync_runtime(&desired) {
                return Err(self.remember_error(error));
            }
        }
        if write || provisioned || guided {
            if let Err(error) = write_config(&self.path, &candidate) {
                if !self.registration_suspended {
                    self.sync_runtime(&old_active).ok();
                }
                self.current = old_config;
                return Err(self.remember_error(error));
            }
        }
        self.current = candidate;
        if self.registration_suspended {
            self.active_shortcuts.clear();
        }
        self.clear_runtime_error();
        self.revision = self.revision.saturating_add(1);
        Ok(self.snapshot())
    }

    pub(super) fn check_profile_revision(
        &mut self,
        expected_revision: u64,
    ) -> Result<(), ConfigError> {
        if expected_revision != self.revision {
            return Err(self.remember_error(ConfigError::conflict(
                &self.path,
                format!(
                    "stale config revision {expected_revision}; current revision is {}",
                    self.revision
                ),
                None,
            )));
        }
        let disk = self
            .read_disk_config()
            .map_err(|error| self.remember_error(error))?;
        if disk.config != self.current {
            return Err(self.remember_error(ConfigError::conflict(
                &self.path,
                "config changed externally; wait for the settings to refresh before saving",
                None,
            )));
        }
        Ok(())
    }

    fn ensure_profile_name_available(&mut self, name: &str) -> Result<(), ConfigError> {
        self.ensure_profile_name_available_except(name, "")
    }

    fn ensure_profile_name_available_except(
        &mut self,
        name: &str,
        except_id: &str,
    ) -> Result<(), ConfigError> {
        let normalized = name.to_lowercase();
        if self
            .current
            .processing
            .profiles
            .iter()
            .any(|(id, profile)| {
                id != except_id && profile.name.trim().to_lowercase() == normalized
            })
        {
            return Err(self.remember_error(ConfigError::validation(
                &self.path,
                format!("duplicate profile name {:?}", name),
                None,
            )));
        }
        Ok(())
    }

    fn read_disk_config(&self) -> Result<LoadedConfig, ConfigError> {
        let raw = fs::read_to_string(&self.path).map_err(|error| {
            ConfigError::io(
                &self.path,
                format!("could not read config {}: {error}", self.path.display()),
            )
        })?;
        parse_config(&self.path, &raw)
    }

    fn desired_shortcuts(&self, config: &AppConfig) -> Result<BTreeMap<u32, String>, ConfigError> {
        let mut desired = BTreeMap::new();
        for binding in &config.capture.bindings {
            let id = native_shortcut_id(&binding.shortcut).map_err(|error| {
                ConfigError::validation(
                    &self.path,
                    format!("invalid capture shortcut {:?}: {error}", binding.shortcut),
                    Some(binding.shortcut.clone()),
                )
            })?;
            desired.insert(id, binding.shortcut.clone());
        }
        let open_id = native_shortcut_id(&config.shortcuts.open).map_err(|error| {
            ConfigError::validation(
                &self.path,
                format!("invalid open shortcut {:?}: {error}", config.shortcuts.open),
                Some(config.shortcuts.open.clone()),
            )
        })?;
        desired.insert(open_id, config.shortcuts.open.clone());
        Ok(desired)
    }

    fn sync_runtime(&mut self, desired: &BTreeMap<u32, String>) -> Result<(), ConfigError> {
        let before = self.active_shortcuts.clone();
        let mut working = before.clone();
        let mut added = Vec::new();
        let mut removed = Vec::new();
        for (native_id, shortcut) in desired {
            if !working.contains_key(native_id) {
                if let Err(error) = self.runtime.register(shortcut) {
                    self.rollback_runtime(&mut working, &added, &removed);
                    self.active_shortcuts = working;
                    return Err(ConfigError::conflict(
                        &self.path,
                        format!("could not register shortcut {:?}: {error}", shortcut),
                        Some(shortcut.clone()),
                    ));
                }
                working.insert(*native_id, shortcut.clone());
                added.push((*native_id, shortcut.clone()));
            } else {
                working.insert(*native_id, shortcut.clone());
            }
        }
        for (native_id, shortcut) in before {
            if !desired.contains_key(&native_id) {
                if let Err(error) = self.runtime.unregister(&shortcut) {
                    self.rollback_runtime(&mut working, &added, &removed);
                    self.active_shortcuts = working;
                    return Err(ConfigError::runtime(
                        &self.path,
                        format!("could not replace shortcut {:?}: {error}", shortcut),
                        Some(shortcut),
                    ));
                }
                working.remove(&native_id);
                removed.push((native_id, shortcut));
            }
        }
        self.active_shortcuts = working;
        Ok(())
    }

    fn rollback_runtime(
        &mut self,
        working: &mut BTreeMap<u32, String>,
        added: &[(u32, String)],
        removed: &[(u32, String)],
    ) {
        for (native_id, shortcut) in added.iter().rev() {
            if self.runtime.unregister(shortcut).is_ok() {
                working.remove(native_id);
            }
        }
        for (native_id, shortcut) in removed.iter().rev() {
            if self.runtime.register(shortcut).is_ok() {
                working.insert(*native_id, shortcut.clone());
            }
        }
    }

    fn is_fully_registered(&self) -> bool {
        if self.registration_suspended {
            return false;
        }
        self.desired_shortcuts(&self.current)
            .map(|desired| desired.keys().eq(self.active_shortcuts.keys()))
            .unwrap_or(false)
    }

    fn clear_runtime_error(&mut self) {
        if matches!(
            self.last_error.as_ref().map(|error| &error.kind),
            Some(ConfigErrorKind::Conflict | ConfigErrorKind::Runtime)
        ) {
            self.last_error = None;
        }
    }

    fn capture_binding_snapshots(&self) -> Vec<CaptureBindingSnapshot> {
        self.current
            .capture
            .bindings
            .iter()
            .map(|binding| CaptureBindingSnapshot {
                id: binding.id.clone(),
                shortcut: binding.shortcut.clone(),
                profile_id: binding.profile_id.clone(),
            })
            .collect()
    }
}

fn next_binding_id(config: &AppConfig) -> String {
    let mut index = 1_u64;
    loop {
        let id = format!("binding-{index}");
        if !config
            .capture
            .bindings
            .iter()
            .any(|binding| binding.id == id)
        {
            return id;
        }
        index += 1;
    }
}

#[cfg(test)]
#[path = "controller_tests.rs"]
mod tests;
