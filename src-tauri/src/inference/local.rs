//! Embedded local runtime, owned by a worker thread or an isolated probe.

use std::{
    num::NonZeroU32,
    path::Path,
    time::{Duration, Instant},
};

use llama_cpp_2::{
    context::params::LlamaContextParams,
    llama_backend::LlamaBackend,
    llama_batch::LlamaBatch,
    model::{params::LlamaModelParams, LlamaModel},
    mtmd::{mtmd_default_marker, MtmdBitmap, MtmdContext, MtmdContextParams, MtmdInputText},
    sampling::LlamaSampler,
};
use minijinja::{context, Environment};
use serde::Serialize;
use tokio_util::sync::CancellationToken;

use super::{
    params::default_generic_conversion_prompt,
    runner::{MAX_IMAGE_BYTES, MAX_OUTPUT_BYTES, MAX_TEXT_PROMPT_BYTES},
};

pub type LocalResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Limits {
    pub context: NonZeroU32,
    pub output_tokens: u32,
    pub output_bytes: usize,
    pub timeout_ms: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            context: NonZeroU32::new(8192).unwrap(),
            output_tokens: 2048,
            output_bytes: MAX_OUTPUT_BYTES,
            timeout_ms: 120_000,
        }
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    EndOfGeneration,
    OutputTokens,
    OutputBytes,
    Cancelled,
    Timeout,
}

#[derive(Debug, Serialize)]
pub struct Response {
    pub text: String,
    pub formatted_prompt: String,
    pub prompt_tokens: usize,
    pub prompt_positions: i32,
    pub generated_tokens: u32,
    pub prefill_ms: u128,
    pub generation_ms: u128,
    pub stop_reason: StopReason,
}

/// Own on one worker thread. Each request gets a fresh context; weights are reused.
pub struct LocalRuntime {
    // Projector must drop before the model and backend it references.
    projector: MtmdContext,
    model: LlamaModel,
    backend: LlamaBackend,
    pub load_ms: u128,
}

impl LocalRuntime {
    pub fn load(model_path: &Path, projector_path: &Path) -> LocalResult<Self> {
        let started = Instant::now();
        let backend = LlamaBackend::init()?;
        let model = LlamaModel::load_from_file(
            &backend,
            model_path,
            &LlamaModelParams::default().with_n_gpu_layers(1000),
        )?;
        let projector = MtmdContext::init_from_file(
            projector_path
                .to_str()
                .ok_or("projector path must be UTF-8")?,
            &model,
            &MtmdContextParams {
                use_gpu: true,
                n_threads: 4,
                ..Default::default()
            },
        )?;
        if !projector.support_vision() {
            return Err("projector does not support vision".into());
        }
        Ok(Self {
            projector,
            model,
            backend,
            load_ms: started.elapsed().as_millis(),
        })
    }

