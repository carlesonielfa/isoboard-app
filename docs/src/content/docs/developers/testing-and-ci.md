---
title: Testing and CI
description: "How to check an isoboard change: frontend and Rust commands, test layout, ignored live tests, the bindings drift check, CI and the contributor agreement."
sidebar:
  order: 12
---

Run `make check`, `make lint`, `make format-check`, and `make test` before you open a pull request. They cover Svelte, TypeScript, Rust, formatting, and the generated IPC bindings, and they match what CI runs. Tests are lean and aimed at product behavior. The repository rule is to add no trivial tests and no tests for one-off scripts.

## Commands

Run everything from the repository root.

| Goal | Command |
|---|---|
| All type checks and the bindings drift check | `make check` |
| All tests | `make test` |
| All lint | `make lint` |
| Verify formatting | `make format-check` |
| Apply formatting | `make format` |

After frontend changes, run `make frontend-check` and `make frontend-lint`. After Rust changes, run `make desktop-format`, then `make desktop-check` and `make desktop-lint`.

### Frontend

| Tool | Command | Notes |
|---|---|---|
| Bun test runner | `make frontend-test` or `bun run test` | Regenerates bindings first, then runs `bun test` over `tests/` |
| `svelte-check` | `make frontend-check` or `bun run check` | Runs `bun run bindings`, `svelte-kit sync`, then `svelte-check` with `tsconfig.json` |
| ESLint | `make frontend-lint` | Flat config, zero warnings allowed. It also enforces that Bits UI is imported only inside `src/lib/components/ui` |
| Prettier | `make frontend-format` and `make frontend-format-check` | Svelte 5 plugin. Covers `src`, `tests`, config files, and `config/*.jsonc` |
| Knip | `bun run check:unused` | Optional audit for unreachable files. Not part of CI |

Prettier and ESLint skip `src/lib/generated/`, because Specta owns that format.

### Rust

| Tool | Command |
|---|---|
| Tests | `make desktop-test` (`cargo test --manifest-path src-tauri/Cargo.toml`) |
| Type check | `make desktop-check` (`cargo check`) |
| Clippy | `make desktop-lint` (`cargo clippy --all-targets --all-features -- -D warnings`) |
| Formatting | `make desktop-format` and `make desktop-format-check` |

Run a subset with a path filter, for example `cargo test --manifest-path src-tauri/Cargo.toml licensing::`.

## Bindings check

The IPC contract is owned by Rust, and `src/lib/generated/bindings.ts` is committed. `make bindings-check` runs the exporter into a temporary file and compares it with the committed file. It fails with a diff when they differ. Run `make bindings` and commit the result when a command, event, serialized type, or build-time JSONC value changes. Ordinary tests never rewrite source files. See [Architecture](../architecture/).

## Test layout

### Frontend tests

`tests/*.test.ts` hold roughly two dozen focused tests. They exercise controllers and pure state modules, not rendered components. Controllers take an `environment` argument for IPC and event functions, so tests replace those at the boundary. Typical subjects are the capture controller and its status projection, configuration session behavior, History navigation, Profile and provider state, model catalog and picker state, licensing, notification and screen-capture permission handling, and the shortcut recorder.

Name a test after the behavior it covers. Prefer observable behavior over internals, and avoid mocks that replace so much of the system that the test stops exercising real code.

### Rust tests

Rust tests live next to the code in `#[cfg(test)]` modules, with a few larger suites in sibling files:

| Location | Covers |
|---|---|
| `config/controller_tests.rs`, `config/inference_transaction_tests.rs` | Transactional configuration, shortcut transitions, rollback, Profile and provider changes |
| `config/storage.rs`, `config_jsonc.rs`, `application_config.rs` | Parsing, defaults, unknown fields, JSONC rules, and validity of the build-time configuration |
| `inference/tests.rs`, `inference/params.rs` | Prompt rendering, options, and provider requests |
| `capture.rs`, `capture/backend.rs` | The capture pipeline with fake inference and clipboard, split-stage ordering, failure and cancellation paths, selection outcomes |
| `rendering/`, `clipboard/` | Sanitization, style policy, image crops, plain text, clipboard payloads. See [HTML output](../html-output/) |
| `history.rs` | SQLite store, pruning limits, and tray menu |
| `licensing/commercial_tests.rs` | Offline evaluation and license lifecycle against a mock protocol |
| `local_models/` | Catalog validity, download resume, checksum verification, and reservations |

