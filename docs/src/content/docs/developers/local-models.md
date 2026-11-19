---
title: Local models and llama.cpp
description: How isoboard runs isOCR and other local models through embedded llama.cpp, with a pinned catalog, resumable verified downloads, lazy loading, and idle unload.
sidebar:
  label: Local models
  order: 6
---

isoboard runs local models in-process through llama.cpp, managed by a bundled catalog and one dedicated worker thread. A model bundle is a GGUF model plus its vision projector, pinned by revision, size, and SHA-256. Bundles download on demand with resume, load lazily on first use, stay resident for reuse, and unload after five minutes idle.

This page covers the runtime, the catalog format, the download pipeline, lifecycle, memory, UI gating, how to add a model, and the probe. For how Profiles select a local model see [Inference and prompts](../inference/). For the user-facing explanation see the [local models guide](https://docs.isoboard.app/models/local-models/).

## Runtime

<!-- os:macos: the Metal row applies to macOS only -->
| Item | Value |
|---|---|
| Rust binding | `llama-cpp-2` and `llama-cpp-sys-2` `=0.1.158`, features `mtmd`, default features off |
| Bundled llama.cpp | `26394b4e6749a41c3633db040e0987500a5f7013` |
| Binding source revision | `3b17d1f160f0e9cfcf95948954cefc967f8f92f2` |
| Cargo feature | `local-inference`, on by default |
| Multimodal | `libmtmd` owns image preprocessing. The app never resizes images |
| Acceleration | Metal on macOS (all layers offloaded). CPU and Vulkan on other platforms are unverified |

`local_models::supported()` is true when the feature is compiled in and the target is 64-bit. Building with the feature needs CMake and a C/C++ toolchain.

The app builds a fresh request context for every request and reuses the loaded weights. Defaults for a capture are a 16,384-token context, 4,096 output tokens, four threads, 512-token batches, a 1 MiB output cap, and a 120-second timeout. A Profile's `max_output_tokens` and `request_timeout_seconds` override the last two. The chat template embedded in the GGUF is rendered with thinking disabled, and sampling is greedy, so only `temperature = 0` and `reasoning` of `none` or `off` are accepted.

Cancellation and timeout are checked around native prefill and between generated tokens. The binding cannot interrupt an in-flight GPU prefill or projector operation, and load time is outside the request timeout.

## Catalog

[`src-tauri/src/local_models/catalog.json`](https://github.com/carlesonielfa/isoboard-app/blob/main/src-tauri/src/local_models/catalog.json) is the bundled source of truth for local model identity, capabilities, prompt presets, and download bundles. The app embeds it and never fetches it, so catalog changes ship with an app rebuild. Existing IDs and pinned revisions stay unchanged when a model is added.

```json
{
  "version": 1,
  "models": [
    {
      "id": "isocr1-0.8b",
      "name": "isOCR1 0.8B",
      "supports_text": false,
      "prompt": "isocr",
      "minimum_ui_mode": "basic",
      "download": {
        "repo": "carlesonielfa/isOCR1-0.8B-GGUF",
        "revision": "d320232acfc20b977359ebbb949ae5fd60527478",
        "license_file": "LICENSE.md",
        "files": [
          { "name": "<model>.gguf", "bytes": 0, "sha256": "<64 hex>" },
          { "name": "<projector>.gguf", "bytes": 0, "sha256": "<64 hex>" }
        ]
      }
    }
  ]
}
```

| Field | Meaning |
|---|---|
| `id` | Stable, unique, path-safe ID. Stored in a Profile's `model` |
| `name` | Display name, without the ™; UI copy that mentions isOCR carries it |
| `supports_text` | `true` for text-capable models. `false` for extraction-only models such as isOCR |
| `prompt` | `standard` uses the full conversion template. `isocr` uses the fixed isOCR prompt |
| `minimum_ui_mode` | Lowest UI mode that shows the model in the picker (`basic`, `advanced`, `debug`) |
| `download.repo`, `download.revision` | Hugging Face repository and the 40-character commit |
| `download.license_file` | Optional repository file linked before download |
| `download.files` | Exactly two files, model first and its matching projector second |

The parser (`catalog.rs`) rejects unknown fields, a version other than 1, duplicate or unsafe IDs, revisions that are not 40 hex characters, file names that are not safe `.gguf` root files, zero sizes, and SHA-256 values that are not 64 hex characters.

Current entries are isOCR1 0.8B and Qwen3.5-2B. isOCR1 is the setup default, `isocr1-0.8b` in `config/model-defaults.jsonc`. The isOCR bundle is 927,767,104 bytes. The Qwen bundle is 1,949,063,104 bytes.

## Storage and downloads

Bundles live under `models/<id>/<revision>/` beside `config.toml`.

<!-- os:macos -->
On macOS the packaged app uses `~/Library/Application Support/app.isoboard/models/`, and a development run uses `.dev/models/`.

Model paths are never written to TOML, and loose files placed there by hand are not recognized. `ModelStore::resolve` reports a bundle as present only when both files exist with the exact pinned sizes.

### Download, verify, resume

`ModelStore::download_bundle` stages into `models/<id>/.partial-<revision>/` and follows these steps for each file.

1. If a verified file is already staged, re-hash it and skip it when size and SHA-256 match. Otherwise delete it.
2. Resume a `<name>.part` file. A partial larger than the pinned size is deleted.
3. Hash the existing prefix, then request `https://huggingface.co/<repo>/resolve/<revision>/<name>` with `Accept-Encoding: identity` and, when resuming, `Range: bytes=<offset>-`. Compression is disabled on the client.
4. For `206`, require the exact `Content-Range` of `bytes <offset>-<size-1>/<size>`. For `200`, discard the prefix and restart from zero (a server that ignores Range is handled). Any other status, a malformed range, or an unexpected `Content-Length` is an error.
5. Stream to disk with an incremental hash. Writing past the pinned size is an error. Progress is reported about every 200 ms.
6. Compare the SHA-256 to the catalog. A mismatch deletes the partial file and fails. A match renames it to its final name.
7. After both files verify, publish the bundle atomically by renaming the stage directory to `models/<id>/<revision>/`. An existing bundle is moved aside first and restored if the rename fails.

Partial files survive cancellation, transport failure, and app restarts. A failure leaves no ready partial bundle, and a later capture retries. Timeouts come from [`config/services.jsonc`](https://github.com/carlesonielfa/isoboard-app/blob/main/config/services.jsonc) (30 s connect, 60 s read inactivity). The app does not download or validate model license documents.

`LocalModels` (`service.rs`) is the shared runtime state used by every Profile editor, the capture overlay, and the footer. It publishes revisioned snapshots through the `local-models-changed` event and rejects stale revisions. Download states are `NotDownloaded`, `Downloading`, `Verifying`, `Paused`, `Failed`, and `Downloaded`.

- A capture that selects a model that is not downloaded downloads it first (`ensure_downloaded`) and shows byte progress.
- Cancelling a capture stops that capture's wait. An explicit Cancel download stops the shared download.
- Remove download deletes finished and partial files. It is refused while captures, downloads, or other resource changes are active, and it unloads the model first.
- A native notification announces a successful verified completion when notification access is enabled. Cancellation, failure, and an already-downloaded bundle produce none, and a failed notification never fails the download.

## Lifecycle: lazy load, reuse, idle unload

`LocalWorker` (`inference/local_worker.rs`) owns one thread, `isoboard-local-inference`, that serializes loading, inference, switching, and unloading over a bounded command channel.

- **Lazy load.** The first request loads the model and projector. The phase is `Loading`, and the overlay and footer show Loading model without a percentage because the loader reports none.
- **Reuse.** A request for the resident model with the same assets skips loading.
- **Switch.** A request for a different model releases the resident one first. Only one bundle is resident at a time.
- **Idle unload.** After five minutes with no command the worker drops the weights, projector, and resources and enters `Sleeping`. Downloaded files stay, and the next capture wakes the model.
- **Cancel and shutdown.** Cancellation reaches the active request, and the worker unloads on app exit.

Capture status stages are Downloading model, Loading model, then Processing image. Downloaded models skip the first, and warm models also skip the second. The menu bar status uses the same stages. Local inference never falls back to an external provider and never reads the Keychain.

## Memory

| Model | Measurement |
|---|---|
| isOCR | About 1.5 GB peak memory during a capture. User docs say to plan for 2 GB of free memory, with a 928 MB download and about 1 GB of free disk |
| Qwen3.5-2B | 2.57 GB maximum resident size and 1.26 GB peak memory footprint for a 1,824 by 1,426 screenshot (3,415 input tokens, 3,751 generated, 16,384-token context). Process measurements, not a full Metal accounting |

Debug mode shows native-process resident RAM and the number of loaded local models beside the footer status. RAM is sampled every two seconds while visible (`get_app_memory_usage`). It excludes WebView helper processes and is not a GPU memory total. Downloaded but sleeping models are not counted as loaded.

## UI mode gating

Each model declares `minimum_ui_mode`. Basic models appear in all modes, Advanced models in Advanced and Debug, and Debug models only in Debug. This filters picker suggestions only. A saved selection stays valid and usable in any mode, and capture readiness ignores the filter. Today isOCR1 is Basic and Qwen is Debug.

## Add a curated model

The helper `scripts/add-local-model.py` pins a Hugging Face bundle without downloading weights. It needs Python 3 and network access.

```sh
python3 scripts/add-local-model.py OWNER/REPO \
  --id MODEL_ID --name "Display name" \
  --prompt standard --text-capable
```

| Flag | Notes |
|---|---|
| `--prompt standard\|isocr` | Required. `isocr` for isOCR checkpoints, `standard` for the full conversion template |
| `--text-capable` or `--extraction-only` | Required, mutually exclusive. Repository metadata does not establish compatibility, so choose explicitly. isOCR checkpoints use `--extraction-only` |
| `--minimum-ui-mode basic\|advanced\|debug` | Picker visibility. Defaults to `basic` |
| `--revision` | Commit, tag, or branch to resolve. Defaults to the default branch head |
| `--model-file`, `--projector-file` | Needed when the repository has several candidates. The error lists them. Both files must be at the repository root |
| `--dry-run` | Print the entry without writing |
| `--catalog PATH` | Write a separate catalog for review |

The helper resolves the revision to a commit, reads file metadata at that commit, and records sizes and LFS SHA-256 values. Missing metadata is an error. Duplicate IDs are rejected, and writes replace the catalog atomically.

After adding an entry:

1. Run `cargo test --manifest-path src-tauri/Cargo.toml local_models::catalog` to validate the catalog.
2. Load the model and run a screenshot through it with the probe below. Metadata checks do not show runtime compatibility or OCR quality.
3. Add a `model-defaults.jsonc` change only if the model becomes the setup default. It must match a catalog ID.
4. Update the user docs if the model is visible in Basic mode.

## Local probe

The opt-in `local-probe` feature runs embedded llama.cpp text and image requests without the desktop app. It adds no provider settings or managed downloads. Use it to check a new bundle.

Download the bundle with `hf` (or `uv tool run --from huggingface_hub hf`) at the pinned revision, verify the hashes, and keep the files under the ignored `.dev/` directory.

```sh
cargo run --manifest-path src-tauri/Cargo.toml --features local-probe \
  --example local_probe -- \
  .dev/local-models/<bundle>/<model>.gguf \
  .dev/local-models/<bundle>/<projector>.gguf \
  .dev/local-probe/manual --image src-tauri/fixtures/local-probe.png --repeat 2
```

Omit `--image` for a text request. The default image prompt is the app's canonical conversion template, and `--prompt FILE` supplies another. Flags are `--context`, `--tokens`, `--bytes`, `--timeout-ms`, `--cancel-after-ms`, and `--repeat`. The context is capped at 32,768 tokens, and input plus reserved output must fit. Defaults are 8,192 context tokens, 2,048 output tokens, 1 MiB output, 120 seconds, four threads, and 512-token batches.

Each request writes the raw output, the exact formatted prompt, limits, token and position counts, timings, and the stop reason to the output directory. Truncated, cancelled, or timed-out output is diagnostic data, not a success. `--repeat N` reuses weights with a fresh context each time. Ctrl+C or `--cancel-after-ms` cancels.

### Opt-in integration tests

These tests need real weights and are `#[ignore]`d.

```sh
# Token and byte limits, context rejection, timeout, cancellation, recovery
ISOBOARD_LOCAL_MODEL_DIR="$PWD/.dev/local-models/<bundle>" \
  cargo test --manifest-path src-tauri/Cargo.toml --features local-probe \
  inference::local::tests::live_limits_cancellation_and_recovery -- --ignored --nocapture

# Capture lifecycle: warm reuse, idle wake, model switching, cancellation, teardown
ISOBOARD_LOCAL_MODELS_ROOT="$PWD/.dev/models" \
  cargo test --manifest-path src-tauri/Cargo.toml \
  capture::tests::live_local_capture_lifecycle -- --ignored --nocapture

# Download, cancel, restart service, resume to a verified bundle (downloads about 1.95 GB)
ISOBOARD_DOWNLOAD_TEST_ROOT="$PWD/.dev/managed-model-verification" \
  cargo test --manifest-path src-tauri/Cargo.toml live_download_cancel_resume -- --ignored --nocapture
```

The download test defaults to Qwen3.5-2B. Set `ISOBOARD_DOWNLOAD_TEST_MODEL=isocr1-0.8b` to test the 928 MB isOCR bundle. The lifecycle test uses a clipboard boundary adapter, so it does not cover native screen selection or the system clipboard. Verify those in the desktop app. Ordinary fixture tests cover byte-range resume, a server that ignores Range, malformed ranges, checksum rejection, and partial retention. See [Testing and CI](../testing-and-ci/).

### Comparing with upstream

To check the integration against `llama-server`, build it from the pinned llama.cpp revision, with Metal on macOS, and run the same model and projector with the same context, batch sizes, and threads:

```sh
llama-server -m <model>.gguf --mmproj <projector>.gguf \
  --host 127.0.0.1 --port 18765 -c 8192 -b 512 -ub 512 \
  -t 4 -tb 4 -ngl 1000 --parallel 1 --reasoning off
```

Send the same single user message to `/apply-template` and `/v1/chat/completions` with `chat_template_kwargs.enable_thinking=false`, `temperature=0`, and `max_tokens=2048`. For images, put the image part before the text part, with a leading newline in the text part. The server substitutes a random media marker, so normalize it before comparing formatted prompts. Compare input token counts, output text, and stop reasons. The server's completion token count includes the final end token, and the Rust report counts content tokens.

A 2026-10-05 comparison on an Apple M1 Max with Qwen3.5-2B produced identical output and token counts. Packaged macOS inference and Windows or Linux inference are not verified.

## Related

- [Inference and prompts](../inference/)
- [Configuration](../configuration/)
- [Releases and notices](../releases-and-notices/)
- [Product specification](../spec/)
