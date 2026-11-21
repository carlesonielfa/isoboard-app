use crate::application_config::APPLICATION;
use crate::inference_profiles::{ModelSettings, ModelSettingsDraft, ModelSettingsSnapshot};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;

pub const DEFAULT_PROFILE_ID: &str = "default";
pub const DEFAULT_PROFILE_NAME: &str = "Default";

static ADDITIONAL_RULES_LIMIT_ERROR: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    format!(
        "additional rules exceed {} bytes",
        APPLICATION.limits.additional_rules_bytes
    )
});
static PROFILE_NAME_LIMIT_ERROR: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    format!(
        "profile name must be at most {} characters",
        APPLICATION.limits.profile_name_characters
    )
});

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
pub struct ProcessingConfig {
    pub active_profile: String,
    pub profiles: BTreeMap<String, ProcessingProfile>,
}

impl<'de> Deserialize<'de> for ProcessingConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            #[serde(default)]
            active_profile: Option<String>,
            #[serde(default)]
            profiles: Option<BTreeMap<String, ProcessingProfile>>,
        }

        let fields = Fields::deserialize(deserializer)?;
        let (active_profile, profiles) = match (fields.active_profile, fields.profiles) {
            (None, None) => return Ok(default_processing_config()),
            (None, Some(profiles)) if !profiles.is_empty() => (
                profiles
                    .keys()
                    .next()
                    .expect("non-empty profiles have a first key")
                    .clone(),
                profiles,
            ),
            (None, Some(profiles)) => (DEFAULT_PROFILE_ID.to_string(), profiles),
            (Some(active_profile), None) => (active_profile, default_profiles()),
            (Some(active_profile), Some(profiles)) => (active_profile, profiles),
        };
        Ok(Self {
            active_profile,
            profiles,
        })
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, specta::Type)]
#[serde(deny_unknown_fields)]
pub struct ProcessingProfile {
    pub name: String,
    /// Optional inference selection.  A profile without one remains valid
    /// while the inference configuration is being set up.
    #[serde(default)]
    pub model_settings: Option<ModelSettings>,
    #[serde(default)]
    pub rules_model_settings: Option<ModelSettings>,
    #[serde(default)]
    pub rules_model_mode: RulesModelMode,
    #[serde(default)]
    pub conversion: ConversionSettings,
    #[serde(
        default = "default_additional_rules",
        deserialize_with = "deserialize_additional_rules"
    )]
    #[specta(type = String)]
    pub additional_rules: String,
    #[serde(default)]
    pub additional_rules_separate_pass: bool,
    #[serde(default)]
    pub application_mode_override: Option<ConversionSettingsApplicationMode>,
    /// Optional complete replacement for the first-stage bundled prompt.
    /// The bundled source is represented by `None`.
    #[serde(default)]
    pub conversion_prompt_template: Option<String>,
    /// Optional complete replacement for the second-stage bundled prompt.
    /// The bundled source is represented by `None`.
    #[serde(default)]
    pub postprocessing_prompt_template: Option<String>,
}

/// Complete editable processing-profile payload used by the transactional IPC
/// create and update commands.
#[derive(Clone, Debug, Deserialize, PartialEq, specta::Type)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingProfileDraft {
    pub name: String,
    #[serde(default)]
    pub model_settings: Option<ModelSettingsDraft>,
    #[serde(default)]
    pub rules_model_settings: Option<ModelSettingsDraft>,
    #[serde(default)]
    pub rules_model_mode: RulesModelMode,
    #[serde(default)]
    pub conversion: ConversionSettings,
    #[serde(
        default = "default_additional_rules",
        deserialize_with = "deserialize_additional_rules"
    )]
    #[specta(type = String)]
    pub additional_rules: String,
    #[serde(default)]
    pub additional_rules_separate_pass: bool,
    #[serde(default)]
    pub application_mode_override: Option<ConversionSettingsApplicationMode>,
    #[serde(default)]
    pub conversion_prompt_template: Option<String>,
    #[serde(default)]
    pub postprocessing_prompt_template: Option<String>,
}

impl ProcessingProfileDraft {
    pub fn into_profile(self) -> Result<ProcessingProfile, String> {
        let conversion_prompt_template =
            crate::inference::params::normalize_prompt_template_override(
                self.conversion_prompt_template,
                crate::inference::params::default_conversion_prompt_template(),
            )
            .map_err(|error| format!("conversion prompt template: {error}"))?;
        let postprocessing_prompt_template =
            crate::inference::params::normalize_prompt_template_override(
                self.postprocessing_prompt_template,
                crate::inference::params::default_postprocessing_prompt_template(),
            )
            .map_err(|error| format!("postprocessing prompt template: {error}"))?;
        // The independent rules stage receives text without an image, so an
        // extraction-only local model cannot run it.
        if self.rules_model_mode == RulesModelMode::Independent
            && self.rules_model_settings.as_ref().is_some_and(|settings| {
                settings.provider == crate::inference_profiles::InferenceBackend::Local
                    && crate::local_models::definition(settings.model.trim())
                        .is_some_and(|model| !model.supports_text)
            })
        {
            return Err("rules model must support text processing".into());
        }
        Ok(ProcessingProfile {
            name: self.name,
            model_settings: self
                .model_settings
                .map(ModelSettingsDraft::into_settings)
                .transpose()?,
            rules_model_settings: self
                .rules_model_settings
                .map(ModelSettingsDraft::into_settings)
                .transpose()?,
            rules_model_mode: self.rules_model_mode,
            conversion: self.conversion,
            additional_rules: normalize_additional_rules(&self.additional_rules)?,
            additional_rules_separate_pass: self.additional_rules_separate_pass,
            application_mode_override: self.application_mode_override,
            conversion_prompt_template,
            postprocessing_prompt_template,
        })
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ConversionSettingsApplicationMode {
    Prompt,
    HtmlCleanup,
    PromptAndHtmlCleanup,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, specta::Type)]
#[serde(deny_unknown_fields)]
#[serde(default)]
pub struct ConversionSettings {
    pub tables: bool,
    pub colors: bool,
    pub images: bool,
}

/// Serialized processing settings always contain all three feature flags.
/// The persisted [`ConversionSettings`] keeps serde defaults for migration,
/// while this boundary type describes the complete IPC snapshot shape.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingConversionSettings {
    pub tables: bool,
    pub colors: bool,
    pub images: bool,
}

