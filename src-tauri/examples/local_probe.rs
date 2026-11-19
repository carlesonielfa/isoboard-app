//! Live probe: no desktop startup, provider configuration, or network inference.

use isoboard_lib::inference::local::{Limits, LocalResult, LocalRuntime};
use std::{path::PathBuf, time::Duration};
use tokio_util::sync::CancellationToken;

const USAGE: &str = "local_probe MODEL MMPROJ OUTPUT_DIR [--image PNG] [--prompt FILE] [--context N] [--tokens N] [--bytes N] [--timeout-ms N] [--cancel-after-ms N] [--repeat N]";

#[tokio::main]
async fn main() -> LocalResult<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() < 3 || (args.len() - 3) % 2 != 0 {
        return Err(USAGE.into());
    }
    let model = PathBuf::from(&args[0]);
    let projector = PathBuf::from(&args[1]);
    let output = PathBuf::from(&args[2]);
    let mut limits = Limits::default();
    let mut image_path = None;
    let mut prompt_path = None;
    let mut cancel_after = None;
    let mut repeat = 1_u32;
    for pair in args[3..].as_chunks::<2>().0 {
        match pair[0].as_str() {
            "--image" => image_path = Some(PathBuf::from(&pair[1])),
            "--prompt" => prompt_path = Some(PathBuf::from(&pair[1])),
            "--context" => limits.context = pair[1].parse()?,
            "--tokens" => limits.output_tokens = pair[1].parse()?,
            "--bytes" => limits.output_bytes = pair[1].parse()?,
            "--timeout-ms" => limits.timeout_ms = pair[1].parse()?,
            "--cancel-after-ms" => cancel_after = Some(pair[1].parse::<u64>()?),
            "--repeat" => repeat = pair[1].parse()?,
            _ => return Err(USAGE.into()),
        }
    }
    if !(1..=10).contains(&repeat) {
        return Err("repeat must be between 1 and 10".into());
    }
    let image = image_path.as_ref().map(std::fs::read).transpose()?;
    let prompt = if let Some(path) = prompt_path {
        std::fs::read_to_string(path)?
    } else if image.is_some() {
        LocalRuntime::conversion_prompt().to_owned()
    } else {
        "Return exactly ISOBOARD 123.".to_owned()
    };
    std::fs::create_dir_all(&output)?;
    std::fs::write(output.join("request.txt"), &prompt)?;
    let cancellation = CancellationToken::new();
    let signal = cancellation.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            signal.cancel();
        }
    });
    let mut runtime = LocalRuntime::load(&model, &projector)?;
    eprintln!("Model + projector loaded in {} ms", runtime.load_ms);
    if let Some(ms) = cancel_after {
        let timer = cancellation.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(ms)).await;
            timer.cancel();
        });
    }
    for index in 1..=repeat {
        let response = runtime.infer(&prompt, image.as_deref(), limits, &cancellation)?;
        std::fs::write(
            output.join(format!("prompt-{index}.txt")),
            &response.formatted_prompt,
        )?;
        std::fs::write(output.join(format!("result-{index}.txt")), &response.text)?;
        let report = serde_json::json!({
            "binding_version": "0.1.158", "model": model, "projector": projector,
            "image": image_path, "limits": limits, "thinking": false, "sampler": "greedy",
            "threads": 4, "batch": 512, "load_ms": runtime.load_ms, "request": response,
        });
        std::fs::write(
            output.join(format!("report-{index}.json")),
            serde_json::to_vec_pretty(&report)?,
        )?;
        println!("request {index}: {:?}, {} input tokens, {} output tokens, {} ms prefill + {} ms generation", response.stop_reason, response.prompt_tokens, response.generated_tokens, response.prefill_ms, response.generation_ms);
    }
    Ok(())
}