Fixtures are in `src-tauri/fixtures/`. Use fakes at the platform boundary (`CaptureBackend`, `ClipboardWriter`, `ShortcutRuntime`, the secret store) instead of the real macOS APIs.

Native behavior still needs a manual check in the desktop app. Screen Recording permission, region selection and cancellation, global shortcuts, the system clipboard, and the native image picker cannot be validated by a browser preview or by unit tests.

## Ignored and opt-in tests

Tests marked `#[ignore]` need weights, a native GPU runtime, a network download, or credentials. `cargo test` skips them, and CI never runs them. Run one with `-- --ignored` and its environment variable.

| Test | Needs |
|---|---|
| `inference::local::tests::live_limits_cancellation_and_recovery` | Qwen3.5-2B weights and projector. Set `ISOBOARD_LOCAL_MODEL_DIR` to the bundle directory and add `--features local-probe` |
| `inference::local_worker::tests::live_setup_warm_up_reuses_model_for_capture` | A downloaded isOCR bundle. Set `ISOBOARD_LOCAL_MODELS_ROOT` |
| `capture::tests::live_local_capture_lifecycle` | Downloaded isOCR and Qwen development models. Set `ISOBOARD_LOCAL_MODELS_ROOT` |
| `local_models::service::tests::live_download_cancel_resume` | Downloads a pinned bundle. Set `ISOBOARD_DOWNLOAD_TEST_ROOT` to a fresh directory, and optionally `ISOBOARD_DOWNLOAD_TEST_MODEL` |
| `licensing::commercial_tests::production_validation_and_native_keychain_lifecycle` | Maintainer-only. See below |

The live local tests use real weights and are covered in [Local models](../local-models/).

The licensing production test calls Polar's production validation endpoint once and exercises isolated Keychain storage, offline restoration, and local removal. It reads its input from stdin, and it is a release-verification step for maintainers with an authorized key. Contributors do not need it. The offline lifecycle and protocol tests in the same module run by default and cover the behavior you are likely to change. Never commit a license key, place one in an example command, or paste one into a log or issue. See [Licensing](../licensing/).

## Continuous integration

`.github/workflows/ci.yml` runs on every pull request and on pushes to `main`. It has one job, `check`, on `macos-15` with a 60-minute timeout. Superseded runs for the same ref are cancelled. The workflow has read-only repository permissions, and third-party actions are pinned to commit hashes. It sets `CARGO_PROFILE_DEV_DEBUG=line-tables-only` to keep caches small and links fast while keeping readable panic backtraces.

Steps in order:

1. Check out the code.
2. Install Bun 1.3.6.
3. Install the stable Rust toolchain with `rustfmt` and `clippy`.
4. Restore the Cargo cache for `src-tauri`.
5. `make install`.
6. `bunx vite build`. Tauri embeds the frontend at compile time, so the build output must exist before any Cargo step. The committed bindings are enough for Vite.
7. `make format-check`.
8. `make lint`.
9. `make bindings-check`.
10. Type-check with `svelte-kit sync`, `svelte-check`, and `make desktop-check`. CI calls the tools directly because the package scripts regenerate bindings and CI already verified them.
11. Test with `bun test` and `make desktop-test`.

Reproduce a CI failure locally with the matching `make` target. Only macOS is a supported CI target today. Add Windows and Linux jobs when those platforms are validated.

## Release workflow

`.github/workflows/release.yml` builds, notarizes and drafts a GitHub release when a version tag is pushed. It runs `make release` with secrets from the `release` environment. See [Releases and notices](../releases-and-notices/#release-workflow).

## Contributor Agreement workflow

`.github/workflows/contributor-agreement.yml` runs on `pull_request_target` (opened, synchronized, closed) and on issue comments. It asks each pull request author to accept the [Contributor Agreement](https://github.com/carlesonielfa/isoboard-app/blob/main/CONTRIBUTOR-AGREEMENT.md) once, by posting a fixed comment, and records the acceptance in a private signatures repository. The check never checks out pull request code, which is why running it with `pull_request_target` is safe. Keep it that way: do not add a checkout of the pull request head or run untrusted code in this workflow.

The agreement text is fixed by a protected Git tag, `isoboard-contributor-v1`. The repository owner and bots are allowlisted. Contributors comment `recheck` to refresh the status. See [CONTRIBUTING.md](https://github.com/carlesonielfa/isoboard-app/blob/main/CONTRIBUTING.md) for the contributor view. The pull request template asks for the change, the checks you ran, and contribution rights.