impl Default for ConversionSettings {
    fn default() -> Self {
        Self {
            tables: true,
            colors: true,
            images: true,
        }
    }
}

fn default_additional_rules() -> String {
    String::new()
}

fn deserialize_additional_rules<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    normalize_additional_rules(&value).map_err(serde::de::Error::custom)
}

pub fn normalize_additional_rules(value: &str) -> Result<String, &'static str> {
    if value.len() > APPLICATION.limits.additional_rules_bytes {
        return Err(&ADDITIONAL_RULES_LIMIT_ERROR);
    }
    if value
        .chars()
        .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return Err("additional rules contain unsupported control characters");
    }
    if value.trim().is_empty() {
        return Ok(String::new());
    }
    Ok(value.to_owned())
}

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingSnapshot {
    pub active_profile_id: String,
    pub profiles: Vec<ProcessingProfileSnapshot>,
    pub prompt_template_defaults: PromptTemplateDefaults,
}

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PromptTemplateDefaults {
    pub conversion: String,
    pub postprocessing: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingProfileSnapshot {
    pub id: String,
    pub name: String,
    pub model_settings: Option<ModelSettingsSnapshot>,
    pub rules_model_settings: Option<ModelSettingsSnapshot>,
    pub rules_model_mode: RulesModelMode,
    #[specta(type = ProcessingConversionSettings)]
    pub conversion: ConversionSettings,
    pub additional_rules: String,
    pub additional_rules_separate_pass: bool,
    pub application_mode_override: Option<ConversionSettingsApplicationMode>,
    pub conversion_prompt_template: Option<String>,
    pub postprocessing_prompt_template: Option<String>,
}

pub fn default_processing_config() -> ProcessingConfig {
    ProcessingConfig {
        active_profile: DEFAULT_PROFILE_ID.to_string(),
        profiles: default_profiles(),
    }
}

fn default_profiles() -> BTreeMap<String, ProcessingProfile> {
    BTreeMap::from([(
        DEFAULT_PROFILE_ID.to_string(),
        ProcessingProfile {
            name: DEFAULT_PROFILE_NAME.to_string(),
            model_settings: None,
            rules_model_settings: None,
            rules_model_mode: RulesModelMode::Same,
            conversion: ConversionSettings::default(),
            additional_rules: String::new(),
            additional_rules_separate_pass: false,
            application_mode_override: None,
            conversion_prompt_template: None,
            postprocessing_prompt_template: None,
        },
    )])
}

impl Default for ProcessingConfig {
    fn default() -> Self {
        default_processing_config()
    }
}

pub fn validate_profile_name(name: &str) -> Result<String, &'static str> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("profile name must not be empty");
    }
    if trimmed.chars().count() > APPLICATION.limits.profile_name_characters {
        return Err(&PROFILE_NAME_LIMIT_ERROR);
    }
    Ok(trimmed.to_string())
}

pub fn validate_profile_id(id: &str) -> Result<(), &'static str> {
    if id.trim().is_empty() {
        Err("profile IDs must not be empty")
    } else {
        Ok(())
    }
}

pub fn next_profile_id(config: &ProcessingConfig) -> String {
    let mut index = 1_u64;
    loop {
        let id = format!("profile-{index}");
        if !config.profiles.contains_key(&id) {
            return id;
        }
        index += 1;
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum RulesModelMode {
    #[default]
    Same,
    Independent,
}

impl ProcessingProfile {
    pub fn normalize_models(&mut self) -> bool {
        let previous = (self.additional_rules_separate_pass, self.rules_model_mode);
        for settings in [&mut self.model_settings, &mut self.rules_model_settings]
            .into_iter()
            .flatten()
        {
            settings.normalize_options();
        }
        if !self.additional_rules.trim().is_empty()
            && self.model_settings.as_ref().is_some_and(|settings| {
                settings.provider == crate::inference_profiles::InferenceBackend::Local
                    && crate::local_models::definition(&settings.model)
                        .is_some_and(|model| !model.supports_text)
            })
        {
            self.additional_rules_separate_pass = true;
            self.rules_model_mode = RulesModelMode::Independent;
        }
        previous != (self.additional_rules_separate_pass, self.rules_model_mode)
    }
}
