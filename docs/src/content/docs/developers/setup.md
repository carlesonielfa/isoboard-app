---
title: Development setup
description: "Set up an isoboard development environment on macOS: install the toolchain, clone the repo, run the app with make, and use probes and isolated config."
sidebar:
  order: 2
---

To work on isoboard you need Bun, Rust, CMake, and the Xcode Command Line Tools. Clone the repository, run `make install`, then `make dev` to start the desktop app with frontend hot reload. `make dev` runs a signed dev bundle with an isolated configuration under `.dev/`, so it never touches your installed isoboard.

<!-- os:macos -->
Desktop development works on macOS 14 or later only. The bundle targets macOS 14 so the native llama.cpp build uses the same minimum version as the app.

## Prerequisites

| Tool | Notes |
|---|---|
| Bun | Installs dependencies and runs scripts. CI uses Bun 1.3.6 |
| Rust with Cargo | The stable toolchain with `rustfmt` and `clippy` |
| Tauri prerequisites | See the [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/) |
| Xcode or Command Line Tools | The native build calls `xcrun clang` to find the compiler runtime that llama.cpp's Metal code needs |
| CMake | Builds the embedded llama.cpp runtime. Install it with `brew install cmake` |
| A stable code-signing certificate | Optional but recommended. See [Signing and permissions](#signing-and-permissions-on-macos) |

## Clone and run

```bash
git clone https://github.com/carlesonielfa/isoboard-app.git
cd isoboard-app
make install
make dev
```

`make install` runs `bun install --frozen-lockfile`. The first `make dev` also compiles the embedded llama.cpp runtime.

If the repository is checked out as the `app/` submodule of a larger workspace, run the same commands from that directory. Do not copy its `.git` file to make a standalone checkout. Use a fresh clone instead.

Tauri starts the frontend dev server on `http://localhost:1420` automatically. A browser preview of the frontend cannot validate capture, global shortcuts, permissions, or the system clipboard. Verify native behavior in the desktop app.

## Make targets

Run `make help` for the short list. Every target is below.

### Setup and run

| Target | Purpose |
|---|---|
| `make install` | Install locked Bun dependencies |
| `make dev` | Run the desktop app with frontend hot reload (alias for `make desktop-dev`) |
| `make desktop-dev` | Run the Tauri app inside the macOS dev bundle with the isolated `.dev/config.toml` |
| `make frontend-dev` | Run the Vite frontend only |
| `make dev-reset-permissions` | Stop the dev app, reset its privacy permissions, and open the Notifications settings |

### Check, test, and lint

| Target | Purpose |
|---|---|
| `make check` | Run `frontend-check`, `desktop-check`, and `bindings-check` |
| `make frontend-check` | Regenerate bindings, then run `svelte-check` |
| `make desktop-check` | Run `cargo check` |
| `make test` | Run `frontend-test` and `desktop-test` |
| `make frontend-test` | Regenerate bindings, then run `bun test` |
| `make desktop-test` | Run `cargo test` |
| `make lint` | Run `frontend-lint` and `desktop-lint` |
| `make frontend-lint` | Run ESLint with zero warnings allowed |
| `make desktop-lint` | Run `cargo clippy --all-targets --all-features` with warnings denied |
| `make format` | Format frontend and Rust sources (`frontend-format`, `desktop-format`) |
| `make format-check` | Verify formatting without writing (`frontend-format-check`, `desktop-format-check`) |
| `make bindings` | Regenerate `src/lib/generated/bindings.ts` from the Rust contracts |
| `make bindings-check` | Export to a temporary file and fail if the committed bindings differ |

### Build and package

| Target | Purpose |
|---|---|
| `make build` | Build the frontend (`frontend-build`) |
| `make bundle` | Build the desktop bundle (`desktop-build`, which runs `bun run tauri build`) |
| `make beta APPLE_SIGNING_IDENTITY="<certificate>"` | Build a signed test app and DMG without notarization |
| `make release` | Build, notarize and verify the release app and DMG. See [Releases and notices](../releases-and-notices/) |

### Provider probes

| Target | Purpose |
|---|---|
| `make api-key-set PROVIDER=<id>` | Read one API key from stdin into a probe Keychain slot |
| `make api-key-status PROVIDER=<id>` | Report whether the slot holds a key, without revealing it |
| `make api-key-logout PROVIDER=<id>` | Remove the slot |
| `make api-probe PROVIDER=<id> MODEL=<id> [ENDPOINT=<url>]` | Send one synthetic image through an external provider |

## The macOS dev bundle

macOS shows the notification permission prompt only to a real app bundle, and it attributes Screen Recording to the process that launches the capture. A bare `bun run tauri dev` binary inherits the terminal's identity and cannot request notification access. `make dev` therefore runs the development binary inside `src-tauri/target/dev/isoboard Dev.app`. The bundle stays out of the hidden `.dev/` folder because LaunchServices does not resolve apps in hidden folders, and macOS can then never add the app to the Screen Recording list.

<!-- os:macos -->
`make desktop-dev` sets `ISOBOARD_CONFIG_PATH` to `.dev/config.toml`, installs `scripts/run-macos-dev.mjs` as the Cargo runner for both Apple targets, and passes Tauri a config override that sets the bundle identifier. After each Rust build the runner:

1. Stops any running dev app.
2. Copies the staged legal resources, the binary, and `src-tauri/Info.plist` into `src-tauri/target/dev/isoboard Dev.app`.
3. Sets the bundle identifier to `app.isoboard.dev` and the name to **isoboard Dev**.
4. Signs the bundle with `codesign`.
5. Registers the bundle with LaunchServices.
6. Opens it with `open -n -W`, redirecting output to `.dev/app-stdout.log` and `.dev/app-stderr.log`, and tails both logs into your terminal.

The runner keeps the frontend server and logs running across permission restarts. The dev app uses a bundle identifier different from release builds (`app.isoboard`), so its permissions and notification settings are separate.

### Signing and permissions on macOS

macOS ties privacy grants and Keychain authorizations to the code signature. An ad-hoc signature changes on every rebuild, so grants are lost. The runner signs with the certificate named in `APPLE_SIGNING_IDENTITY` when it is set, and otherwise with the first valid code-signing certificate in your keychain, such as an Apple Development or Developer ID certificate. To choose one, export it in your shell:

```bash
export APPLE_SIGNING_IDENTITY="<your code-signing certificate>"
make dev
```

Without a usable certificate the runner falls back to ad-hoc signing and prints a warning. Screen Recording grants and stored credential authorizations may then need to be repeated after rebuilds. Signing does not notarize the app.

### Reset permissions on macOS

To test the first-run permission prompts again:

```bash
make dev-reset-permissions
make dev
```

The reset requires a previously built dev app. It stops the running dev app, registers the bundle, and runs `tccutil reset All app.isoboard.dev`, which clears Screen Recording and the other privacy approvals for the dev identifier only. It then opens System Settings → Notifications. Notification authorization has its own control: right-click **isoboard Dev** there and choose **Reset Notifications**.

The reset keeps the dev configuration, History, credentials, and downloaded models. It never changes release-app permissions. If initial setup was already completed, use **Settings** → **Restart initial setup** in the app to see the setup guide again.

## Isolated development data

In debug builds, the configuration path is `ISOBOARD_CONFIG_PATH` when set, and `.dev/config.toml` otherwise. Release builds ignore the variable and always use the platform app-config directory.

<!-- os:macos -->
On macOS the release path is `~/Library/Application Support/app.isoboard/config.toml`.

Every other data file sits beside `config.toml`:

| File | Owner |
|---|---|
| `history.sqlite3` | Clipboard History store |
| `licensing.json` | Evaluation start date |
| `commercial-license.json` | Cached commercial license metadata (no key) |
| `last-opened-version` | Launch bookkeeping for the first-run window |
| `models/<model-id>/<revision>/` | Downloaded local model bundles |

The models.dev catalog cache is separate disposable data in the platform cache directory. The `.dev/` folder is gitignored. Deleting it resets configuration, History, licensing state, and downloads, but not Keychain items.

## Logs and diagnostics

The app writes diagnostics to standard error with an `isoboard:` prefix and has no log files of its own. In development the runner captures both streams:

- `.dev/app-stdout.log`
- `.dev/app-stderr.log`

For a release build, start the binary from a terminal to see the same output. Debug mode in the app footer shows resident memory of the native process and the number of loaded local model bundles.

## Reset pre-release configuration

The schema stays at version 1 until the first public release, with no migrations. If an old `config.toml` no longer loads, for example after a schema refactor, quit isoboard, move the file to a backup name, and restart. The app creates defaults and runs setup again. Do not delete the `models/` folder or the managed credentials to reset Profiles. See [Configuration system](../configuration/) for the rules.

## Probe tools

Probes are opt-in Cargo examples behind feature flags. Normal builds and checks never run them, and they never read the app configuration or your capture files. Each sends a generated image containing the text `ISOBOARD 123`.

### API probe

The `api-probe` feature checks one external provider with one model. Supported providers are `openai`, `anthropic`, `gemini`, and `openai-compatible`. The provider and model are always explicit and no product default is selected. The compatible provider also needs `ENDPOINT`.

<!-- os:macos: probe credentials use the macOS Keychain -->
Keys live in separate provider-specific Keychain slots. They are read from stdin, never accepted as arguments, never written to TOML, and never printed by `status`. Interactive use shows a hidden prompt. Piped stdin works for automation.

```bash
make api-key-set PROVIDER=openai
make api-key-status PROVIDER=openai
make api-probe PROVIDER=openai MODEL=<model-id>
make api-probe PROVIDER=openai-compatible MODEL=<model-id> ENDPOINT=http://localhost:11434/v1
make api-key-logout PROVIDER=openai
```

The probe prints the model output and a final `expected_text_present=true` or `false` line. It requires a stored key slot for every provider, including `openai-compatible`. Press `Ctrl-C` to cancel a request.

### Local probe

The `local-probe` feature runs embedded llama.cpp text and image inference without starting the desktop app. It has no `make` target. Download a vision-capable GGUF model and its multimodal projector, then run:

```bash
cargo run --manifest-path src-tauri/Cargo.toml --features local-probe \
  --example local_probe -- \
  <model.gguf> <mmproj.gguf> <output-dir> \
  --image src-tauri/fixtures/local-probe.png --repeat 2
```

The positional arguments are the model file, the multimodal projector, and an output directory. Omit `--image` for a text request.

| Option | Meaning |
|---|---|
| `--image <png>` | Image to send. The prompt defaults to the app's conversion prompt |
| `--prompt <file>` | Explicit prompt for a text or image request |
| `--context <n>` | Context tokens |
| `--tokens <n>` | Output token limit |
| `--bytes <n>` | Output byte limit |
| `--timeout-ms <n>` | Request timeout |
| `--cancel-after-ms <n>` | Cancel the request after a delay |
| `--repeat <n>` | Repeat 1 to 10 times, reusing weights with a fresh request context |

Each request writes the raw output, the exact formatted prompt, limits, token and position counts, timings, and the stop reason to the output directory. Truncated, cancelled, or timed-out output is diagnostic data, not a success. `Ctrl-C` cancels the request.

Put downloaded weights under the ignored `.dev/` directory. The [Local models](../local-models/) page covers the catalog, pinned revisions, and the opt-in live tests that use these files.

## Other scripts

| Script | Purpose |
|---|---|
| `bun run bindings` | Same as `make bindings` |
| `bun run check:unused` | Report unreachable frontend files with Knip. Review unused exports separately, because a re-export can keep a component reachable |
| `bun run menu-icons` | Export the tray menu icons to `src-tauri/icons/menu` |
| `bun scripts/render-capture-sounds.mjs` | Render the capture sound cues |
| `python3 scripts/add-local-model.py --help` | Add a pinned Hugging Face model to the catalog. See [Local models](../local-models/) |

## Troubleshooting

- If a bundle build reports llama.cpp `std::filesystem` availability errors after the minimum macOS version changed, clear the cached CMake configuration once with `cargo clean --manifest-path src-tauri/Cargo.toml --release -p llama-cpp-sys-2`, then rebuild.
- If `make dev` reports that the dev app is still running, quit **isoboard Dev** and retry.
- If Screen Recording keeps resetting between builds, the dev bundle is signed ad-hoc. Set `APPLE_SIGNING_IDENTITY`.
