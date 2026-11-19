//! Capture shortcut binding validation and native accelerator identity.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::str::FromStr;
use tauri_plugin_global_shortcut::Shortcut;

use super::types::{AppConfig, ConfigError};

pub const DEFAULT_BINDING_ID: &str = "default";

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ShortcutBinding {
    pub(crate) id: String,
    pub(crate) shortcut: String,
    pub(crate) profile_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CaptureInvocation {
    pub(crate) binding_id: String,
    pub(crate) profile_id: String,
}

pub(crate) fn default_binding() -> ShortcutBinding {
    ShortcutBinding {
        id: DEFAULT_BINDING_ID.to_string(),
        shortcut: super::types::DEFAULT_CAPTURE_SHORTCUT.to_string(),
        profile_id: None,
    }
}

pub(crate) fn native_shortcut_id(shortcut: &str) -> Result<u32, String> {
    Shortcut::from_str(shortcut)
        .map(|shortcut| shortcut.id())
        .map_err(|error| error.to_string())
}

pub(crate) fn validate_bindings(
    config: &AppConfig,
    path: &std::path::Path,
) -> Result<(), ConfigError> {
    let bindings = &config.capture.bindings;
    if bindings.is_empty() {
        return Err(ConfigError::validation(
            path,
            "capture.bindings must contain the default binding",
            None,
        ));
    }

    let mut ids = HashSet::new();
    let mut native_ids = HashSet::new();
    let mut default_count = 0;
    for binding in bindings {
        if binding.id.trim().is_empty() {
            return Err(ConfigError::validation(
                path,
                "capture binding IDs must not be empty",
                Some(binding.shortcut.clone()),
            ));
        }
        if !ids.insert(binding.id.clone()) {
            return Err(ConfigError::validation(
                path,
                format!("duplicate capture binding ID {:?}", binding.id),
                Some(binding.shortcut.clone()),
            ));
        }

        let native_id = native_shortcut_id(&binding.shortcut).map_err(|error| {
            ConfigError::validation(
                path,
                format!("invalid capture shortcut {:?}: {error}", binding.shortcut),
                Some(binding.shortcut.clone()),
            )
        })?;
        if !native_ids.insert(native_id) {
            return Err(ConfigError::validation(
                path,
                format!("duplicate capture shortcut {:?}", binding.shortcut),
                Some(binding.shortcut.clone()),
            ));
        }

        if binding.id == DEFAULT_BINDING_ID {
            default_count += 1;
            if binding.profile_id.is_some() {
                return Err(ConfigError::validation(
                    path,
                    "the default capture binding must use the active profile",
                    Some(binding.shortcut.clone()),
                ));
            }
        } else if binding.profile_id.is_none() {
            return Err(ConfigError::validation(
                path,
                format!("capture binding {:?} must target a profile", binding.id),
                Some(binding.shortcut.clone()),
            ));
        } else if !config
            .processing
            .profiles
            .contains_key(binding.profile_id.as_ref().expect("checked above"))
        {
            return Err(ConfigError::validation(
                path,
                format!(
                    "capture binding {:?} targets missing profile {:?}",
                    binding.id,
                    binding.profile_id.as_deref().unwrap_or_default()
                ),
                Some(binding.shortcut.clone()),
            ));
        }
    }

    if default_count != 1 {
        return Err(ConfigError::validation(
            path,
            "capture.bindings must contain exactly one default binding",
            None,
        ));
    }
    Ok(())
}
