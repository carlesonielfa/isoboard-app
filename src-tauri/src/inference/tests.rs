use super::*;
use serde_json::Value;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::sync::CancellationToken;

const PNG: &[u8] = b"\x89PNG\r\n\x1a\nfixture";
const TEXT_PROMPT: &str = "Apply literal {{ rules }}: \"preserve\".\n<p>sanitized &amp; HTML</p>\n";

fn assert_text_request(request: &Value, provider: Provider) {
    let content = match provider {
        Provider::OpenAi | Provider::OpenAiCompatible => &request["messages"][0]["content"],
        Provider::Anthropic => &request["messages"][0]["content"],
        Provider::Gemini => &request["contents"][0]["parts"][0]["text"],
    };
    assert_eq!(content, TEXT_PROMPT);
    let encoded = request.to_string();
    for forbidden in [
        "image_url",
        "input_image",
        "inline_data",
        "base64",
        "iVBORw",
    ] {
        assert!(
            !encoded.contains(forbidden),
            "unexpected image field: {forbidden}"
        );
    }
}

#[tokio::test]
async fn product_service_rejects_a_missing_credential_before_transport() {
    let service = InferenceService::new(None).expect("inference service");
    let selection = FrozenInference {
        processing_profile_id: "default".into(),
        backend: crate::inference_profiles::InferenceBackend::GeminiApi,
        model: "vision-model".into(),
        options: crate::inference_profiles::InferenceOptions::default(),
        endpoint: None,
        credential_ref: None,
    };
    let result = service
        .extract(
            &selection,
            crate::inference::params::default_generic_conversion_prompt(),
            PNG,
            "image/png",
            CancellationToken::new(),
        )
        .await;
    assert_eq!(result, Err(InferenceError::MissingCredential));
    assert_eq!(
        service
            .process_text(&selection, TEXT_PROMPT, CancellationToken::new())
            .await,
        Err(InferenceError::MissingCredential)
    );
    for backend in [
        InferenceBackend::OpenaiApi,
        InferenceBackend::AnthropicApi,
        InferenceBackend::GeminiApi,
    ] {
        let mut missing = selection.clone();
        missing.backend = backend;
        assert_eq!(
            missing.validate_credential_reference(),
            Err(InferenceError::MissingCredential)
        );
        assert_eq!(
            service
                .process_text(&missing, TEXT_PROMPT, CancellationToken::new())
                .await,
            Err(InferenceError::MissingCredential)
        );
        missing.credential_ref = Some(
            ManagedSecretRef::parse("0123456789abcdef0123456789abcdef")
                .expect("opaque test reference"),
        );
        assert_eq!(missing.validate_credential_reference(), Ok(()));
    }
}

async fn read_request(stream: &mut TcpStream) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 4096];
    let (header_len, body_len) = loop {
        let count = stream.read(&mut chunk).await.expect("headers");
        assert!(count > 0);
        bytes.extend_from_slice(&chunk[..count]);
        if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            let headers = String::from_utf8_lossy(&bytes[..end]);
            let len = headers
                .lines()
                .find_map(|line| {
                    line.split_once(':').and_then(|(name, value)| {
                        name.eq_ignore_ascii_case("content-length")
                            .then_some(value.trim())
                    })
                })
                .expect("content length")
                .parse::<usize>()
                .expect("length");
            break (end + 4, len);
        }
    };
    while bytes.len() < header_len + body_len {
        let count = stream.read(&mut chunk).await.expect("body");
        assert!(count > 0);
        bytes.extend_from_slice(&chunk[..count]);
    }
    bytes.truncate(header_len + body_len);
    bytes
}

async fn run_fixture(
    provider: Provider,
    stream_body: &str,
) -> (Result<String, InferenceError>, Value) {
    run_fixture_with_image(provider, stream_body, PNG, "image/png").await
}

