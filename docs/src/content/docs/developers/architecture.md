---
title: Architecture
description: "How isoboard is built: the Rust module map, Svelte frontend, generated IPC bindings, and the capture flow from shortcut to clipboard and History."
sidebar:
  order: 3
---

isoboard is a Tauri 2 app. A Rust process owns configuration, capture, inference, rendering, the clipboard, History, licensing, and credentials. A Svelte 5 frontend in a WebView renders settings and the capture overlay. They talk through typed Tauri commands and events, and the Rust definitions generate the TypeScript types. This page maps the modules, the IPC contract, the capture flow, and state ownership.

## Design rules

- Rust owns every serialized contract. The frontend derives its types from `src/lib/generated/bindings.ts`.
- Capture freezes its settings at the start. A running capture never sees later edits to Profiles or providers.
- The trusted Jinja template `src-tauri/src/inference/conversion.jinja` is the source of the requested HTML. Rendering and clipboard code implement its contract instead of narrowing it. See [HTML output](../html-output/).
- Platform APIs sit behind narrow traits (`CaptureBackend`, `ClipboardWriter`, `ShortcutRuntime`, secret store, window presentation). Shared orchestration depends on the trait, and the platform implementation lives in its own module.
- Secrets never enter TOML, JSONC, History, or IPC payloads. See [Secrets and Keychain](../secrets/).

## Rust module map

All modules are under `src-tauri/src/`. `lib.rs` declares them, builds the Tauri app, and wires startup. `main.rs` only calls `run()`.

### Application shell

| Module | Responsibility |
|---|---|
| `lib.rs` | Plugin registration, global shortcut handler, managed state, tray menu, window events, startup sequence, shutdown of the local worker |
| `contracts.rs` | Registers every IPC command, event, and build-time config type for Specta and exports the TypeScript bindings |
| `window_presentation/` | Reveals and hides the main window, with a macOS adapter for native effects |
| `tray_activity.rs`, `tray_menu_icons.rs`, `capture_feedback.rs` | Menu bar icon animation, template-tinted SVG menu icons, capture status in the menu |
| `capture_overlay.rs`, `capture_overlay/` | Placement, visibility, and dismissal of the transient capture overlay window |
| `launch.rs` | Decides whether the settings window opens at launch, using the last opened version |
| `notifications.rs`, `notifications/macos.rs` | Native notifications, such as a finished model download |
| `app_sounds.rs` | Capture sound cues through `rodio` |
| `resource_usage.rs` | Resident memory of the native process for Debug mode |
| `screen_capture_permission.rs`, `screen_capture_permission/macos.rs` | Screen Recording permission checks, including the out-of-process probe that `run()` handles first |

### Configuration and Profiles

| Module | Responsibility |
|---|---|
| `config/` | The runtime configuration subsystem. See [Configuration system](../configuration/) |
| `profiles.rs` | Conversion settings, application mode, and Profile types |
| `inference_profiles.rs` | Persisted provider, `InferenceOptions`, and `ReasoningLevel` types |
| `builtin_inference_profiles.rs` | Starting Profiles provisioned once per provider |
| `application_config.rs`, `product_config.rs`, `config_jsonc.rs` | Build-time JSONC schemas and the shared parser |
| `model_catalog.rs` | Cached models.dev metadata, advisory only |

### Capture and conversion

| Module | Responsibility |
|---|---|
| `capture.rs`, `capture/backend.rs` | `CaptureCoordinator`, `CaptureStatus` events, image validation, the capture pipeline, and the platform `CaptureBackend` (macOS runs `/usr/sbin/screencapture`) |
| `inference/` | `InferenceService`, the `ProductInference` trait, prompt rendering (`params.rs`), the provider runner, local runtime, and the three bundled prompt templates. See [Inference](../inference/) |
| `local_models/` | Model catalog, downloads, the store, and runtime state. See [Local models](../local-models/) |
| `rendering/` | Sanitization, style policy, image crops, plain-text derivation, and the split-stage helpers. See [HTML output](../html-output/) |
| `clipboard/` | `ClipboardPayload`, the `ClipboardWriter` trait, and the `arboard` implementation. See [Clipboard and history](../clipboard-and-history/) |
| `history.rs`, `history/` | SQLite store, pending captures, tray History menu, and History IPC |

### Licensing and secrets

| Module | Responsibility |
|---|---|
| `licensing.rs`, `licensing/` | Evaluation period, commercial license cache, Polar validation. See [Licensing](../licensing/) |
| `secret_store.rs`, `secret_store/macos.rs` | Closed set of secret slots and the Keychain adapter |
| `managed_credentials.rs` | Per-installation credential references and lifecycle |
| `api_credentials.rs` | Probe-only API-key slots, compiled with the `api-probe` feature |

### Cargo features

| Feature | Effect |
|---|---|
| `local-inference` (default) | Embeds llama.cpp and enables `inference::local`, `local_worker`, and the local provider |
| `api-probe` | Enables `api_credentials` and the `api_probe` and `api_credentials` examples |
| `local-probe` | Enables the `local_probe` example |
| `probe-runtime` | Shared base for the probe features |

