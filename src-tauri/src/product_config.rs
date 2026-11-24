//! Public build-time product settings shared with the frontend. Never put secrets here.
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::LazyLock;

#[derive(Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProductConfig {
    pub support_email: String,
    pub checkout_url: String,
    pub docs_url: String,
    pub licensing: LicensingConfig,
    pub polar: PolarConfig,
    pub legal_documents: Vec<LegalDocument>,
    pub third_party_documents: Vec<LegalDocument>,
}

#[derive(Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LicensingConfig {
    pub trial_days: u32,
    pub support_months: u32,
    pub covered_major: u32,
    pub key_prefix: String,
    pub validation_interval_seconds: u64,
    pub background_poll_seconds: u64,
    pub status_refresh_milliseconds: u32,
}

#[derive(Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PolarConfig {
    pub organization_id: String,
    pub benefit_id: String,
    pub product_id: String,
    pub validation_url: String,
    pub storefront_url: String,
    pub request_timeout_seconds: u64,
    pub connect_timeout_seconds: u64,
}

pub(crate) static PRODUCT: LazyLock<ProductConfig> = LazyLock::new(|| {
    crate::config_jsonc::parse(include_str!("../../config/product.jsonc"))
        .expect("product.jsonc must match the Rust product configuration schema")
});

pub(crate) fn evaluation_seconds() -> u64 {
    u64::from(PRODUCT.licensing.trial_days) * 24 * 60 * 60
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_product_settings_are_valid() {
        assert!(PRODUCT.licensing.trial_days > 0);
        assert!(PRODUCT.licensing.support_months > 0);
        assert!(PRODUCT.licensing.covered_major > 0);
        assert!(PRODUCT.licensing.validation_interval_seconds > 0);
        assert!(PRODUCT.licensing.background_poll_seconds > 0);
        assert!(PRODUCT.licensing.status_refresh_milliseconds >= 1000);
        assert!(!PRODUCT.licensing.key_prefix.is_empty());
        assert!(PRODUCT.support_email.contains('@'));
        for value in [
            &PRODUCT.checkout_url,
            &PRODUCT.docs_url,
            &PRODUCT.polar.validation_url,
            &PRODUCT.polar.storefront_url,
        ] {
            assert_eq!(reqwest::Url::parse(value).unwrap().scheme(), "https");
        }
        for value in [
            &PRODUCT.polar.organization_id,
            &PRODUCT.polar.benefit_id,
            &PRODUCT.polar.product_id,
        ] {
            uuid::Uuid::parse_str(value).unwrap();
        }
        assert!(PRODUCT.polar.connect_timeout_seconds > 0);
        assert!(PRODUCT.polar.request_timeout_seconds >= PRODUCT.polar.connect_timeout_seconds);
    }
}

#[derive(Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LegalDocument {
    pub label: String,
    pub resource: String,
}