async fn run_fixture_with_image(
    provider: Provider,
    stream_body: &str,
    image: &[u8],
    mime_type: &str,
) -> (Result<String, InferenceError>, Value) {
    run_fixture_with_input(
        provider,
        stream_body,
        default_generic_conversion_prompt(),
        InferenceInput::Image {
            bytes: image,
            mime_type,
        },
    )
    .await
}

async fn run_fixture_with_input(
    provider: Provider,
    stream_body: &str,
    prompt: &str,
    input: InferenceInput<'_>,
) -> (Result<String, InferenceError>, Value) {
    run_fixture_with_options(
        provider,
        stream_body,
        prompt,
        input,
        crate::inference_profiles::InferenceOptions::default(),
    )
    .await
}

async fn run_fixture_with_options(
    provider: Provider,
    stream_body: &str,
    prompt: &str,
    input: InferenceInput<'_>,
    options: crate::inference_profiles::InferenceOptions,
) -> (Result<String, InferenceError>, Value) {
    run_fixture_with_model_options(
        provider,
        "fixture-model",
        stream_body,
        prompt,
        input,
        options,
    )
    .await
}

async fn run_fixture_with_model_options(
    provider: Provider,
    model: &str,
    stream_body: &str,
    prompt: &str,
    input: InferenceInput<'_>,
    options: crate::inference_profiles::InferenceOptions,
) -> (Result<String, InferenceError>, Value) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let address = listener.local_addr().expect("address");
    let body = stream_body.to_owned();
    let (sender, receiver) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept");
        let request = read_request(&mut stream).await;
        sender.send(request).expect("request sender");
        let response = format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
        stream
            .write_all(response.as_bytes())
            .await
            .expect("response");
    });
    let endpoint = format!("http://127.0.0.1:{}/v1/", address.port());
    let config = InferenceConfig::new(provider, model, "fixture-key")
        .with_base_url(endpoint)
        .with_options(options);
    let runtime = ApiInference::new().expect("test HTTP client");
    let result = runtime
        .infer_at_for_test(&config, prompt, input, CancellationToken::new())
        .await;
    let request = tokio::time::timeout(Duration::from_secs(2), receiver)
        .await
        .expect("request capture timeout")
        .expect("request capture");
    let header_end = request
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("headers");
    (
        result,
        serde_json::from_slice(&request[header_end + 4..]).expect("JSON body"),
    )
}

impl ApiInference {
    async fn extract_at_for_test_with_prompt(
        &self,
        config: &InferenceConfig,
        prompt: &str,
        image: &[u8],
        mime_type: &str,
        cancellation: CancellationToken,
    ) -> Result<String, InferenceError> {
        self.infer_at_for_test(
            config,
            prompt,
            InferenceInput::Image {
                bytes: image,
                mime_type,
            },
            cancellation,
        )
        .await
    }

    async fn infer_at_for_test(
        &self,
        config: &InferenceConfig,
        prompt: &str,
        input: InferenceInput<'_>,
        cancellation: CancellationToken,
    ) -> Result<String, InferenceError> {
        let mut validation = config.clone();
        if validation.provider != Provider::OpenAiCompatible {
            validation.base_url = None;
        }
        validate_api_config(&validation)?;
        let request = input.request(prompt)?;
        let endpoint = config
            .base_url
            .clone()
            .ok_or(InferenceError::InvalidEndpoint)?;
        let target = target::api_target(&self.runner.client, &validation, Some(&endpoint)).await?;
        let options =
            runner::chat_options(validation.provider, &validation.model, &validation.options)?;
        self.runner
            .extract(
                target,
                request,
                options,
                runner::request_timeout(&validation.options),
                cancellation,
            )
            .await
    }
}

