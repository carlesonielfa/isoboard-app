//! Public-client validation only: no administrative token or device activation.
use serde::{Deserialize, Serialize};
use std::{future::Future, pin::Pin, time::Duration};

use crate::product_config::PRODUCT;

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Validation {
    Granted {
        license_id: String,
        support_expires_at: Option<u64>,
    },
    Rejected(&'static str),
    Unavailable(&'static str),
}

pub(super) type ValidationFuture<'a> = Pin<Box<dyn Future<Output = Validation> + Send + 'a>>;
pub(super) trait LicenseProvider: Send + Sync {
    fn validate(&self, key: String) -> ValidationFuture<'_>;
}

pub(super) struct Polar {
    client: Option<reqwest::Client>,
}

impl Polar {
    pub(super) fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent(concat!("isoboard/", env!("CARGO_PKG_VERSION")))
                .timeout(Duration::from_secs(PRODUCT.polar.request_timeout_seconds))
                .connect_timeout(Duration::from_secs(PRODUCT.polar.connect_timeout_seconds))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .ok(),
        }
    }
}

#[derive(Serialize)]
struct Request<'a> {
    key: &'a str,
    organization_id: &'static str,
    benefit_id: &'static str,
}

#[derive(Deserialize)]
struct Response {
    id: String,
    organization_id: String,
    benefit_id: String,
    status: String,
    expires_at: Option<String>,
    limit_activations: Option<u64>,
    limit_usage: Option<u64>,
    #[serde(default)]
    created_at: serde_json::Value,
}

// Support metadata never affects the perpetual commercial entitlement.
fn support_expiry(created_at: &serde_json::Value) -> Option<u64> {
    let issued = chrono::DateTime::parse_from_rfc3339(created_at.as_str()?).ok()?;
    let expires =
        issued.checked_add_months(chrono::Months::new(PRODUCT.licensing.support_months))?;
    u64::try_from(expires.timestamp()).ok()
}

const UNAVAILABLE: &str = "Could not verify the license with Polar. Try again when connected.";

fn classify(status: u16, bytes: &[u8]) -> Validation {
    if status == 200 {
        let Ok(response) = serde_json::from_slice::<Response>(bytes) else {
            return Validation::Unavailable(
                "Polar returned an unexpected response. Try again later.",
            );
        };
        if response.organization_id != PRODUCT.polar.organization_id.as_str()
            || response.benefit_id != PRODUCT.polar.benefit_id.as_str()
        {
            return Validation::Rejected("This key does not cover this edition of isoboard.");
        }
        match response.status.as_str() {
            "revoked" | "disabled" => return Validation::Rejected("This license is no longer active. Check your Polar purchase or enter a replacement key."),
            "granted" => {},
            _ => return Validation::Unavailable("Polar returned an unknown license status. Try again later."),
        }
        if uuid::Uuid::parse_str(&response.id).is_err() {
            return Validation::Unavailable(
                "Polar returned an unexpected response. Try again later.",
            );
        }
        // The configured product is perpetual and has no device or usage limits.
        // A changed benefit must be reconciled rather than silently narrowing rights.
        if response.expires_at.is_some()
            || response.limit_activations.is_some()
            || response.limit_usage.is_some()
        {
            return Validation::Unavailable(
                "The license settings do not match this product. Contact isoboard support.",
            );
        }
        return Validation::Granted {
            support_expires_at: support_expiry(&response.created_at),
            license_id: response.id,
        };
    }
    // Only a recognized Polar rejection changes a cached entitlement. Generic
    // proxy errors, HTML, malformed responses, auth errors and outages do not.
    if status == 404 {
        if let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) {
            if value.get("error").and_then(|v| v.as_str()) == Some("ResourceNotFound") {
                return match value.get("detail").and_then(|v| v.as_str()) {
                    Some("Not found") => Validation::Rejected("This key is invalid or has been replaced. Check your Polar purchase and try again."),
                    Some("License key is no longer active.") => Validation::Rejected("This license is no longer active. Check your Polar purchase or enter a replacement key."),
                    Some("License key has expired.") => Validation::Rejected("Polar reports that this license has expired. Contact isoboard support."),
                    Some("License key does not match given benefit.") => Validation::Rejected("This key does not cover this edition of isoboard."),
                    _ => Validation::Unavailable(UNAVAILABLE),
                };
            }
        }
    }
    if status == 429 {
        return Validation::Unavailable("Polar is receiving too many requests. Try again later.");
    }
    Validation::Unavailable(UNAVAILABLE)
}

