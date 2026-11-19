---
title: Inference and prompts
description: How isoboard picks providers and models per Profile, renders the conversion prompt, runs the Additional Rules pass and caches the models.dev catalog.
sidebar:
  label: Inference
  order: 5
---

Each capture freezes a Profile's conversion flags, Additional Rules, stage assignments, and prompt templates, then sends one image request (or an image request plus a text request) to a fixed provider. Provider-neutral settings live in the Profile, credentials and endpoints live in shared provider records, and every prompt comes from a bundled Jinja template. The conversion template is the source of truth for the HTML the model must return.

This page covers providers, Profiles, prompts, the split-stage pass, the model catalog, and the API probe. For the local runtime see [Local models](../local-models/). For the HTML the renderer accepts see [HTML output](../html-output/). For credential storage see [Secrets and Keychain](../secrets/).

## Providers

A model configuration names a fixed provider. The set is closed (`InferenceBackend` in [`inference_profiles.rs`](https://github.com/carlesonielfa/isoboard-app/blob/main/src-tauri/src/inference_profiles.rs)).

| TOML value | Integration | Credential | Endpoint |
|---|---|---|---|
| `local` | Embedded llama.cpp | None | None |
| `openai_api` | `genai` crate | API key | Adapter default |
| `anthropic_api` | `genai` crate | API key | Adapter default |
| `gemini_api` | `genai` crate | API key | Adapter default |
| `openai_compatible` | `genai` crate, OpenAI wire protocol | API key | Required, `https`, or `http` on loopback only |

Provider authentication and endpoints stay in shared, backend-owned records under `[inference.providers.<backend>]`. A record holds an optional endpoint and an opaque `credential_ref`, never a secret. Profiles never copy credentials or endpoints. Only `openai_compatible` accepts a custom endpoint. The endpoint validator in [`inference/target.rs`](https://github.com/carlesonielfa/isoboard-app/blob/main/src-tauri/src/inference/target.rs) rejects credentials in the URL, query strings, fragments, and plain `http` hosts other than `localhost` and loopback IPs. Redirects are disabled on the shared HTTP client.

The key is optional only for `openai_compatible` (`InferenceBackend::credential_optional`, exposed to the frontend as `credentialOptional`). Saving that provider with a blank key stores the endpoint with `credential_ref = None` and writes nothing to the Keychain. A profile with an endpoint and no reference is capture-ready (`FrozenInference::keyless`). Without a key, `api_target` overrides genai's auth so the request carries no `Authorization` header. OpenAI, Anthropic and Gemini still require a key.

The Local provider is available automatically in builds with llama.cpp support and needs no credential. Missing local readiness is a download state, not an authentication state. Capture never silently falls back from Local to an external provider.

### Request path

`InferenceService` implements the `ProductInference` boundary with two operations: `extract` (prompt plus image) and `process_text` (prompt only). It resolves a frozen stage chain, reads the credential on first use, and dispatches by provider. Shared limits live in [`inference/runner.rs`](https://github.com/carlesonielfa/isoboard-app/blob/main/src-tauri/src/inference/runner.rs):

| Limit | Value |
|---|---|
| Image input | 20 MiB |
| Accumulated decoded output | 1 MiB |
| Text prompt | 1 MiB plus 128 KiB |
| Model ID | 256 bytes |
| API key | 16 KiB, UTF-8, no control characters |
| Endpoint | 2 KiB |
| Default request timeout | 120 seconds, covering the whole response stream |

The runner builds the request, enforces cancellation, timeouts, and stream completion, rejects tool-call outputs (no tools are ever sent), and normalizes errors so keys and responses are not printed. Provider-specific wire work is delegated to `genai`.

## Processing Profiles and stage assignment

Profiles own their model settings. There are no standalone inference-profile identities, references, or deletion guards. See [Product specification](../spec/#processing-profile-contract) for the full schema. Key points:

- A Profile has a primary `model_settings` and an optional remembered `rules_model_settings`. Each holds a provider, a model ID, and optional `options`.
- `rules_model_mode` is `same` (default) or `independent`. Same mode reuses the complete primary configuration. Independent mode needs its own configured model and never falls back.
- Switching back to same mode keeps one independent configuration for later reuse, including across restarts. This is not a per-model history.
- Inactive settings are validated structurally but never affect capture readiness or credential access.
- Duplicating a Profile copies both configurations independently. Credentials and downloaded models stay shared.

```toml
[processing.profiles.default]
name = "Default"
additional_rules = ""
additional_rules_separate_pass = false
rules_model_mode = "same"

[processing.profiles.default.conversion]
tables = true
colors = true
images = true

[processing.profiles.default.model_settings]
provider = "local"
model = "isocr1-0.8b"

[inference]
provisioned_providers = []

[inference.providers.openai_api]
credential_ref = "<opaque-secret-ref>"
```

### Inference options

Options are optional and live in a nested `options` table. An omitted option is not serialized and lets the provider or runtime default apply. `reasoning = "none"` is distinct from omission.

| Option | Valid values |
|---|---|
| `temperature` | Finite number, 0.0 to 2.0 |
| `max_output_tokens` | Positive integer |
| `request_timeout_seconds` | 1 to 3600. Omitted means 120 |
| `reasoning` | `none`, `minimal`, `low`, `medium`, `high`, `xhigh`, `max`, `on`, `off` |

The limits come from [`config/application.jsonc`](https://github.com/carlesonielfa/isoboard-app/blob/main/config/application.jsonc) and are shared by Rust validation and the frontend. Reasoning is provider and model specific. The adapter maps supported values, and unsupported explicit values fail with an actionable error instead of being dropped. The generic adapter does not implement `on` and `off`, so those fail too. Local inference accepts only `temperature = 0` and `reasoning` of `none` or `off`.

### Built-in Profiles

Connecting a provider creates its Efficient and Accurate starting Profiles once. They are ordinary editable and deletable Profiles with concrete model IDs in TOML. `[inference].provisioned_providers` records which providers have been seeded, so startup, reload, reconnect, and readiness refresh never overwrite edits or recreate deleted Profiles. Removing a credential does not delete Profiles. `openai_compatible` and `local` have no bundled choices. App updates do not rewrite model settings in existing Profiles. The seed models are defined in [`config/model-defaults.jsonc`](https://github.com/carlesonielfa/isoboard-app/blob/main/config/model-defaults.jsonc). The tiers describe a relative speed and capability tradeoff, and OCR quality has not been benchmarked across them.

### Freezing and readiness

Capture freezes the conversion flags, effective application mode, Additional Rules, both stage assignments, both prompt template sources, and the separate-pass flag at run start. Missing active settings block execution and never select a fallback. With a separate pass, both stage chains and credential references are checked before the first transport. Readiness is matched by provider and the exact current credential reference.

## Conversion prompt

The prompt is a bundled Jinja template, rendered by `minijinja` in [`inference/params.rs`](https://github.com/carlesonielfa/isoboard-app/blob/main/src-tauri/src/inference/params.rs). The three bundled sources are:

| File | Used for |
|---|---|
| [`conversion.jinja`](https://github.com/carlesonielfa/isoboard-app/blob/main/src-tauri/src/inference/conversion.jinja) | First-pass image conversion for generic models |
| [`postprocessing.jinja`](https://github.com/carlesonielfa/isoboard-app/blob/main/src-tauri/src/inference/postprocessing.jinja) | Text-only second pass that applies Additional Rules |
| [`isocr.jinja`](https://github.com/carlesonielfa/isoboard-app/blob/main/src-tauri/src/inference/isocr.jinja) | Fixed extraction prompt for isOCR catalog entries |

### Ownership rules

- `conversion.jinja` is authoritative. `params.rs` is its rendering adapter.
- All trusted first-pass wording and conditionals, including the Additional Rules section, belong in the template. Rust supplies typed context and owns rendering, caching, readiness, and transport, not instruction text.
- Rendering, sanitizing, and clipboard code must implement what the template requests: styles, table structure, the `<html><body>` wrapper, and image markers. Never narrow the prompt to fit a renderer.
- A prompt edit is a deliberate product-contract change.
- Future stage prompts must also be authoritative bundled templates.

### Template context

`conversion.jinja` receives four values.

| Variable | Type | Effect |
|---|---|---|
| `tables` | bool | Keep table structure, or flatten rows into lines separated by `<br>` |
| `colors` | bool | Allow color styling, or omit color-bearing styles |
| `images` | bool | Emit `<img data-x1 data-y1 data-x2 data-y2>` markers, or omit image regions |
| `additional_rules` | string | Literal rules for the Additional Rules section, empty to omit the section |

`postprocessing.jinja` receives `additional_rules` and `intermediate_html`. With all three flags on and no rules, the rendered conversion prompt is the default prompt, and it is exposed byte for byte to the Profile editor.

Rendering runs with strict undefined behavior and a fuel limit of 100,000 steps. Output is capped at the text prompt limit, and a template over 64 KiB (`promptTemplateBytes`) or with control characters other than LF, CR, and TAB is rejected.

### Application mode

The **Conversion settings application** option (Advanced mode) decides where the three flags apply. The persisted `application_mode_override` is `prompt`, `html_cleanup`, or `prompt_and_html_cleanup`. Omitting it means automatic, which is currently `prompt_and_html_cleanup` for every backend.

| Mode | Prompt clauses | Deterministic cleanup after the model call |
|---|---|---|
| `prompt` | Selected flags render into the template | No |
| `html_cleanup` | Default flags (everything on) | Selected flags apply |
| `prompt_and_html_cleanup` | Selected flags render into the template | Selected flags apply |

The mode never applies to Additional Rules. An explicit override survives model changes.

### Prompt template overrides

A Profile may store complete replacement sources in `conversion_prompt_template` and `postprocessing_prompt_template`. The editor shows them in Advanced mode only, prefilled from the bundled defaults, which the backend exposes once as `promptTemplateDefaults` in the config snapshot. The postprocessing field appears only while the separate pass is on.

- An unchanged default is saved as `null`, so the bundled template stays authoritative. Any whitespace change is a real override.
- An edited empty source is invalid and leaves the last valid Profile unchanged.
- Non-null sources are checked as Jinja at save time and rendered with the frozen typed context at run time.
- A source replaces the corresponding bundled template in full. It is never appended.

### Additional Rules

Additional Rules are literal context data. The template interpolates the string, and it is never parsed or evaluated as Jinja. Whitespace, template-like syntax, HTML, and quotes pass through unchanged, including when a custom template is active. The field is limited to 64 KiB of UTF-8 (`additionalRulesBytes`) before whitespace normalization, and control characters other than LF, CR, and TAB are rejected. Whitespace-only rules count as empty, omit the section, and leave the base prompt byte for byte unchanged.

The template states that rules cannot override the HTML output requirements, the image-marker contract, or the conversion feature policies.

## Split-stage pass

With meaningful Additional Rules and `additional_rules_separate_pass = true`, a capture makes two calls.

1. The image call uses the conversion template without the rules.
2. A text-only call renders `postprocessing.jinja` with the literal rules and the sanitized intermediate HTML. It applies only the rules and preserves everything else. Feature cleanup stays deterministic app logic after the last call.

Empty rules take the ordinary one-call path even when the flag is on. With an OCR-only primary model (`supports_text = false` in the local catalog), meaningful rules automatically require the separate pass and an independent text-capable rules model. The editor guides the choice and never picks an external provider. With empty rules this is presentation only.

### Intermediate HTML

The second stage never receives the original response, screenshot pixels, crop base64, coordinates, or provider image attributes. During the first stage each valid crop marker is validated once and given a short private run-local reference such as `i1` in `data-isoboard-image-ref`. Crops are prepared once before the second request, even when images are disabled. The second stage may keep, remove, reorder, or repeat known references. Unknown or new references, missing or duplicate refs, coordinates, and external or data sources make the output invalid. Finalization resolves only references from the current run, applies cleanup, embeds crops, derives plain text, and publishes to the clipboard and History once. A failed, cancelled, or invalid second stage publishes neither. See [HTML output](../html-output/#split-stage-intermediate).

### Raw stage responses

History stores each exact raw response separately (`first_inference_html`, `second_inference_html`) and never renders them. Normal History shows only the final result. Debug mode exposes each stage output for copying as plain text. See [Clipboard and history](../clipboard-and-history/).

## Model catalog

The model picker reads a native cache of the public [models.dev catalog](https://models.dev/). The catalog is advisory metadata and never gates saving a Profile or validating a model ID. An arbitrary ID is valid when typed directly, with or without the catalog.

| Application provider | models.dev provider |
|---|---|
| `openai_api` | `openai` |
| `anthropic_api` | `anthropic` |
| `gemini_api` | `google` |
| `openai_compatible` | None, manual model ID only |
| `local` | None, the bundled local catalog |

### Picker behavior

- Search matches the friendly name and the opaque ID. Entries sort by release date, newest first, then last-updated date and ID.
- The suggestion filter uses text output, and vision models appear even when image input is not declared.
- The two built-in choices for a provider appear above catalog entries. They also appear when the network catalog is unavailable.
- The selected model stays visible when it falls outside the filter.
- Temperature is hidden when metadata says the model does not support it. A hidden saved value is kept. Reasoning is disabled when metadata says reasoning is unsupported or exposes only a toggle or a budget, and enabled for manual IDs. Effort choices win when both toggle and effort are declared. Unknown metadata never clears a value.
- Reasoning shows in Basic mode. Temperature, max output, and timeout show in Advanced mode.
- Blank numeric inputs leave the option unset. Invalid edits stay local with an inline error.

### Cache and refresh

[`model_catalog.rs`](https://github.com/carlesonielfa/isoboard-app/blob/main/src-tauri/src/model_catalog.rs) owns the catalog. The cache file is `models.dev.catalog.json` in the app cache directory.

1. On startup the service loads the last-known-good cache before any network work.
2. It then starts one background refresh per launch. The refresh sends `If-None-Match` and `If-Modified-Since` from the stored `ETag` and `Last-Modified`.
3. A `304` refreshes freshness metadata without replacing entries.
4. A successful body (at most 16 MiB) is hashed with SHA-256. The cached catalog is replaced only when the hash changes.
5. Network, decode, or cache-write failures keep an existing catalog as stale. With no cache, or a discarded one, the catalog stays unavailable.

The cache is disposable application data with no compatibility or migration contract. An unsupported or invalid cache is discarded and fetched again. The decoder rejects empty catalogs, unknown lifecycle statuses, and entries missing required nullable fields. The endpoint and timeout come from [`config/services.jsonc`](https://github.com/carlesonielfa/isoboard-app/blob/main/config/services.jsonc).

## API probe

The `api-probe` Cargo feature is a development-only transport check for the external providers. It is not a release default and normal capture does not depend on it. It uses the same bounded runner as capture, sends only a generated image and a fixed extraction prompt, and never reads capture files or the app TOML.

The provider set is closed (`openai`, `anthropic`, `gemini`, `openai-compatible`), the model ID is always explicit, and `openai-compatible` requires an explicit endpoint.

```sh
make api-key-set PROVIDER=openai
make api-key-status PROVIDER=openai
make api-key-logout PROVIDER=openai
make api-probe PROVIDER=openai MODEL=<explicit-provider-model-id>
make api-probe PROVIDER=openai-compatible MODEL=<explicit-provider-model-id> ENDPOINT=<https://host/v1>
```

- Keys go to four provider-specific development Keychain slots (see [Secrets and Keychain](../secrets/#keychain-layout-on-macos)). They are never accepted in argv, written to TOML, logged, or returned to the frontend.
- `api-key-set` hides terminal input. Piped input is accepted for automation, limited to 16 KiB, and validated as UTF-8 without control characters.
- The runner caps accumulated decoded output at 1 MiB. This does not cap an individual provider frame.
- A passing probe shows the transport and capability work. It does not establish OCR quality or a support claim.

## Related

- [Local models](../local-models/)
- [HTML output](../html-output/)
- [Secrets and Keychain](../secrets/)
- [Configuration](../configuration/)
- [Architecture](../architecture/)
- [Product specification](../spec/)