#[tokio::test]
async fn custom_conversion_prompt_is_forwarded_without_changing_image_request() {
    let body = concat!(
        "data: {\"choices\":[{\"delta\":{\"content\":\"<p>ok</p>\"},\"finish_reason\":null}]}\n\n",
        "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
        "data: [DONE]\n\n",
    );
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let address = listener.local_addr().expect("address");
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let response_body = body.to_owned();
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept");
        let request = read_request(&mut stream).await;
        sender.send(request).expect("request capture");
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response_body.len(),
            response_body
        );
        stream
            .write_all(response.as_bytes())
            .await
            .expect("response");
    });
    let endpoint = format!("http://127.0.0.1:{}/v1/", address.port());
    let config = InferenceConfig::new(Provider::OpenAiCompatible, "fixture-model", "fixture-key")
        .with_base_url(endpoint);
    let runtime = ApiInference::new().expect("test HTTP client");
    let result = runtime
        .extract_at_for_test_with_prompt(
            &config,
            "custom conversion prompt",
            PNG,
            "image/png",
            CancellationToken::new(),
        )
        .await;
    assert_eq!(result.expect("completion"), "<p>ok</p>");
    let request = tokio::time::timeout(Duration::from_secs(2), receiver)
        .await
        .expect("request timeout")
        .expect("request capture");
    let header_end = request
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("headers");
    let request: Value = serde_json::from_slice(&request[header_end + 4..]).expect("JSON body");
    assert_eq!(
        request["messages"][0]["content"][0]["text"],
        "custom conversion prompt"
    );
}

#[tokio::test]
async fn openai_stream_uses_image_and_requires_done() {
    let body = concat!(
        "data: {\"choices\":[{\"delta\":{\"content\":\"<p>ok</p>\"},\"finish_reason\":null}]}\n\n",
        "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
        "data: [DONE]\n\n",
    );
    let (result, request) = run_fixture(Provider::OpenAiCompatible, body).await;
    assert_eq!(result.expect("completion"), "<p>ok</p>");
    assert_eq!(request["model"], "fixture-model");
    assert_eq!(
        request["messages"][0]["content"][1]["image_url"]["url"],
        "data:image/png;base64,iVBORw0KGgpmaXh0dXJl"
    );
    for provider in [Provider::OpenAi, Provider::OpenAiCompatible] {
        let (result, request) =
            run_fixture_with_input(provider, body, TEXT_PROMPT, InferenceInput::Text).await;
        assert_eq!(result.expect("text completion"), "<p>ok</p>");
        assert_text_request(&request, provider);
    }
}

#[tokio::test]
async fn openai_compatible_options_are_forwarded_for_image_and_text() {
    let body = concat!(
        "data: {\"choices\":[{\"delta\":{\"content\":\"ok\"},\"finish_reason\":null}]}\n\n",
        "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
        "data: [DONE]\n\n",
    );
    let options = crate::inference_profiles::InferenceOptions {
        temperature: Some(0.35),
        max_output_tokens: Some(321),
        reasoning: Some(crate::inference_profiles::ReasoningLevel::High),
        request_timeout_seconds: None,
    };
    for input in [
        InferenceInput::Image {
            bytes: PNG,
            mime_type: "image/png",
        },
        InferenceInput::Text,
    ] {
        let (result, request) = run_fixture_with_options(
            Provider::OpenAiCompatible,
            body,
            TEXT_PROMPT,
            input,
            options.clone(),
        )
        .await;
        assert_eq!(result.expect("completion"), "ok");
        assert_eq!(request["temperature"], 0.35);
        assert_eq!(request["max_tokens"], 321);
        assert_eq!(request["reasoning_effort"], "high");
    }
    for input in [
        InferenceInput::Image {
            bytes: PNG,
            mime_type: "image/png",
        },
        InferenceInput::Text,
    ] {
        let (result, request) =
            run_fixture_with_input(Provider::OpenAiCompatible, body, TEXT_PROMPT, input).await;
        assert_eq!(result.expect("completion"), "ok");
        assert!(request.get("temperature").is_none());
        assert!(request.get("max_tokens").is_none());
        assert!(request.get("reasoning_effort").is_none());
    }

    let (result, request) = run_fixture_with_model_options(
        Provider::OpenAiCompatible,
        "gpt-5.1",
        body,
        TEXT_PROMPT,
        InferenceInput::Text,
        crate::inference_profiles::InferenceOptions {
            temperature: None,
            max_output_tokens: Some(321),
            reasoning: Some(crate::inference_profiles::ReasoningLevel::Low),
            request_timeout_seconds: None,
        },
    )
    .await;
    assert_eq!(result.expect("completion"), "ok");
    assert_eq!(request["max_completion_tokens"], 321);
    assert!(request.get("max_tokens").is_none());
    assert_eq!(request["reasoning_effort"], "low");
}

