//! Starting model choices used to provision editable profiles once per provider.

use crate::inference_profiles::InferenceBackend;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum BuiltinModelTier {
    Efficient,
    Accurate,
}

#[derive(Clone, Deserialize, Serialize, specta::Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BuiltinProfileDefinition {
    pub id: String,
    pub name: String,
    pub backend: InferenceBackend,
    pub model: String,
    pub tier: BuiltinModelTier,
    pub explanation: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BuiltinProfileMetadata {
    pub id: String,
    pub name: String,
    pub model: String,
    pub tier: BuiltinModelTier,
    pub explanation: String,
}

pub fn profiles() -> &'static [BuiltinProfileDefinition] {
    &crate::application_config::MODEL_DEFAULTS.builtin_profiles
}

pub fn metadata_for(backend: InferenceBackend) -> Vec<BuiltinProfileMetadata> {
    profiles()
        .iter()
        .filter(|profile| profile.backend == backend)
        .map(|profile| BuiltinProfileMetadata {
            id: profile.id.to_owned(),
            name: profile.name.to_owned(),
            model: profile.model.to_owned(),
            tier: profile.tier,
            explanation: profile.explanation.to_owned(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn builtins_cover_supported_providers_with_unique_stable_ids() {
        let ids: HashSet<_> = profiles()
            .iter()
            .map(|profile| profile.id.as_str())
            .collect();
        assert_eq!(ids.len(), profiles().len());
        for backend in InferenceBackend::ALL {
            let profiles = metadata_for(backend);
            if matches!(
                backend,
                InferenceBackend::OpenaiCompatible | InferenceBackend::Local
            ) {
                assert!(profiles.is_empty());
                continue;
            }
            assert_eq!(profiles.len(), 2);
            assert_eq!(profiles[0].tier, BuiltinModelTier::Efficient);
            assert_eq!(profiles[1].tier, BuiltinModelTier::Accurate);
            assert_ne!(profiles[0].model, profiles[1].model);
            assert!(profiles
                .iter()
                .all(|profile| super::profiles().iter().any(|seed| seed.id == profile.id)));
        }
    }
}
