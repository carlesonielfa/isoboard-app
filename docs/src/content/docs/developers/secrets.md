---
title: Secrets and Keychain
description: How isoboard stores API keys and license keys in the macOS Keychain, how items are named and scoped per installation, and when the app reads them.
sidebar:
  order: 10
---

isoboard keeps every credential in the system keychain behind a small `SecretStore` trait. TOML and the frontend only ever see an opaque reference. On macOS the adapter is the Keychain, release and development builds use separate namespaces, and each installation has its own generated identity. There is no plaintext fallback.

Credentials covered here are provider API keys and the commercial license key. For how the license key is used, see [Licensing](../licensing/). For how provider records reference credentials, see [Inference and prompts](../inference/) and [Configuration](../configuration/).

## Rules

- Raw secrets never appear in TOML, exports, snapshots, logs, or frontend readback. Credential entry is write-only through backend commands.
- A missing or locked store is a runtime readiness error, not a reason to fall back to a file.
- TOML stores a validated opaque `credential_ref` on a provider record under `[inference.providers]`. The reference identifies a managed item and contains no provider name, Keychain service, path, or secret.
- Callers never derive Keychain identities from TOML. The app resolves a reference inside its own namespace and installation scope.
- Local inference never touches the Keychain.

## Code map

| File | Responsibility |
|---|---|
| [`src-tauri/src/secret_store.rs`](https://github.com/carlesonielfa/isoboard-app/blob/main/src-tauri/src/secret_store.rs) | Platform-neutral `SecretStore` trait, closed slot and kind enums, `ManagedSecretRef`, namespace and scope types |
| [`src-tauri/src/secret_store/macos.rs`](https://github.com/carlesonielfa/isoboard-app/blob/main/src-tauri/src/secret_store/macos.rs) | macOS Keychain adapter (via the `keyring` crate) and the service and account names |
| [`src-tauri/src/managed_credentials.rs`](https://github.com/carlesonielfa/isoboard-app/blob/main/src-tauri/src/managed_credentials.rs) | Installation identity, staging, validation, session cache, readiness, deletion |
| [`src-tauri/src/api_credentials.rs`](https://github.com/carlesonielfa/isoboard-app/blob/main/src-tauri/src/api_credentials.rs) | Development-only API-key slots used by the API probe |

Non-macOS builds return `UnsupportedPlatform` from the store constructors until an adapter exists.

<!-- os:macos -->
## Keychain layout on macOS

Two families of items exist.

| Family | Service | Account | Used by |
|---|---|---|---|
| Managed credentials | `app.isoboard.managed.release` or `app.isoboard.managed.dev` | `<installation-id>:<kind>:<reference>` | The app, for API keys and the license key |
| Development probe slots | `app.isoboard.dev.api.<provider>` | `api-key` | `make api-key-set` and the API probe only |

`<provider>` is one of `openai`, `anthropic`, `gemini`, or `openai-compatible`. An experimental provider has its own probe slot, which is not covered here.

`<kind>` is `api-key` or `license-key`. The namespace is `dev` when the binary is built with debug assertions and `release` otherwise (`ManagedSecretNamespace::current()`), so a debug build never reads or overwrites packaged-app items.

## Installation identity and scope

A scope is the namespace plus an installation ID. The ID is a random UUID rendered as 32 lowercase hexadecimal characters, generated on first use and stored beside `config.toml`:

```text
.isoboard-installation-<dev|release>.id
```

- The file is created with owner-only permissions on Unix.
- Creation is atomic and no-clobber. The app writes a temporary file and hard-links it into place, so concurrent launches converge on one identity.
- A malformed existing file is an error. It is never regenerated, because replacing it would orphan every stored item.
- The identity is never read from TOML.

Because the development config lives in the repository's `.dev/` directory, a development run has its own identity and its own Keychain items. See [Setup](../setup/).

## Managed credential lifecycle

`ManagedCredentialStore` is the only entry point the app uses for credentials.

### References and validation

A `ManagedSecretRef` is exactly 32 lowercase hexadecimal characters. Secrets are validated before they reach the store:

| Kind | Limit | Content rules |
|---|---|---|
| `api-key`, `license-key` | 16 KiB | UTF-8, not blank, no control characters |

The raw store only persists bytes. Provider-specific parsing stays with the feature that owns the credential.

### Atomic save

Saving a credential and its configuration follows one order:

1. `stage` generates a fresh reference, checks that the item does not already exist, and writes the new item. Items are immutable. A collision retries with a new reference up to three times.
2. The caller commits TOML (or the license record) that references the new item.
3. If either step fails, the previously referenced item and configuration stay valid. A failed staging write deletes the new item. If that cleanup also fails, the caller receives `CleanupFailed`.

An existing shared secret is never overwritten before the configuration commit. Replacing a credential allocates a new slot.

### Removal

`delete` is called only after the configuration controller has established that no live record references the item. Removing a provider's credential in the UI detaches the reference. Deleting the Keychain item is a separate step. The license service retries failed deletions through a `pending_cleanup` list that survives restarts, and a failed cleanup never restores the removed entitlement.

## Passive checks and explicit access

Reading a Keychain item can show a system prompt, so the app separates the two.

| Operation | Reads the Keychain | Result |
|---|---|---|
| Settings and capture availability checks | No | Uses saved references and session state. A saved but unread credential reports `Unknown`, shown as configured, not verified |
| First external inference, or explicit license recheck | Yes | Value or error is cached for the process |
| Save, replace, token refresh | Writes | Updates the cache |
| Delete | Deletes | Clears the cache entry |

Readiness values are `Unknown`, `Ready`, `Missing`, `Locked`, `Corrupt`, and `Unavailable`. Access errors are remembered for status display. There are no background retries. An explicit capture can retry denied access.

## Tests

The unit tests in `managed_credentials.rs` use an in-memory store factory and cover reference format, staging, readiness sanitization, passive versus explicit access, cache coherence, and identity corruption handling. Run them with:

```sh
cargo test --manifest-path src-tauri/Cargo.toml managed_credentials
```

Real Keychain behavior is covered only by the ignored licensing test described in [Licensing](../licensing/#tests). See [Testing and CI](../testing-and-ci/) for the full suite.

## Related

- [Licensing](../licensing/)
- [Inference and prompts](../inference/)
- [Architecture](../architecture/)