#[tokio::test]
async fn known_provider_option_payloads_preserve_reasoning_levels() {
    let anthropic_body = concat!(
        "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":1}}}\n\n",
        "event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n",
        "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"ok\"}}\n\n",
        "event: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
        "event: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"}}\n\n",
        "event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n",
    );
    for (reasoning, expected_effort) in [
        (crate::inference_profiles::ReasoningLevel::Low, "low"),
        (crate::inference_profiles::ReasoningLevel::Medium, "medium"),
        (crate::inference_profiles::ReasoningLevel::High, "high"),
    ] {
        let (result, request) = run_fixture_with_model_options(
            Provider::Anthropic,
            "claude-opus-4-6",
            anthropic_body,
            TEXT_PROMPT,
            InferenceInput::Text,
            crate::inference_profiles::InferenceOptions {
                temperature: Some(0.35),
                max_output_tokens: Some(321),
                reasoning: Some(reasoning),
                request_timeout_seconds: None,
            },
        )
        .await;
        assert_eq!(result.expect("completion"), "ok");
        assert_eq!(request["temperature"], 0.35);
        assert_eq!(request["max_tokens"], 321);
        assert_eq!(request["output_config"]["effort"], expected_effort);
    }

    let gemini_body = concat!(
        "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"ok\"}]}}]}\n\n",
        "data: {\"candidates\":[{\"finishReason\":\"STOP\"}]}\n\n",
    );
    for (reasoning, expected_level) in [
        (crate::inference_profiles::ReasoningLevel::Low, "LOW"),
        (crate::inference_profiles::ReasoningLevel::Medium, "MEDIUM"),
        (crate::inference_profiles::ReasoningLevel::High, "HIGH"),
    ] {
        let (result, request) = run_fixture_with_model_options(
            Provider::Gemini,
            "gemini-3-pro",
            gemini_body,
            TEXT_PROMPT,
            InferenceInput::Image {
                bytes: PNG,
                mime_type: "image/png",
            },
            crate::inference_profiles::InferenceOptions {
                temperature: Some(0.35),
                max_output_tokens: Some(321),
                reasoning: Some(reasoning),
                request_timeout_seconds: None,
            },
        )
        .await;
        assert_eq!(result.expect("completion"), "ok");
        assert_eq!(request["generationConfig"]["temperature"], 0.35);
        assert_eq!(request["generationConfig"]["maxOutputTokens"], 321);
        assert_eq!(
            request["generationConfig"]["thinkingConfig"]["thinkingLevel"],
            expected_level
        );
    }
}

#[test]
fn unsupported_reasoning_levels_are_rejected_before_transport() {
    for (provider, model, reasoning) in [
        (
            Provider::Anthropic,
            "claude-opus-4-6",
            crate::inference_profiles::ReasoningLevel::Minimal,
        ),
        (
            Provider::Anthropic,
            "claude-opus-4-6",
            crate::inference_profiles::ReasoningLevel::Xhigh,
        ),
        (
            Provider::Anthropic,
            "claude-opus-4-6",
            crate::inference_profiles::ReasoningLevel::Max,
        ),
        (
            Provider::Gemini,
            "gemini-2.5-pro",
            crate::inference_profiles::ReasoningLevel::Minimal,
        ),
        (
            Provider::Gemini,
            "gemini-2.5-pro",
            crate::inference_profiles::ReasoningLevel::Xhigh,
        ),
        (
            Provider::Gemini,
            "gemini-2.5-pro",
            crate::inference_profiles::ReasoningLevel::Max,
        ),
    ] {
        let options = crate::inference_profiles::InferenceOptions {
            reasoning: Some(reasoning),
            ..crate::inference_profiles::InferenceOptions::default()
        };
        assert!(matches!(
            runner::chat_options(provider, model, &options),
            Err(InferenceError::UnsupportedOption("reasoning effort"))
        ));
    }
}