## Frontend structure

The frontend is a SvelteKit app in SPA mode (`ssr = false`, static adapter). Tauri loads it from `../build` in release builds and from `http://localhost:1420` in development.

| Path | Contents |
|---|---|
| `src/routes/+page.svelte` | The settings window. Composes the shell, tabs (History, Shortcuts, Profiles, Providers, Settings), setup guide, and footer |
| `src/routes/overlay/+page.svelte` | The capture overlay window, labelled `capture-overlay` in `tauri.conf.json` |
| `src/lib/components/` | App-facing components. Every displayed element is a component here |
| `src/lib/components/ui/` | Generated and headless primitives (Bits UI, shadcn-svelte). Only this folder imports Bits UI |
| `src/lib/*.svelte.ts` | Controllers that hold reactive state with Svelte 5 runes and call commands. Examples: `capture-controller`, `config-session`, `history-controller`, `licensing`, `local-models-controller`, `model-catalog-controller` |
| `src/lib/*-state.ts`, `*-presentation.ts` | Pure projections and helpers, tested without a DOM |
| `src/lib/ipc.ts` | Re-exports `commands` and `events` from the generated bindings. Import IPC from here |
| `src/lib/product-config.ts`, `application-config.ts` | Re-export build-time config values that Rust generates into the bindings |
| `src/lib/generated/bindings.ts` | Committed generated output. Excluded from Prettier and ESLint |

Controllers take an `environment` argument for their IPC and event functions, so tests substitute them at the boundary. Components render state and forward user actions. They do not call IPC directly for domain logic. The [UI](../ui/) page covers tokens and the component inventory.

## IPC commands and generated bindings

`contracts::builder()` returns a `tauri_specta::Builder` that lists every command with `collect_commands!` and every event with `collect_events!`. The running app uses it for the Tauri invoke handler and event mounting. The exporter uses the same builder to write TypeScript, so the two cannot drift.

Command errors use `ErrorHandlingMode::Throw`. A failing command rejects its promise, and controllers handle it through the app error store. Integer types that cross IPC use plain JSON numbers (`dangerously_cast_bigints_to_number`).

The generated file also exports the build-time configuration values `PRODUCT_CONFIG`, `APPLICATION_CONFIG`, and `MODEL_DEFAULTS`, plus `DEFAULT_INFERENCE_BACKEND_METADATA` as fallback provider metadata. `services.jsonc` is exported as a type only, because the frontend does not use those values. See [Configuration system](../configuration/).

### Add or change a command or event

1. Define the function or payload type in the owning Rust module with `#[tauri::command]` and `#[specta::specta]`, or derive `specta::Type` on the payload.
2. Register it in `contracts.rs` under `collect_commands!` or `collect_events!`.
3. Run `make bindings`. This runs `cargo run --example export_bindings -- src/lib/generated/bindings.ts`.
4. Use it from the frontend through `$lib/ipc`.
5. Commit the regenerated `bindings.ts` with the Rust change.

`make bindings-check` regenerates to a temporary file and fails when the committed file differs. `make check` and CI run it, and the `bun run dev`, `build`, `check`, and `test` scripts regenerate bindings first. See [Testing and CI](../testing-and-ci/).

Events in the contract: `ConfigSnapshot`, `ConfigError`, `CaptureStatus`, `CaptureOverlayPresentation`, `LocalModelsChangedEvent`, `ModelCatalogSnapshot`, `LicenseChangedEvent`, `HistoryChangedEvent`, `HistoryErrorEvent`, and `OpenHistoryEvent`. Commands group by area: licensing, capture, local models, configuration and Profiles, provider credentials, model catalog, and History. `contracts.rs` is the authoritative list.

## Capture flow

A capture runs from a trigger to a clipboard write and a History entry. The numbers match the steps below.

```text
 trigger                  freeze              blocking thread        async task (Tokio)
 -------                  ------              ---------------        ------------------
 global shortcut  ─┐
 menu bar action  ─┼─> CaptureContext ─> screencapture -i ─> process_capture
 Open image(s)    ─┘   (frozen config)    (region only)           │
                                                                  ├─ validate image, add pending History entry
                                                                  ├─ inference: image -> HTML (stage 1)
                                                                  ├─ [rules + separate pass] text -> HTML (stage 2)
                                                                  ├─ rendering: sanitize, crops, plain text
                                                                  ├─ clipboard write (HTML + plain text)
                                                                  └─ History insert, tray refresh, final status
```

