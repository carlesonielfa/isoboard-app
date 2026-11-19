//! Shared stage request construction and bounded stream consumption.

use base64::Engine as _;
use futures_util::StreamExt;
use genai::chat::{
    Binary, ChatMessage, ChatOptions, ChatRequest, ChatStreamEvent, ContentPart, MessageContent,
    StopReason,
};
use genai::resolver::AuthResolver;
use genai::{Client, ServiceTarget};
use tokio::time::{timeout, Duration};
use tokio_util::sync::CancellationToken;

use crate::inference_profiles::{InferenceOptions, ReasoningLevel};

use super::error::{map_genai_error, InferenceError};
use super::target::Provider;

pub(crate) const MAX_IMAGE_BYTES: usize = 20 * 1024 * 1024;
pub(crate) const MAX_MODEL_BYTES: usize = 256;
pub(crate) const MAX_OUTPUT_BYTES: usize = 1024 * 1024;
// The intermediate provider HTML and literal rules are bounded at their input
// boundaries. Reserve another 64 KiB for the bundled stage template.
pub(crate) const MAX_TEXT_PROMPT_BYTES: usize = MAX_OUTPUT_BYTES + 2 * 64 * 1024;
pub(crate) const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Clone, Copy)]
pub(crate) enum InferenceInput<'a> {
    Image { bytes: &'a [u8], mime_type: &'a str },
    Text,
}

impl InferenceInput<'_> {
    pub(crate) fn request(self, prompt: &str) -> Result<ChatRequest, InferenceError> {
        match self {
            Self::Image { bytes, mime_type } => {
                validate_image(bytes, mime_type)?;
                Ok(build_image_request(prompt, bytes, mime_type))
            }
            Self::Text => {
                validate_text_prompt(prompt)?;
                Ok(ChatRequest::new(vec![ChatMessage::user(prompt)]))
            }
        }
    }
}

pub(crate) fn validate_text_prompt(prompt: &str) -> Result<(), InferenceError> {
    if prompt.len() > MAX_TEXT_PROMPT_BYTES {
        return Err(InferenceError::PromptTooLarge);
    }
    if prompt.trim().is_empty()
        || prompt
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return Err(InferenceError::InvalidPrompt);
    }
    Ok(())
}
/// Shared genai client and stream policy for API inference.
pub(crate) struct InferenceRunner {
    pub(crate) client: Client,
}

impl InferenceRunner {
    pub(crate) fn new() -> Result<Self, InferenceError> {
        let http_client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| InferenceError::Configuration)?;
        let auth_resolver = AuthResolver::from_resolver_fn(|_: genai::ModelIden| {
            Ok(Some(genai::resolver::AuthData::None))
        });
        Ok(Self {
            client: Client::builder()
                .with_reqwest(http_client)
                .with_auth_resolver(auth_resolver)
                .build(),
        })
    }

    pub(crate) async fn extract(
        &self,
        target: ServiceTarget,
        request: ChatRequest,
        options: ChatOptions,
        timeout_duration: Duration,
        cancellation: CancellationToken,
    ) -> Result<String, InferenceError> {
        let result = timeout(
            timeout_duration,
            self.consume_stream(target, request, options, cancellation.clone()),
        )
        .await;
        match result {
            Ok(result) => result,
            Err(_) => Err(InferenceError::TimedOut),
        }
    }

    async fn consume_stream(
        &self,
        target: ServiceTarget,
        request: ChatRequest,
        options: ChatOptions,
        cancellation: CancellationToken,
    ) -> Result<String, InferenceError> {
        let mut response = tokio::select! {
            _ = cancellation.cancelled() => return Err(InferenceError::Cancelled),
            response = self.client.exec_chat_stream(target, request, Some(&options)) => {
                response.map_err(map_genai_error)?
            }
        };

        let mut output = String::new();
        let mut completed = false;
        loop {
            let event = tokio::select! {
                _ = cancellation.cancelled() => return Err(InferenceError::Cancelled),
                event = response.stream.next() => event,
            };
            match event {
                Some(Ok(ChatStreamEvent::Chunk(chunk))) => {
                    let size = output
                        .len()
                        .checked_add(chunk.content.len())
                        .ok_or(InferenceError::OutputTooLarge)?;
                    if size > MAX_OUTPUT_BYTES {
                        return Err(InferenceError::OutputTooLarge);
                    }
                    output.push_str(&chunk.content);
                }
                Some(Ok(ChatStreamEvent::ToolCallChunk(_))) => {
                    return Err(InferenceError::ToolCallRejected)
                }
                Some(Ok(ChatStreamEvent::End(end))) => {
                    if end
                        .captured_tool_calls()
                        .is_some_and(|calls| !calls.is_empty())
                    {
                        return Err(InferenceError::ToolCallRejected);
                    }
                    completed = matches!(end.captured_stop_reason, Some(StopReason::Completed(_)));
                    break;
                }
                Some(Ok(
                    ChatStreamEvent::Start
                    | ChatStreamEvent::ReasoningChunk(_)
                    | ChatStreamEvent::ThoughtSignatureChunk(_),
                )) => {}
                Some(Err(error)) => return Err(map_genai_error(error)),
                None => break,
            }
        }

        if !completed {
            return Err(InferenceError::MalformedStream);
        }
        if cancellation.is_cancelled() {
            return Err(InferenceError::Cancelled);
        }
        if output.trim().is_empty() {
            return Err(InferenceError::NoContent);
        }
        Ok(output)
    }
}