#[tokio::test]
async fn anthropic_stream_is_completed() {
    let body = concat!(
            "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":1}}}\n\n",
            "event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n",
            "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"hello\"}}\n\n",
            "event: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
            "event: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"}}\n\n",
            "event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n",
        );
    let (result, request) = run_fixture(Provider::Anthropic, body).await;
    assert_eq!(result.expect("completion"), "hello");
    assert_eq!(
        request["messages"][0]["content"][1]["source"]["type"],
        "base64"
    );
    let (result, request) =
        run_fixture_with_input(Provider::Anthropic, body, TEXT_PROMPT, InferenceInput::Text).await;
    assert_eq!(result.expect("text completion"), "hello");
    assert_text_request(&request, Provider::Anthropic);
}

#[tokio::test]
async fn gemini_stream_is_completed() {
    let body = concat!(
        "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"hello\"}]}}]}\n\n",
        "data: {\"candidates\":[{\"finishReason\":\"STOP\"}]}\n\n",
    );
    let (result, request) = run_fixture(Provider::Gemini, body).await;
    assert_eq!(result.expect("completion"), "hello");
    assert_eq!(
        request["contents"][0]["parts"][1]["inline_data"]["mime_type"],
        "image/png"
    );
    let (result, request) =
        run_fixture_with_input(Provider::Gemini, body, TEXT_PROMPT, InferenceInput::Text).await;
    assert_eq!(result.expect("text completion"), "hello");
    assert_text_request(&request, Provider::Gemini);
}

#[tokio::test]
async fn jpeg_input_preserves_its_mime_type() {
    let body = concat!(
        "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"hello\"}]}}]}\n\n",
        "data: {\"candidates\":[{\"finishReason\":\"STOP\"}]}\n\n",
    );
    let (result, request) =
        run_fixture_with_image(Provider::Gemini, body, b"\xff\xd8\xfffixture", "image/jpeg").await;
    assert_eq!(result.expect("completion"), "hello");
    assert_eq!(
        request["contents"][0]["parts"][1]["inline_data"]["mime_type"],
        "image/jpeg"
    );
}

#[tokio::test]
async fn truncated_stream_and_invalid_endpoint_are_rejected() {
    let (result, _) = run_fixture(
        Provider::OpenAiCompatible,
        "data: {\"choices\":[{\"delta\":{\"content\":\"partial\"}}]}\n\n",
    )
    .await;
    assert_eq!(result, Err(InferenceError::MalformedStream));
    let bad = InferenceConfig::new(Provider::OpenAiCompatible, "model", "key")
        .with_base_url("http://example.com/v1/?key=secret");
    assert_eq!(validate_config(&bad), Err(InferenceError::InvalidEndpoint));
}

#[tokio::test]
async fn cancellation_and_bounds_are_enforced() {
    let token = CancellationToken::new();
    token.cancel();
    let runtime = ApiInference::new().expect("test HTTP client");
    let config = InferenceConfig::new(Provider::OpenAiCompatible, "model", "key")
        .with_base_url("http://127.0.0.1:1/v1/");
    assert_eq!(
        runtime.extract(&config, PNG, "image/png", token).await,
        Err(InferenceError::Cancelled)
    );
    assert_eq!(
        runtime
            .extract(&config, b"bad", "image/png", CancellationToken::new())
            .await,
        Err(InferenceError::InvalidImage)
    );
}

