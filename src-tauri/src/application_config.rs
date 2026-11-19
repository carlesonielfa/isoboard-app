//! Build-time application policy, integration settings, and default model choices.
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::LazyLock;

#[derive(Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ApplicationConfig {
    pub limits: ValidationLimits,
    pub history: HistoryPolicy,
    pub feedback: FeedbackPolicy,
}

#[derive(Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ValidationLimits {
    pub additional_rules_bytes: usize,
    pub prompt_template_bytes: usize,
    pub profile_name_characters: usize,
    pub temperature_min: f64,
    pub temperature_max: f64,
    pub max_output_tokens: u32,
    pub request_timeout_min_seconds: u32,
    pub request_timeout_max_seconds: u32,
}

#[derive(Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HistoryPolicy {
    pub max_entries: usize,
    pub max_bytes: u64,
    pub max_originals: usize,
    pub max_original_bytes: u64,
}

#[derive(Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct FeedbackPolicy {
    pub copy_milliseconds: u64,
    pub resource_poll_milliseconds: u64,
    pub capture_success_milliseconds: u64,
    pub capture_failure_milliseconds: u64,
    pub capture_fade_milliseconds: u64,
    pub capture_cursor_poll_milliseconds: u64,
    pub capture_exit_grace_milliseconds: u64,
    pub tray_frame_milliseconds: Vec<u64>,
}

#[derive(Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ServiceConfig {
    pub models_dev: ModelsDevService,
    pub hugging_face: HuggingFaceService,
    pub updates: UpdatesService,
}

#[derive(Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ModelsDevService {
    pub url: String,
    pub request_timeout_seconds: u64,
}

#[derive(Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HuggingFaceService {
    pub base_url: String,
    pub connect_timeout_seconds: u64,
    pub read_timeout_seconds: u64,
}

#[derive(Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct UpdatesService {
    pub manifest_url: String,
    pub request_timeout_seconds: u64,
    pub initial_delay_seconds: u64,
    pub interval_seconds: u64,
}

#[derive(Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ModelDefaults {
    pub setup_local_model: String,
    pub provider_labels:
        std::collections::HashMap<crate::inference_profiles::InferenceBackend, String>,
    pub builtin_profiles: Vec<crate::builtin_inference_profiles::BuiltinProfileDefinition>,
}

pub(crate) static APPLICATION: LazyLock<ApplicationConfig> = LazyLock::new(|| {
    crate::config_jsonc::parse(include_str!("../../config/application.jsonc"))
        .expect("invalid application.jsonc")
});

pub(crate) static SERVICES: LazyLock<ServiceConfig> = LazyLock::new(|| {
    crate::config_jsonc::parse(include_str!("../../config/services.jsonc"))
        .expect("invalid services.jsonc")
});

pub(crate) static MODEL_DEFAULTS: LazyLock<ModelDefaults> = LazyLock::new(|| {
    crate::config_jsonc::parse(include_str!("../../config/model-defaults.jsonc"))
        .expect("invalid model-defaults.jsonc")
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configuration_preserves_valid_application_and_integration_contracts() {
        let limits = &APPLICATION.limits;
        assert!(limits.additional_rules_bytes > 0 && limits.prompt_template_bytes > 0);
        assert!(limits.profile_name_characters > 0);
        assert!(limits.temperature_min.is_finite() && limits.temperature_max.is_finite());
        assert!(limits.temperature_min <= limits.temperature_max);
        assert!(limits.max_output_tokens > 0);
        assert!(limits.request_timeout_min_seconds > 0);
        assert!(limits.request_timeout_min_seconds <= limits.request_timeout_max_seconds);
        assert!(APPLICATION.history.max_entries > 0 && APPLICATION.history.max_bytes > 0);
        let feedback = &APPLICATION.feedback;
        assert_eq!(feedback.tray_frame_milliseconds.len(), 4);
        assert!(feedback.copy_milliseconds > 0 && feedback.resource_poll_milliseconds > 0);
        assert!(
            feedback.capture_success_milliseconds > 0 && feedback.capture_failure_milliseconds > 0
        );
        assert!(feedback.capture_cursor_poll_milliseconds > 0);
        assert!(feedback.capture_exit_grace_milliseconds >= feedback.capture_fade_milliseconds);
        assert!(feedback
            .tray_frame_milliseconds
            .iter()
            .all(|value| *value > 0));
        for endpoint in [
            &SERVICES.models_dev.url,
            &SERVICES.hugging_face.base_url,
            &SERVICES.updates.manifest_url,
        ] {
            let url = url::Url::parse(endpoint).unwrap();
            assert_eq!(url.scheme(), "https");
            assert!(url.host_str().is_some());
        }
        assert!(!SERVICES.hugging_face.base_url.ends_with('/'));
        for timeout in [
            SERVICES.models_dev.request_timeout_seconds,
            SERVICES.hugging_face.connect_timeout_seconds,
            SERVICES.hugging_face.read_timeout_seconds,
            SERVICES.updates.request_timeout_seconds,
            SERVICES.updates.interval_seconds,
        ] {
            assert!(timeout > 0);
        }
        let catalog: serde_json::Value =
            serde_json::from_str(include_str!("local_models/catalog.json")).unwrap();
        assert!(catalog["models"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["id"].as_str() == Some(&MODEL_DEFAULTS.setup_local_model)));
        for backend in crate::inference_profiles::InferenceBackend::ALL {
            assert!(MODEL_DEFAULTS
                .provider_labels
                .get(&backend)
                .is_some_and(|label| !label.is_empty()));
        }
    }
}
