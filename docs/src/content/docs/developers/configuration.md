---
title: Configuration system
description: "How isoboard configuration works: the runtime config.toml schema, validation, last-valid behavior, file watcher, and build-time JSONC files."
sidebar:
  label: Configuration
  order: 4
---

isoboard has two configuration layers. The runtime layer is a user-editable `config.toml` that the Rust `config/` module validates, saves atomically, and watches for external edits. The build-time layer is four JSONC files under `config/` that are embedded in the binary and shared with the frontend. Neither layer holds secrets.

## Runtime configuration: `config.toml`

### Location

<!-- os:macos -->
Release builds read `~/Library/Application Support/app.isoboard/config.toml`. Debug builds read `ISOBOARD_CONFIG_PATH` when it is set and `.dev/config.toml` otherwise. See [Development setup](../setup/). Data that is not configuration sits in files beside it, such as `history.sqlite3`, `licensing.json`, and the `models/` folder.

### Schema version policy

The app is unreleased, so the schema stays at **version 1** until the first public release. `CONFIG_VERSION` in `src-tauri/src/config/types.rs` is the single constant.

- An empty file, a file with only `version = 1`, and files with omitted sections or fields load the documented defaults.
- An explicit invalid value, an unknown field, or an unsupported version is an error. Every struct uses `#[serde(deny_unknown_fields)]`.
- Do not add migration code, legacy virtual-profile handling, or format-preservation branches before the first release. A file from an older schema is moved aside and recreated. See [Reset pre-release configuration](../setup/#reset-pre-release-configuration).
- The defaults are version 1, the standard default capture binding (`default`), the `default` Profile, and empty provider and provisioning records.

The models.dev catalog cache is disposable data, not a configuration format. An unsupported cache is discarded and refreshed, with no migration. Refresh uses conditional `ETag` and `Last-Modified` requests and a body hash, and a valid current cache survives network failure.

### Top-level sections

`AppConfig` has these sections. `types.rs` is authoritative for fields.

| Section | Contents |
|---|---|
| `version` | Schema version, `1` |
| `capture` | Capture bindings (an extensible list of shortcut bindings, each referencing a Profile by ID, with one `default` binding that uses the active Profile) and the overlay position |
| `shortcuts` | The shortcut that opens the settings window. It must not collide with a capture shortcut |
| `ui` | Mode (`basic`, `advanced`, `debug`) and the sounds setting |
| `processing` | The active Profile ID and the map of Profiles. Each Profile owns its conversion flags, application mode, Additional Rules, prompt template overrides, and model settings for one or two stages |
| `inference` | Fixed provider records (optional endpoint and an opaque credential reference) and `provisioned_providers`, the provisioning ledger |

Profiles own their model settings. Provider credentials and endpoints live in the shared provider records and are never copied into Profiles. See [Inference](../inference/) for the model settings, and [Secrets and Keychain](../secrets/) for credential references.

### Validation

`AppConfig::validate` runs after parsing and after every mutation. It rejects:

- An unsupported version.
- No Profiles, or an active Profile that does not exist.
- Invalid Profile IDs, invalid or duplicate names (compared trimmed and case-insensitively), and Additional Rules above the configured byte limit or with unsupported control characters.
- Invalid capture bindings or open shortcut, and an open shortcut that matches a capture shortcut.
- Invalid provider or model settings, including out-of-range options.

Prompt template overrides are normalized and validated during parsing. Validation bounds such as name length and byte limits come from `config/application.jsonc` so the frontend and Rust agree.

### Errors and the last valid configuration

`ConfigError` carries a kind (I/O, parse, validation, conflict, or runtime), the path, a message, an optional offending value, a revision number, and the current registered bindings. The frontend receives it as the `ConfigError` event.

| Situation | Behavior |
|---|---|
| File missing at startup | Defaults are created and written |
| File invalid at startup | Defaults are used in memory, the file is left untouched, and an error is emitted. The file is not overwritten |
| Valid external edit while running | The new configuration is applied, shortcuts are re-registered, and a `ConfigSnapshot` event is emitted |
| Invalid external edit while running | The previous valid configuration stays active, and an error is emitted. Fixing the file clears the error |
| Mutation fails (for example a shortcut cannot register) | The controller rolls back to the previous state and returns the error |

The controller also applies provisioning on load. Built-in Profiles are created once per provider, and the ledger in the `inference` section stops them from reappearing after you delete them. When provisioning or model normalization changes the file, the app writes it back.

### Writes

`storage::write_config` serializes with `toml::to_string_pretty` to a temporary file named `.config.toml.<pid>.tmp` in the same directory, then renames it over the target. A failed rename removes the temporary file. Readers never see a partial file.

### File watcher

`watcher.rs` watches the config **directory** (non-recursive) rather than the file alone, so atomic replacement and editor save strategies still trigger reloads. It filters events to the config path and sends them to a worker thread. The worker waits 150 ms, drains queued events, locks the controller, and calls `reload_from_disk`. A result that equals the current state is ignored.

### Module responsibilities

| File | Responsibility |
|---|---|
| `config/types.rs` | Version-one schema, validation, snapshots, `ConfigError` |
| `config/storage.rs` | TOML parsing, version check, and atomic writes |
| `config/controller.rs` | Transactional state, Profile and binding changes, shortcut registration, rollback, reloads |
| `config/watcher.rs` | Debounced directory watching and event emission |
| `config/tauri.rs` | Native startup, path resolution, the global shortcut plugin, shortcut runtime adapter |
| `config/commands.rs`, `config/inference_commands.rs` | Tauri IPC handlers for configuration, Profiles, and provider access |
| `config/provider_auth.rs` | Provider credential helpers used by the commands |
| `config/bindings.rs` | Shortcut parsing and native shortcut IDs |
| `config/mod.rs` | Small facade used by `lib.rs` |

Keep validation and state transitions in `controller.rs` and `types.rs`. Platform or IPC integrations stay in their adapter modules. `inference_profiles.rs` owns the persisted `InferenceOptions` and `ReasoningLevel` types.

## Build-time configuration: JSONC

Four files under `config/` hold public settings that ship inside the app. Rust embeds each file with `include_str!`, and a shared parser (`config_jsonc.rs`, built on `jsonc-parser`) reads comments and trailing commas. It rejects every other JSON extension, such as unquoted keys, single quotes, hexadecimal numbers, and missing commas.

| File | Owns | Rust schema |
|---|---|---|
| `config/product.jsonc` | Support address, checkout URL, bundled legal and third-party document paths, evaluation days, support months, covered major version, license-key prefix, Polar identifiers, validation URL and timeouts, validation and polling intervals | `product_config.rs` (`PRODUCT`) |
| `config/application.jsonc` | Shared validation limits (Additional Rules and prompt template bytes, Profile name length, temperature range, output tokens, request timeouts), History retention defaults, UI and native feedback timings | `application_config.rs` (`APPLICATION`) |
| `config/services.jsonc` | Endpoints and timeouts per integration: models.dev and Hugging Face | `application_config.rs` (`SERVICES`) |
| `config/model-defaults.jsonc` | The setup local model ID, provider display labels, and the built-in Profile definitions | `application_config.rs` (`MODEL_DEFAULTS`) |

Every schema uses `deny_unknown_fields`, so a typo fails instead of being ignored.

### How the values are parsed and shared

1. Each value is a `LazyLock` static. The first access parses the embedded file and panics with `invalid <file>.jsonc` if it is invalid. The `application_config.rs` tests read each of its three files, so an invalid file fails `cargo test` instead of a user's launch.
2. `contracts::export_typescript` parses the same files and writes them into `src/lib/generated/bindings.ts` as `PRODUCT_CONFIG`, `APPLICATION_CONFIG`, and `MODEL_DEFAULTS`. `services.jsonc` stays Rust-side.
3. The frontend imports them through `$lib/product-config` and `$lib/application-config`, so both sides use the same limits and policies.
4. Edit the JSONC and rebuild. `bun run dev`, `build`, `check`, and `test` regenerate bindings first. Restart a running development session. `make bindings-check` fails when the committed bindings are stale.

Existing user data is never rewritten by a configuration change. Changing the default models does not touch saved Profiles. Changing trial days does not reset a stored evaluation start. Support deadlines come only from license validation responses.

The Polar product ID is reference metadata. Validation checks the organization and benefit that grant the license.

### What belongs here and what does not

Put a value here when it is public, shared between Rust and the frontend or between several modules, and not a user preference. Examples are limits, URLs, timeouts, intervals, retention defaults, and default model identifiers.

Never put any of these in the JSONC files, which ship in every build and appear in the committed bindings:

- API keys, license keys, or client secrets.
- An administrative Polar token. Validation uses the public customer-portal endpoint and needs none.
- User preferences. Those go in `config.toml`.
- Model file lists and hashes. Those stay in `src-tauri/src/local_models/catalog.json`.

Provider capabilities remain Rust-owned. `make bindings` generates fallback provider metadata from them.

Tauri capability scopes in `src-tauri/capabilities/` are explicit security allowlists. When you change a bundled document path in `product.jsonc`, review the capability files and the resource mapping in `tauri.conf.json` too.

## Intentionally local constants

Not every number belongs in configuration. These stay with their owning code because moving them would create indirection or risk saved data and system identity:

| Constant | Where it lives |
|---|---|
| Tauri bundle identifier, window labels, and window geometry | `src-tauri/tauri.conf.json` |
| Keychain namespaces and secret slot names | `secret_store.rs` |
| Persisted schema identifiers and the `default` capture binding ID | `config/types.rs`, `config/bindings.rs` |
| Image input limits (20 MiB, 100 megapixels, 10,000 px per side) and thumbnail limits | `capture.rs`, `history.rs` |
| Local model idle timeout (five minutes) | `inference/local_worker.rs` |
| Component sizes, breakpoints, icon strokes, and transition times, for example the 180 px default secondary height of `ResizableSplitPane.svelte` | The component |
| Shared design values such as colors and radii | Theme tokens in `src/app.css`. See [UI](../ui/) |

Changing an identifier can affect saved data, permissions, or OS identity, so treat each as a migration, not a setting.