#[tokio::test]
async fn text_prompt_validation_and_cancelled_requests_fail_before_transport() {
    let runtime = ApiInference::new().expect("test HTTP client");
    let config = InferenceConfig::new(Provider::OpenAiCompatible, "model", "key")
        .with_base_url("http://127.0.0.1:1/v1/");
    for invalid in [" \n\t\r", "<p>bad\0control</p>"] {
        assert_eq!(
            runtime
                .process_text(&config, invalid, CancellationToken::new())
                .await,
            Err(InferenceError::InvalidPrompt)
        );
    }
    assert_eq!(
        runtime
            .process_text(
                &config,
                &"é".repeat(runner::MAX_TEXT_PROMPT_BYTES / 2 + 1),
                CancellationToken::new()
            )
            .await,
        Err(InferenceError::PromptTooLarge)
    );
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    assert_eq!(
        runtime
            .process_text(&config, TEXT_PROMPT, cancellation)
            .await,
        Err(InferenceError::Cancelled)
    );
}

#[tokio::test]
async fn text_stream_reuses_completion_and_output_bounds() {
    let (result, _) = run_fixture_with_input(
        Provider::OpenAiCompatible,
        "data: {\"choices\":[{\"delta\":{\"content\":\"partial\"}}]}\n\n",
        TEXT_PROMPT,
        InferenceInput::Text,
    )
    .await;
    assert_eq!(result, Err(InferenceError::MalformedStream));
    let oversized = serde_json::json!({"choices": [{"delta": {"content": "x".repeat(runner::MAX_OUTPUT_BYTES + 1)}, "finish_reason": null}]});
    let body = format!("data: {oversized}\n\ndata: {{\"choices\":[{{\"delta\":{{}},\"finish_reason\":\"stop\"}}]}}\n\ndata: [DONE]\n\n");
    let (result, _) = run_fixture_with_input(
        Provider::OpenAiCompatible,
        &body,
        TEXT_PROMPT,
        InferenceInput::Text,
    )
    .await;
    assert_eq!(result, Err(InferenceError::OutputTooLarge));
}

#[tokio::test]
async fn text_request_cancels_while_provider_stream_remains_open() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let address = listener.local_addr().expect("address");
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept");
        read_request(&mut stream).await;
        stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\ndata: {\"choices\":[{\"delta\":{\"content\":\"partial\"}}]}\n\n").await.expect("partial response");
        sender.send(()).expect("ready receiver");
        std::future::pending::<()>().await;
    });
    let config = InferenceConfig::new(Provider::OpenAiCompatible, "model", "key")
        .with_base_url(format!("http://127.0.0.1:{}/v1/", address.port()));
    let runtime = ApiInference::new().expect("test HTTP client");
    let cancellation = CancellationToken::new();
    let request_cancellation = cancellation.clone();
    let request = tokio::spawn(async move {
        runtime
            .infer_at_for_test(
                &config,
                TEXT_PROMPT,
                InferenceInput::Text,
                request_cancellation,
            )
            .await
    });
    tokio::time::timeout(Duration::from_secs(2), receiver)
        .await
        .expect("provider readiness timeout")
        .expect("provider readiness");
    cancellation.cancel();
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(2), request)
            .await
            .expect("cancellation timeout")
            .expect("request task"),
        Err(InferenceError::Cancelled)
    );
    server.abort();
}

#[tokio::test]
async fn whole_stream_timeout_applies_while_provider_remains_open() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let address = listener.local_addr().expect("address");
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept");
        read_request(&mut stream).await;
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: keep-alive\r\n\r\ndata: {\"choices\":[{\"delta\":{\"content\":\"partial\"}}]}\n\n",
            )
            .await
            .expect("partial response");
        sender.send(()).expect("ready receiver");
        std::future::pending::<()>().await;
    });
    let endpoint = format!("http://127.0.0.1:{}/v1/", address.port());
    let config = InferenceConfig::new(Provider::OpenAiCompatible, "fixture-model", "fixture-key")
        .with_base_url(endpoint.clone());
    let runtime = ApiInference::new().expect("test HTTP client");
    let target = target::api_target(&runtime.runner.client, &config, None)
        .await
        .expect("target");
    let request = InferenceInput::Text.request(TEXT_PROMPT).expect("request");
    let options =
        runner::chat_options(Provider::OpenAiCompatible, "fixture-model", &config.options)
            .expect("options");
    let inference = tokio::spawn(async move {
        runtime
            .runner
            .extract(
                target,
                request,
                options,
                Duration::from_millis(100),
                CancellationToken::new(),
            )
            .await
    });
    tokio::time::timeout(Duration::from_secs(2), receiver)
        .await
        .expect("provider readiness timeout")
        .expect("provider readiness");
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(2), inference)
            .await
            .expect("whole-stream timeout wait")
            .expect("inference task"),
        Err(InferenceError::TimedOut)
    );
    server.abort();
}

