//! Shared, sanitized failures for product and probe inference transports.

/// Upper bound for the provider response kept in a failure report.
const MAX_PROVIDER_RESPONSE_BYTES: usize = 2048;

/// Failures returned by image and text inference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InferenceError {
    MissingAssignment,
    MissingCredential,
    LocalModelMissing,
    LocalModelLoad,
    LocalModelDownload,
    InvalidModel,
    InvalidApiKey,
    InvalidImage,
    ImageTooLarge,
    InvalidPrompt,
    PromptTooLarge,
    InvalidEndpoint,
    EndpointNotAllowed,
    Cancelled,
    TimedOut,
    AuthenticationRequired,
    AccessDenied,
    RateLimited,
    Transport,
    CompletionFailed,
    MalformedStream,
    NoContent,
    OutputTooLarge,
    ToolCallRejected,
    UnsupportedOption(&'static str),
    Configuration,
    /// A provider-reported failure with the provider's own response, kept for
    /// diagnostics. `kind` is one of the transport-level variants above.
    Provider(Box<ProviderFailure>),
}

/// What the provider returned for a failed request. The response is the
/// provider's error body or error event, never the request payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderFailure {
    pub kind: InferenceError,
    pub status: Option<u16>,
    pub response: String,
}

impl InferenceError {
    /// The underlying failure, looking through provider diagnostics.
    pub fn kind(&self) -> &InferenceError {
        match self {
            Self::Provider(failure) => &failure.kind,
            other => other,
        }
    }

    /// Provider response suitable for copied diagnostics, when one exists.
    pub fn provider_response(&self) -> Option<String> {
        let Self::Provider(failure) = self else {
            return None;
        };
        Some(match failure.status {
            Some(status) => {
                let reason = reqwest::StatusCode::from_u16(status)
                    .ok()
                    .and_then(|status| status.canonical_reason())
                    .map(|reason| format!(" {reason}"))
                    .unwrap_or_default();
                format!("HTTP {status}{reason}: {}", failure.response)
            }
            None => failure.response.clone(),
        })
    }
}

impl std::fmt::Display for InferenceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::Provider(failure) => return failure.kind.fmt(formatter),
            Self::MissingAssignment => "the profile needs a model for each active stage",
            Self::MissingCredential => "provider credentials are not configured",
            Self::LocalModelMissing => "the selected local model is not downloaded",
            Self::LocalModelDownload => {
                "the model download failed; check your connection and try capture again"
            }
            Self::LocalModelLoad => "the selected local model could not be loaded",
            Self::InvalidModel => "the model identifier is invalid",
            Self::InvalidApiKey => "the API key is invalid",
            Self::InvalidImage => "the image is invalid",
            Self::ImageTooLarge => "the image is too large",
            Self::InvalidPrompt => "the text prompt is invalid",
            Self::PromptTooLarge => "the text prompt is too large",
            Self::InvalidEndpoint => "the endpoint is invalid",
            Self::EndpointNotAllowed => "the endpoint is not allowed",
            Self::Cancelled => "inference was cancelled",
            Self::TimedOut => "inference timed out",
            Self::AuthenticationRequired => "provider authentication is required",
            Self::AccessDenied => "provider access was denied",
            Self::RateLimited => "provider inference is rate limited",
            Self::Transport => "the inference request failed",
            Self::CompletionFailed => "the provider did not complete the response",
            Self::MalformedStream => "the provider response was malformed or truncated",
            Self::NoContent => "the provider completed without text content",
            Self::OutputTooLarge => "the provider output is too large",
            Self::ToolCallRejected => "the provider returned an unsupported tool call",
            Self::UnsupportedOption(option) => {
                return write!(
                    formatter,
                    "the inference option {option} is not supported by this provider or model"
                )
            }
            Self::Configuration => "the inference client could not be configured",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for InferenceError {}

pub(crate) fn map_genai_error(error: genai::Error) -> InferenceError {
    let (status, response) = match &error {
        genai::Error::HttpError { status, body, .. } => (Some(*status), Some(body.clone())),
        genai::Error::WebStream { error, cause, .. } => {
            match error.downcast_ref::<genai::Error>() {
                Some(genai::Error::HttpError { status, body, .. }) => {
                    (Some(*status), Some(body.clone()))
                }
                _ => (None, Some(cause.clone())),
            }
        }
        genai::Error::ChatResponse { body, .. } => (None, Some(error_event_text(body))),
        // Stream parse errors carry the provider message that failed to parse.
        genai::Error::StreamParse { serde_error, .. } => (None, Some(serde_error.to_string())),
        _ => (None, None),
    };

    let kind = if matches!(error, genai::Error::ChatResponse { .. }) {
        InferenceError::CompletionFailed
    } else if matches!(error, genai::Error::StreamParse { .. }) {
        InferenceError::MalformedStream
    } else {
        match status.map(|status| status.as_u16()) {
            Some(401) => InferenceError::AuthenticationRequired,
            Some(403) => InferenceError::AccessDenied,
            Some(429) => InferenceError::RateLimited,
            Some(400..=599) => InferenceError::CompletionFailed,
            _ => InferenceError::Transport,
        }
    };
    match response.map(|response| sanitize_provider_response(&response)) {
        Some(response) if !response.is_empty() => {
            InferenceError::Provider(Box::new(ProviderFailure {
                kind,
                status: status.map(|status| status.as_u16()),
                response,
            }))
        }
        _ => kind,
    }
}

/// Error events can embed the whole response object, including the prompt, so
/// keep only its `error` member when the provider supplies one.
fn error_event_text(body: &serde_json::Value) -> String {
    let error = body
        .pointer("/response/error")
        .or_else(|| body.get("error"))
        .unwrap_or(body);
    error.to_string()
}

/// Collapse control characters, drop URL query strings, and bound the size.
fn sanitize_provider_response(response: &str) -> String {
    let mut text: String = response
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect();
    text = redact_url_queries(
        text.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .as_str(),
    );
    if text.len() > MAX_PROVIDER_RESPONSE_BYTES {
        let mut end = MAX_PROVIDER_RESPONSE_BYTES;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
        text.push('…');
    }
    text
}

/// Transport errors quote request URLs; query strings may carry credentials.
fn redact_url_queries(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut in_query = false;
    let mut in_url = false;
    for (index, character) in text.char_indices() {
        if character.is_whitespace() || matches!(character, ')' | '"' | '\'' | '>') {
            in_url = false;
            in_query = false;
        } else if !in_url
            && (text[index..].starts_with("http://") || text[index..].starts_with("https://"))
        {
            in_url = true;
        } else if in_url && character == '?' && !in_query {
            in_query = true;
            output.push_str("?…");
            continue;
        }
        if !in_query {
            output.push(character);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_response_is_bounded_and_drops_url_queries() {
        let long = "x".repeat(MAX_PROVIDER_RESPONSE_BYTES * 2);
        let text = sanitize_provider_response(&format!("bad\nline {long}"));
        assert!(text.starts_with("bad line "));
        assert!(text.len() <= MAX_PROVIDER_RESPONSE_BYTES + '…'.len_utf8());
        assert_eq!(
            sanitize_provider_response(
                "error sending request for url (https://h.test/a?key=secret) x"
            ),
            "error sending request for url (https://h.test/a?…) x"
        );
    }

    #[test]
    fn error_events_keep_only_the_error_member() {
        let body = serde_json::json!({
            "type": "response.failed",
            "response": {"instructions": "prompt", "error": {"message": "boom"}}
        });
        assert_eq!(error_event_text(&body), r#"{"message":"boom"}"#);
    }
}