impl LicenseProvider for Polar {
    fn validate(&self, key: String) -> ValidationFuture<'_> {
        Box::pin(async move {
            let Some(client) = &self.client else {
                return Validation::Unavailable(UNAVAILABLE);
            };
            let result = client
                .post(&PRODUCT.polar.validation_url)
                .header(reqwest::header::ACCEPT, "application/json")
                .json(&Request {
                    key: &key,
                    organization_id: PRODUCT.polar.organization_id.as_str(),
                    benefit_id: PRODUCT.polar.benefit_id.as_str(),
                })
                .send()
                .await;
            let Ok(mut response) = result else {
                return Validation::Unavailable(UNAVAILABLE);
            };
            let status = response.status().as_u16();
            let mut bytes = Vec::new();
            loop {
                match response.chunk().await {
                    Ok(Some(chunk)) if bytes.len() + chunk.len() <= 64 * 1024 => {
                        bytes.extend_from_slice(&chunk)
                    }
                    Ok(None) => break,
                    _ => return Validation::Unavailable(UNAVAILABLE),
                }
            }
            classify(status, &bytes)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn verifies_identity_and_perpetual_entitlement_without_retaining_customer_data() {
        let mut value = serde_json::json!({"id":"d63fb336-39be-4541-bfe7-a61b8fdbe828", "organization_id":PRODUCT.polar.organization_id.as_str(),"benefit_id":PRODUCT.polar.benefit_id.as_str(),"status":"granted","expires_at":null,"limit_activations":null,"limit_usage":null,"customer":{"email":"private@example.com"},"key":"must-not-be-retained"});
        assert!(matches!(
            classify(200, &serde_json::to_vec(&value).unwrap()),
            Validation::Granted { .. }
        ));
        value["benefit_id"] = "other-benefit".into();
        assert!(matches!(
            classify(200, &serde_json::to_vec(&value).unwrap()),
            Validation::Rejected(_)
        ));
        value["benefit_id"] = PRODUCT.polar.benefit_id.as_str().into();
        value["expires_at"] = "2027-01-01T00:00:00Z".into();
        assert!(matches!(
            classify(200, &serde_json::to_vec(&value).unwrap()),
            Validation::Unavailable(_)
        ));
    }
    #[test]
    fn support_uses_license_issuance_calendar_anniversary_and_never_gates_license() {
        let mut response = serde_json::json!({"id":"d63fb336-39be-4541-bfe7-a61b8fdbe828", "organization_id":PRODUCT.polar.organization_id.as_str(),"benefit_id":PRODUCT.polar.benefit_id.as_str(),"status":"granted","expires_at":null,"limit_activations":null,"limit_usage":null,
            "order":{"paid":true,"status":"paid","created_at":"2020-01-01T00:00:00Z"}});
        for (created_at, expected) in [
            (
                serde_json::json!("2024-02-29T15:08:16Z"),
                Some(
                    chrono::DateTime::parse_from_rfc3339("2025-02-28T15:08:16Z")
                        .unwrap()
                        .timestamp() as u64,
                ),
            ),
            (serde_json::json!("invalid"), None),
            (serde_json::json!(42), None),
            (serde_json::Value::Null, None),
        ] {
            response["created_at"] = created_at;
            assert_eq!(
                classify(200, &serde_json::to_vec(&response).unwrap()),
                Validation::Granted {
                    license_id: "d63fb336-39be-4541-bfe7-a61b8fdbe828".into(),
                    support_expires_at: expected,
                }
            );
        }
        response.as_object_mut().unwrap().remove("created_at");
        assert!(matches!(
            classify(200, &serde_json::to_vec(&response).unwrap()),
            Validation::Granted {
                support_expires_at: None,
                ..
            }
        ));
    }

    #[test]
    fn only_recognized_rejections_remove_permission() {
        for detail in [
            "Not found",
            "License key is no longer active.",
            "License key has expired.",
            "License key does not match given benefit.",
        ] {
            let body = serde_json::to_vec(
                &serde_json::json!({"error":"ResourceNotFound","detail":detail}),
            )
            .unwrap();
            assert!(matches!(classify(404, &body), Validation::Rejected(_)));
        }
        // Polar's version middleware uses this body when a version is retired.
        assert!(matches!(
            classify(404, br#"{"detail":"Not Found"}"#),
            Validation::Unavailable(_)
        ));
        for status in [200, 400, 401, 403, 404, 422, 429, 500, 503] {
            assert!(matches!(
                classify(status, b"<html>proxy failure</html>"),
                Validation::Unavailable(_)
            ));
        }
    }
}