#[test]
fn only_openai_compatible_accepts_an_empty_api_key() {
    for provider in [Provider::OpenAi, Provider::Anthropic, Provider::Gemini] {
        assert_eq!(
            validate_config(&InferenceConfig::new(provider, "model", "")),
            Err(InferenceError::InvalidApiKey)
        );
    }
    let keyless = InferenceConfig::new(Provider::OpenAiCompatible, "model", "");
    assert_eq!(
        validate_config(&keyless),
        Err(InferenceError::InvalidEndpoint)
    );
    assert_eq!(
        validate_config(&keyless.clone().with_base_url("http://127.0.0.1:1/v1/")),
        Ok(())
    );
    assert_eq!(
        validate_config(
            &InferenceConfig::new(Provider::OpenAiCompatible, "model", " ")
                .with_base_url("http://127.0.0.1:1/v1/")
        ),
        Err(InferenceError::InvalidApiKey)
    );
}

#[tokio::test]
async fn openai_compatible_sends_authorization_only_with_a_key() {
    let body = concat!(
        "data: {\"choices\":[{\"delta\":{\"content\":\"ok\"},\"finish_reason\":null}]}\n\n",
        "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
        "data: [DONE]\n\n",
    );
    for (api_key, expected) in [("", None), ("fixture-key", Some("Bearer fixture-key"))] {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let address = listener.local_addr().expect("address");
        let (sender, receiver) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept");
            sender
                .send(read_request(&mut stream).await)
                .expect("request capture");
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream
                .write_all(response.as_bytes())
                .await
                .expect("response");
        });
        let config = InferenceConfig::new(Provider::OpenAiCompatible, "fixture-model", api_key)
            .with_base_url(format!("http://127.0.0.1:{}/v1", address.port()));
        let runtime = ApiInference::new().expect("test HTTP client");
        let result = runtime
            .process_text(&config, TEXT_PROMPT, CancellationToken::new())
            .await;
        assert_eq!(result.expect("completion"), "ok");
        let request = tokio::time::timeout(Duration::from_secs(2), receiver)
            .await
            .expect("request timeout")
            .expect("request capture");
        let header_end = request
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .expect("headers");
        let head = String::from_utf8_lossy(&request[..header_end]).into_owned();
        assert!(head.starts_with("POST /v1/chat/completions "), "{head}");
        let authorization = head.lines().find_map(|line| {
            line.split_once(':')
                .filter(|(name, _)| name.eq_ignore_ascii_case("authorization"))
                .map(|(_, value)| value.trim().to_owned())
        });
        assert_eq!(authorization.as_deref(), expected);
        let request: Value = serde_json::from_slice(&request[header_end + 4..]).expect("JSON body");
        assert_text_request(&request, Provider::OpenAiCompatible);
    }
}

#[test]
fn openai_compatible_needs_only_an_endpoint_to_be_ready() {
    let mut selection = FrozenInference {
        processing_profile_id: "default".into(),
        backend: InferenceBackend::OpenaiCompatible,
        model: "model".into(),
        options: crate::inference_profiles::InferenceOptions::default(),
        endpoint: None,
        credential_ref: None,
    };
    assert!(!selection.keyless());
    assert_eq!(
        selection.validate_credential_reference(),
        Err(InferenceError::MissingCredential)
    );
    selection.endpoint = Some("http://localhost:11434/v1".into());
    assert!(selection.keyless());
    assert_eq!(selection.validate_credential_reference(), Ok(()));
}
