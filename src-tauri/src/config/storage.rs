//! Version-one TOML parsing and atomic persistence.

use super::types::*;
use std::fs;
use std::path::Path;

#[derive(Clone, Debug)]
pub(super) struct LoadedConfig {
    pub(super) config: AppConfig,
}

pub(super) fn parse_config(path: &Path, raw: &str) -> Result<LoadedConfig, ConfigError> {
    let value: toml::Value = toml::from_str(raw).map_err(|error| {
        ConfigError::parse(
            path,
            format!("could not parse config {}: {error}", path.display()),
        )
    })?;
    let version = match value.get("version") {
        None => i64::from(CONFIG_VERSION),
        Some(value) => value
            .as_integer()
            .ok_or_else(|| ConfigError::parse(path, "config version must be an integer"))?,
    };
    let version = u32::try_from(version)
        .map_err(|_| ConfigError::parse(path, "config version must be a positive integer"))?;
    if version != CONFIG_VERSION {
        return Err(ConfigError::validation(
            path,
            format!("unsupported config version {version}; expected {CONFIG_VERSION}"),
            None,
        ));
    }

    let mut config: AppConfig = toml::from_str(raw).map_err(|error| {
        ConfigError::parse(
            path,
            format!("could not parse config {}: {error}", path.display()),
        )
    })?;
    for profile in config.processing.profiles.values_mut() {
        for settings in [
            &mut profile.model_settings,
            &mut profile.rules_model_settings,
        ]
        .into_iter()
        .flatten()
        {
            settings.normalize_options();
        }
        profile.conversion_prompt_template =
            crate::inference::params::normalize_prompt_template_override(
                profile.conversion_prompt_template.take(),
                crate::inference::params::default_conversion_prompt_template(),
            )
            .map_err(|error| {
                ConfigError::validation(path, format!("conversion prompt template: {error}"), None)
            })?;
        profile.postprocessing_prompt_template =
            crate::inference::params::normalize_prompt_template_override(
                profile.postprocessing_prompt_template.take(),
                crate::inference::params::default_postprocessing_prompt_template(),
            )
            .map_err(|error| {
                ConfigError::validation(
                    path,
                    format!("postprocessing prompt template: {error}"),
                    None,
                )
            })?;
    }
    config.validate(path)?;
    Ok(LoadedConfig { config })
}

pub(super) fn write_config(path: &Path, config: &AppConfig) -> Result<(), ConfigError> {
    let parent = path.parent().ok_or_else(|| {
        ConfigError::io(
            path,
            format!("config path {} has no parent directory", path.display()),
        )
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        ConfigError::io(
            path,
            format!(
                "could not create config directory {}: {error}",
                parent.display()
            ),
        )
    })?;

    let content = toml::to_string_pretty(config)
        .map_err(|error| ConfigError::io(path, format!("could not serialize config: {error}")))?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("config.toml");
    let temp_path = parent.join(format!(".{file_name}.{}.tmp", std::process::id()));

    fs::write(&temp_path, content).map_err(|error| {
        ConfigError::io(
            path,
            format!(
                "could not write temporary config {}: {error}",
                temp_path.display()
            ),
        )
    })?;
    if let Err(error) = fs::rename(&temp_path, path) {
        let _ = fs::remove_file(&temp_path);
        return Err(ConfigError::io(
            path,
            format!("could not replace config {}: {error}", path.display()),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types::OverlayPosition;
    use crate::inference_profiles::ReasoningLevel;
    use std::path::Path;

    #[test]
    fn overlay_settings_default_and_reject_unknown_presets() {
        let path = Path::new("config.toml");
        let default = parse_config(path, "version = 1\n").unwrap();
        assert!(default.config.capture.overlay_enabled);
        assert_eq!(
            default.config.capture.overlay_position,
            OverlayPosition::BottomCenter
        );

        let top_left = parse_config(
            path,
            "version = 1\n[capture]\noverlay_enabled = false\noverlay_position = 'top_left'\n",
        )
        .unwrap();
        assert!(!top_left.config.capture.overlay_enabled);
        assert_eq!(
            top_left.config.capture.overlay_position,
            OverlayPosition::TopLeft
        );

        assert!(parse_config(
            path,
            "version = 1\n[capture]\noverlay_position = 'elsewhere'\n"
        )
        .is_err());
    }

    #[test]
    fn parse_config_normalizes_empty_inference_options() {
        let loaded = parse_config(
            Path::new("config.toml"),
            r#"
version = 1

[processing.profiles.default]
name = "OCR"
[processing.profiles.default.model_settings]
provider = "openai_api"
model = "vision-model"

[processing.profiles.default.model_settings.options]
"#,
        )
        .unwrap();
        assert!(loaded.config.processing.profiles["default"]
            .model_settings
            .as_ref()
            .unwrap()
            .options
            .is_none());
    }

    #[test]
    fn parse_config_keeps_typed_inference_options() {
        let loaded = parse_config(
            Path::new("config.toml"),
            r#"
version = 1

[processing.profiles.default]
name = "OCR"
[processing.profiles.default.model_settings]
provider = "openai_api"
model = "vision-model"

[processing.profiles.default.model_settings.options]
temperature = 0.5
max_output_tokens = 256
reasoning = "high"
request_timeout_seconds = 90
"#,
        )
        .unwrap();
        let options = loaded.config.processing.profiles["default"]
            .model_settings
            .as_ref()
            .unwrap()
            .options
            .as_ref()
            .unwrap();
        assert_eq!(options.temperature, Some(0.5));
        assert_eq!(options.max_output_tokens, Some(256));
        assert_eq!(options.reasoning, Some(ReasoningLevel::High));
        assert_eq!(options.request_timeout_seconds, Some(90));
    }

    #[test]
    fn parse_config_rejects_unknown_inference_option() {
        let error = parse_config(
            Path::new("config.toml"),
            r#"
version = 1

[processing.profiles.default]
name = "OCR"
[processing.profiles.default.model_settings]
provider = "openai_api"
model = "vision-model"

[processing.profiles.default.model_settings.options]
sampling = 0.5
"#,
        )
        .unwrap_err();
        assert_eq!(error.kind, ConfigErrorKind::Parse);
    }
}