1. **Trigger.** The global shortcut handler in `lib.rs`, the **Capture region** menu item, or the `start_region_capture_command` and `open_image_command` IPC commands start a capture. The handler resolves the shortcut ID to a capture binding and its Profile.
2. **Freeze.** `config.resolve_capture_context` builds a `CaptureContext` holding the Profile's conversion flags, application mode, Additional Rules, both stage model assignments (`FrozenInference`), prompt templates, and the separate-pass flag. Frozen assignments carry only an opaque credential reference.
3. **Readiness and begin.** `CaptureCoordinator::begin_with_context` rejects the capture with `Busy` when one is already running. Setup readiness and credential references are checked before any transport call. Capture emits `CaptureStatus::Started`.
4. **Acquire the image.** For a region capture, a `std::thread` runs the blocking `CaptureBackend`, which shows the system region picker. A cancelled selection ends quietly and a denied permission fails with `PermissionDenied`. For opened files, which the backend supports but no UI entry point starts yet, the file dialog and decoder produce the bytes. Images are validated against size and pixel limits (20 MiB, 100 megapixels, 10,000 px per side, and a minimum region size).
5. **Process.** `process_capture` runs on the Tauri async runtime. It adds a pending History entry (with a generated thumbnail, and the original capture in memory when originals are kept) and publishes `ImageReady`, then progress states (`DownloadingModel`, `LoadingModel`, `Processing`).
6. **Inference.** `InferenceService` sends the image and the rendered conversion prompt to the frozen provider and returns raw HTML. With meaningful Additional Rules and the separate pass on, `rendering::prepare_conversion` first produces safe intermediate HTML, and a text-only call with `postprocessing.jinja` returns the transformed HTML. See [Inference](../inference/).
7. **Render and publish.** `rendering` sanitizes the HTML, embeds only app-generated image crops, and derives plain text. `clipboard` writes one sanitized HTML fragment plus the plain-text fallback. See [HTML output](../html-output/).
8. **Persist.** Only after the clipboard write succeeds does the capture build a `HistoryEntryInput` and insert it. A history write failure emits `HistoryErrorEvent` and does not change the clipboard result. A failed capture persists a failure entry, and `Cancelled` discards the pending entry.
9. **Finish.** The coordinator publishes `Completed`, `Failed`, or `Cancelled`. The frontend overlay and settings footer render the status, and the tray menu refreshes.

## Threading and state ownership

| State | Type and owner | Access |
|---|---|---|
| Configuration | `SharedConfig` = `Arc<Mutex<ConfigController>>` | Commands, the shortcut handler, and the watcher thread lock it briefly. Long work copies a snapshot first |
| Capture status | `CaptureCoordinator`, with an atomic in-flight flag, a sequence counter, and a locked last status | Cloned into capture tasks. Publishing emits `CaptureStatus` |
| Inference | `Arc<InferenceService>` | Shared by all captures. Holds the credential store and, when enabled, the local worker |
| Local runtime | `LocalWorker` on one dedicated native thread | Requests arrive on a channel and are serialized. Idle models unload |
| Model files | `LocalModels` and `ModelStore` | Publishes `LocalModelsChangedEvent` snapshots |
| History | `Arc<dyn HistoryStore>` (SQLite behind a connection lock) and `PendingHistoryState` in memory | Written after clipboard publication |
| Clipboard | `Arc<dyn ClipboardWriter>` | Used from the async task |
| Licensing | `CommercialService` (`Arc`) plus a mutexed `LicenseStore` | A background thread revalidates when due and skips while a capture is busy |
| Credentials | `ManagedCredentialStore` in `ManagedCredentialsState` | Opened at startup from the config path |
| Model catalog | `Arc<ModelCatalogService>` | Loads its cache at startup, refreshes in the background |

`lib.rs` registers these with `.manage(...)` during `setup`. The setup order matters: configuration first, then History and licensing (both resolve files beside `config.toml`), then inference and local models, the clipboard writer, the model catalog, and finally the tray.

Threads in use: the Tauri main thread for the event loop and menu, the Tokio runtime for processing and network calls, one native thread per blocking capture, one local-inference thread, one configuration watcher thread, and licensing and catalog background tasks.

## Where configuration and secrets plug in

- **Runtime configuration.** `config/` loads `config.toml`, validates it, registers shortcuts, and emits `ConfigSnapshot` events. Captures read it only through `resolve_capture_context`. See [Configuration system](../configuration/).
- **Build-time configuration.** `config/*.jsonc` is embedded with `include_str!`, parsed once into `LazyLock` statics, and exported to the frontend through the bindings.
- **Secrets.** Profiles and `config.toml` hold only fixed provider identifiers, endpoints, and opaque credential references. `inference::InferenceService` resolves a reference to secret bytes inside the native process at call time, through `managed_credentials` and `secret_store`. The frontend can write a credential and read its readiness, never read its value.

## Tauri capabilities and window configuration

`src-tauri/capabilities/` holds three permission files: `default.json` for the main window, `capture-overlay.json` for the overlay, and `macos-window.json`, which lets the main window be dragged by its overlay title bar on macOS. Document and resource paths in `config/product.jsonc` need matching entries when they change. `tauri.conf.json` declares the two windows, the macOS minimum version (14.0), the private API flag, and the `gen/legal/` resource mapping that `build.rs` stages from `src-tauri/legal/`.