    pub fn conversion_prompt() -> &'static str {
        default_generic_conversion_prompt()
    }

    pub fn infer(
        &mut self,
        prompt: &str,
        image: Option<&[u8]>,
        limits: Limits,
        cancellation: &CancellationToken,
    ) -> LocalResult<Response> {
        if prompt.len() > MAX_TEXT_PROMPT_BYTES {
            return Err("prompt exceeds byte limit".into());
        }
        if limits.output_tokens == 0 || limits.output_bytes == 0 || limits.timeout_ms == 0 {
            return Err("limits must be positive".into());
        }
        if limits.context.get() > 32768 {
            return Err("local context exceeds 32768 tokens".into());
        }
        let started = Instant::now();
        let deadline = Duration::from_millis(limits.timeout_ms);
        let content = if image.is_some() {
            format!("{}\n{prompt}", mtmd_default_marker())
        } else {
            prompt.to_owned()
        };
        let template = self.model.chat_template(None)?;
        let mut environment = Environment::new();
        environment.set_fuel(Some(100_000));
        environment
            .set_unknown_method_callback(minijinja_contrib::pycompat::unknown_method_callback);
        environment.add_function(
            "raise_exception",
            |message: String| -> Result<String, minijinja::Error> {
                Err(minijinja::Error::new(
                    minijinja::ErrorKind::InvalidOperation,
                    message,
                ))
            },
        );
        let formatted_prompt = environment.template_from_str(template.to_str()?)?.render(context! {
            messages => [serde_json::json!({"role":"user", "content":content})],
            add_generation_prompt => true, enable_thinking => false, tools => false, add_vision_id => false,
        })?;
        let mut response = Response {
            text: String::new(),
            formatted_prompt,
            prompt_tokens: 0,
            prompt_positions: 0,
            generated_tokens: 0,
            prefill_ms: 0,
            generation_ms: 0,
            stop_reason: StopReason::OutputTokens,
        };
        if let Some(reason) = interrupted(cancellation, started, deadline) {
            response.stop_reason = reason;
            return Ok(response);
        }
        let bitmap = image
            .map(|bytes| -> LocalResult<MtmdBitmap> {
                if bytes.len() > MAX_IMAGE_BYTES {
                    return Err("image exceeds byte limit".into());
                }
                let mut reader =
                    image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format()?;
                let mut decode_limits = image::Limits::default();
                decode_limits.max_image_width = Some(8192);
                decode_limits.max_image_height = Some(8192);
                decode_limits.max_alloc = Some(256 * 1024 * 1024);
                reader.limits(decode_limits);
                let rgb = reader.decode()?.into_rgb8();
                Ok(MtmdBitmap::from_image_data(
                    rgb.width(),
                    rgb.height(),
                    rgb.as_raw(),
                )?)
            })
            .transpose()?;
        let bitmaps: Vec<_> = bitmap.iter().collect();
        let chunks = self.projector.tokenize(
            MtmdInputText {
                text: response.formatted_prompt.clone(),
                add_special: true,
                parse_special: true,
            },
            &bitmaps,
        )?;
        response.prompt_tokens = chunks.total_tokens();
        response.prompt_positions = chunks.total_positions();
        // Visual positions can differ from token count; reserve the larger footprint.
        let footprint = response
            .prompt_tokens
            .max(usize::try_from(response.prompt_positions)?);
        if footprint
            .checked_add(limits.output_tokens as usize)
            .is_none_or(|total| total > limits.context.get() as usize)
        {
            return Err(format!(
                "context budget exceeded: input {footprint} + output {} > {}",
                limits.output_tokens, limits.context
            )
            .into());
        }
        if let Some(reason) = interrupted(cancellation, started, deadline) {
            response.stop_reason = reason;
            return Ok(response);
        }
        let params = LlamaContextParams::default()
            .with_n_ctx(Some(limits.context))
            .with_n_batch(512)
            .with_n_ubatch(512)
            .with_n_threads(4)
            .with_n_threads_batch(4);
        let mut ctx = self.model.new_context(&self.backend, params)?;
        // Native GPU prefill is indivisible through this binding. Check cancellation
        // immediately before and after it, then between every generated token.
        let first_position = chunks.eval_chunks(&self.projector, &ctx, 0, 0, 512, true)?;
        // Reading logits waits for asynchronous prefill work before timing it.
        let _ = ctx.get_logits();
        response.prefill_ms = started.elapsed().as_millis();
        let generation = Instant::now();
        let mut sampler = LlamaSampler::chain_simple([LlamaSampler::greedy()]);
        let mut batch = LlamaBatch::new(1, 1);
        let mut decoder = encoding_rs::UTF_8.new_decoder();
        for index in 0..limits.output_tokens {
            if let Some(reason) = interrupted(cancellation, started, deadline) {
                response.stop_reason = reason;
                break;
            }
            let token = sampler.sample(&ctx, -1);
            sampler.accept(token);
            if self.model.vocab().is_eog(token) {
                response.stop_reason = StopReason::EndOfGeneration;
                break;
            }
            let piece = self.model.vocab().token_to_piece(token, true, None);
            let mut text = String::with_capacity(
                decoder
                    .max_utf8_buffer_length(piece.len())
                    .ok_or("token is too large")?,
            );
            let (result, read, _) = decoder.decode_to_string(&piece, &mut text, false);
            if result != encoding_rs::CoderResult::InputEmpty || read != piece.len() {
                return Err("token UTF-8 decoding exceeded buffer".into());
            }
            if response.text.len() + text.len() > limits.output_bytes {
                response.stop_reason = StopReason::OutputBytes;
                break;
            }
            response.text.push_str(&text);
            response.generated_tokens += 1;
            batch.clear();
            batch.add(token, first_position + i32::try_from(index)?, &[0], true)?;
            ctx.decode(&mut batch)?;
        }
        response.generation_ms = generation.elapsed().as_millis();
        Ok(response)
    }
}