pub(crate) fn chat_options(
    provider: Provider,
    model: &str,
    saved: &InferenceOptions,
) -> Result<ChatOptions, InferenceError> {
    saved
        .validate()
        .map_err(|_| InferenceError::UnsupportedOption("invalid inference option"))?;
    let mut options = ChatOptions::default().with_capture_tool_calls(true);
    if let Some(temperature) = saved.temperature {
        options = options.with_temperature(temperature);
    }
    if let Some(max_output_tokens) = saved.max_output_tokens {
        options = options.with_max_tokens(max_output_tokens);
    }
    if let Some(reasoning) = saved.reasoning {
        let effort = match reasoning {
            ReasoningLevel::None => genai::chat::ReasoningEffort::None,
            ReasoningLevel::Minimal => genai::chat::ReasoningEffort::Minimal,
            ReasoningLevel::Low => genai::chat::ReasoningEffort::Low,
            ReasoningLevel::Medium => genai::chat::ReasoningEffort::Medium,
            ReasoningLevel::High => genai::chat::ReasoningEffort::High,
            ReasoningLevel::Xhigh => genai::chat::ReasoningEffort::XHigh,
            ReasoningLevel::Max => genai::chat::ReasoningEffort::Max,
            ReasoningLevel::On | ReasoningLevel::Off => {
                return Err(InferenceError::UnsupportedOption("reasoning toggle"))
            }
        };
        if matches!(provider, Provider::Anthropic | Provider::Gemini)
            && matches!(reasoning, ReasoningLevel::None)
        {
            return Err(InferenceError::UnsupportedOption("reasoning effort"));
        }
        if matches!(provider, Provider::Anthropic)
            && matches!(
                reasoning,
                ReasoningLevel::Minimal | ReasoningLevel::Xhigh | ReasoningLevel::Max
            )
        {
            return Err(InferenceError::UnsupportedOption("reasoning effort"));
        }
        if matches!(provider, Provider::Gemini)
            && matches!(reasoning, ReasoningLevel::Xhigh | ReasoningLevel::Max)
        {
            return Err(InferenceError::UnsupportedOption("reasoning effort"));
        }
        if matches!(provider, Provider::Gemini)
            && matches!(reasoning, ReasoningLevel::Minimal)
            && !model.contains("gemini-3")
            && !model.contains("gemma-4")
        {
            return Err(InferenceError::UnsupportedOption("reasoning effort"));
        }
        options = options.with_reasoning_effort(effort);
    }
    Ok(options)
}

pub(crate) fn request_timeout(saved: &InferenceOptions) -> Duration {
    Duration::from_secs(u64::from(
        saved
            .request_timeout_seconds
            .unwrap_or(REQUEST_TIMEOUT.as_secs() as u32),
    ))
}

pub(crate) fn build_image_request(
    prompt: &str,
    image_bytes: &[u8],
    mime_type: &str,
) -> ChatRequest {
    let encoded = base64::engine::general_purpose::STANDARD.encode(image_bytes);
    let image = Binary::from_base64(mime_type, encoded, None);
    let content = MessageContent::from_parts(vec![
        ContentPart::Text(prompt.to_owned()),
        ContentPart::Binary(image),
    ]);
    ChatRequest::new(vec![ChatMessage::user(content)])
}

pub(crate) fn validate_image(image: &[u8], mime_type: &str) -> Result<(), InferenceError> {
    if image.len() > MAX_IMAGE_BYTES {
        return Err(InferenceError::ImageTooLarge);
    }
    let valid_mime = matches!(mime_type, "image/png" | "image/jpeg");
    if !valid_mime {
        return Err(InferenceError::InvalidImage);
    }
    let valid_signature = match mime_type {
        "image/png" => image.starts_with(b"\x89PNG\r\n\x1a\n"),
        "image/jpeg" => image.starts_with(&[0xff, 0xd8, 0xff]),
        _ => false,
    };
    if !valid_signature {
        return Err(InferenceError::InvalidImage);
    }
    Ok(())
}
