---
title: Developer documentation
description: Contributor guide to isoboard, a source-available Tauri 2 app (Rust, Svelte 5, Bun, embedded llama.cpp) that turns screenshots into pasteable rich content.
sidebar:
  label: Overview
  order: 1
---

isoboard is a Tauri 2 desktop app with a Rust backend, a Svelte 5 and TypeScript frontend, and an embedded llama.cpp runtime for local models. It captures a screenshot, converts it to HTML with a local model or an AI provider, and puts the result on the clipboard as rich content. It runs on macOS 14 or later today. These pages cover how the code is organized and how to change it.

<!-- os:macos -->
Windows and Linux are planned. Shared orchestration, validation, and event sequencing stay independent of operating-system APIs, and platform behavior sits behind narrow interfaces. Today the capture, window, permission, notification, and Keychain adapters are implemented for macOS only. The clipboard writer shares the same boundary, but its Windows and Linux behavior is not validated.

## Tech stack

| Layer | Technology |
|---|---|
| Desktop shell | Tauri 2 (`tauri ~2.11`), tray icon, global shortcuts, dialog, opener, and autostart plugins |
| Backend | Rust (edition 2021), Tokio, `reqwest`, `genai` for provider transport, `rusqlite` (bundled SQLite) for History, `minijinja` for prompt templates |
| Local inference | `llama-cpp-2` (llama.cpp with multimodal `mtmd`), enabled by the default `local-inference` Cargo feature |
| IPC | `tauri-specta` and Specta generate typed TypeScript bindings from the Rust definitions |
| Frontend | Svelte 5, SvelteKit (static adapter, SPA mode), TypeScript, Vite, Tailwind CSS 4, Bits UI and shadcn-svelte primitives |
| Tooling | Bun for dependencies and scripts, Prettier, ESLint, `svelte-check`, Knip, `cargo fmt`, `cargo clippy` |

## Repository layout

Paths are relative to the root of the [isoboard-app repository](https://github.com/carlesonielfa/isoboard-app).

| Path | Contents |
|---|---|
| `src/` | Svelte 5 interface. `routes/` has the settings window page and the capture overlay page, `lib/components/` the UI components, `lib/*.svelte.ts` the controllers, `lib/generated/bindings.ts` the generated IPC bindings |
| `src-tauri/src/` | Rust application. See [Architecture](./architecture/) for the module map |
| `src-tauri/examples/` | Bindings exporter and the opt-in probe tools |
| `src-tauri/capabilities/` | Tauri permission scopes per window |
| `src-tauri/legal/` | Bundled legal documents, notice manifests, and MPL source archives |
| `src-tauri/tauri.conf.json` | Window, bundle, and resource settings |
| `config/` | Build-time JSONC: product, application, services, model defaults. See [Configuration](./configuration/) |
| `tests/` | Frontend tests run by `bun test` |
| `scripts/` | Dev-bundle runner, local-model catalog helper, icon and sound generators |
| `docs/` | This documentation site and the product specification |
| `.github/workflows/` | CI and the contributor-agreement check |
| `AGENTS.md` | Conventions for coding agents and contributors |
| `Makefile` | Every development command. See [Development setup](./setup/) |

## Where to start

| You want to change | Start here |
|---|---|
| The conversion prompt or what the model is asked to produce | `src-tauri/src/inference/conversion.jinja` and `params.rs`, then [Inference](./inference/) and [HTML output](./html-output/) |
| How model HTML is sanitized, styled, or turned into plain text | `src-tauri/src/rendering/`, then [HTML output](./html-output/) |
| What lands on the clipboard, or what History stores | `src-tauri/src/clipboard/` and `src-tauri/src/history.rs`, then [Clipboard and history](./clipboard-and-history/) |
| A provider, a model option, or a Profile field | `src-tauri/src/inference_profiles.rs`, `profiles.rs`, `config/`, then [Inference](./inference/) |
| Local models, downloads, or the model catalog | `src-tauri/src/local_models/`, then [Local models](./local-models/) |
| Licensing, evaluation, or Polar validation | `src-tauri/src/licensing/`, `config/product.jsonc`, then [Licensing](./licensing/) |
| Where a credential is stored | `src-tauri/src/secret_store.rs` and `managed_credentials.rs`, then [Secrets and Keychain](./secrets/) |
| A settings screen or a shared control | `src/lib/components/`, then [UI](./ui/) |
| A Tauri command or event | Add it in Rust, register it in `src-tauri/src/contracts.rs`, run `make bindings`. See [Architecture](./architecture/) |
| A build-time limit, URL, or default | `config/*.jsonc`. See [Configuration](./configuration/) |
| Product behavior | [Product specification](./spec/) first, then update it with the code |

## Developer pages

- [Development setup](./setup/): prerequisites, `make` targets, the macOS dev bundle, probes.
- [Architecture](./architecture/): Rust modules, frontend, IPC bindings, the capture flow.
- [Configuration system](./configuration/): runtime `config.toml` and build-time JSONC.
- [Inference](./inference/): providers, Profiles, prompt templates, the split-stage pass.
- [Local models](./local-models/): the llama.cpp runtime, the catalog, downloads.
- [HTML output](./html-output/): the HTML contract between prompt and renderer.
- [Clipboard and history](./clipboard-and-history/): publication and the SQLite store.
- [Licensing](./licensing/): evaluation, Polar validation, the offline cache.
- [Secrets and Keychain](./secrets/): the credential layout.
- [UI](./ui/): design tokens, primitives, and components.
- [Testing and CI](./testing-and-ci/): checks, test layout, the CI workflows.
- [Releases and notices](./releases-and-notices/): versioning, signed builds, third-party notices.
- [Product specification](./spec/): the product behavior reference.

## Working conventions

- Keep the Rust definitions as the source of IPC types. Never restate a serialized type by hand in the frontend.
- Keep operating-system APIs behind narrow interfaces, with the platform implementation in its own module.
- Keep secrets out of TOML, JSONC, logs, and History. They belong in the Keychain through the secret store.
- Add tests for product behavior only. Do not add trivial tests or tests for one-off scripts.
- Update the [product specification](./spec/) and these pages when behavior, scope, or setup changes.
- Coding agents read [AGENTS.md](https://github.com/carlesonielfa/isoboard-app/blob/main/AGENTS.md) before working in the repository.

## Documentation site

These pages are an [Astro Starlight](https://starlight.astro.build) site in `docs/`, published at docs.isoboard.app on Cloudflare as static assets. Run commands from `docs/`:

```bash
bun install --frozen-lockfile
bun run dev
```

`bun run build` writes the site to `docs/dist/` and fails on broken internal links.

## License and contributing

isoboard uses a custom source-available license, not an OSI-approved open-source license. You can read, build, modify, and test the code, including as paid development work. Personal use is free. Productive commercial use needs a commercial license after the evaluation period. The [LICENSE.md](https://github.com/carlesonielfa/isoboard-app/blob/main/LICENSE.md) file holds the binding terms.

You do not need to submit your changes. To have a change included in official releases, open a pull request and accept the [Contributor Agreement](https://github.com/carlesonielfa/isoboard-app/blob/main/CONTRIBUTOR-AGREEMENT.md) once with a comment. You keep ownership of your work. [CONTRIBUTING.md](https://github.com/carlesonielfa/isoboard-app/blob/main/CONTRIBUTING.md) describes the pull request process, and [Testing and CI](./testing-and-ci/) describes the agreement check. Report vulnerabilities privately as described in [SECURITY.md](https://github.com/carlesonielfa/isoboard-app/blob/main/SECURITY.md).