fn interrupted(
    cancellation: &CancellationToken,
    started: Instant,
    deadline: Duration,
) -> Option<StopReason> {
    if cancellation.is_cancelled() {
        Some(StopReason::Cancelled)
    } else if started.elapsed() >= deadline {
        Some(StopReason::Timeout)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires Qwen3.5-2B weights, matching projector, and a GPU runtime"]
    fn live_limits_cancellation_and_recovery() -> LocalResult<()> {
        let bundle = std::env::var("ISOBOARD_LOCAL_MODEL_DIR")?;
        let bundle = Path::new(&bundle);
        let mut runtime = LocalRuntime::load(
            &bundle.join("Qwen3.5-2B-Q4_K_M.gguf"),
            &bundle.join("mmproj-F16.gguf"),
        )?;
        let prompt = "Return exactly ISOBOARD 123.";
        let token = CancellationToken::new();
        let limits = Limits::default();
        assert_eq!(
            runtime
                .infer(
                    prompt,
                    None,
                    Limits {
                        output_tokens: 1,
                        ..limits
                    },
                    &token
                )?
                .stop_reason,
            StopReason::OutputTokens
        );
        assert_eq!(
            runtime
                .infer(
                    prompt,
                    None,
                    Limits {
                        output_bytes: 1,
                        ..limits
                    },
                    &token
                )?
                .stop_reason,
            StopReason::OutputBytes
        );
        assert!(runtime
            .infer(
                prompt,
                None,
                Limits {
                    context: NonZeroU32::new(16).unwrap(),
                    output_tokens: 1,
                    ..limits
                },
                &token
            )
            .is_err());
        assert_eq!(
            runtime
                .infer(
                    prompt,
                    None,
                    Limits {
                        timeout_ms: 1,
                        ..limits
                    },
                    &token
                )?
                .stop_reason,
            StopReason::Timeout
        );
        let cancelled = CancellationToken::new();
        cancelled.cancel();
        let response = runtime.infer(prompt, None, limits, &cancelled)?;
        assert_eq!(response.stop_reason, StopReason::Cancelled);
        assert_eq!(response.generated_tokens, 0);

        // Cancel a long generation, then verify that fresh contexts recover cleanly.
        let running = CancellationToken::new();
        let signal = running.clone();
        let timer = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(500));
            signal.cancel();
        });
        let response = runtime.infer(
            "List integers from 1 to 10000, one per line, with no explanation.",
            None,
            limits,
            &running,
        )?;
        timer.join().unwrap();
        assert_eq!(response.stop_reason, StopReason::Cancelled);
        assert!(
            response.generated_tokens > 0,
            "cancellation must exercise generation"
        );
        assert!(
            response.generation_ms < 1000,
            "cancellation must stop promptly"
        );
        let restored = runtime.infer(prompt, None, limits, &token)?;
        assert_eq!(restored.stop_reason, StopReason::EndOfGeneration);
        assert_eq!(restored.text, "ISOBOARD 123");
        let image = include_bytes!("../../fixtures/local-probe.png");
        let restored_image = runtime.infer(
            LocalRuntime::conversion_prompt(),
            Some(image),
            limits,
            &token,
        )?;
        assert_eq!(restored_image.stop_reason, StopReason::EndOfGeneration);
        assert!(restored_image.text.contains("ISOBOARD 123"));
        assert!(restored_image.text.contains("Apples"));
        eprintln!(
            "Cancelled after {} tokens; recovery text and image passed",
            response.generated_tokens
        );
        Ok(())
    }
}
