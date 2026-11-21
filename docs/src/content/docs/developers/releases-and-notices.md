---
title: Releases and third-party notices
description: "Build, sign, notarize and publish an isoboard release for macOS: versioning, make release, the release workflow, the DMG, third-party notices, MPL source archives and the changelog."
sidebar:
  label: Releases and notices
  order: 13
---

A release is a notarized `isoboard.app` and DMG built with `make release`, with the same version in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`. Every build bundles legal documents, third-party notices, and exact-version source archives for MPL-licensed dependencies under `Contents/Resources/legal`. This page covers the build, signing and notarization, publishing, and the notice upkeep.

## Versioning

The app version appears in three files that you update together:

| File | Field |
|---|---|
| `package.json` | `version` |
| `src-tauri/Cargo.toml` | `[package] version` |
| `src-tauri/tauri.conf.json` | `version` |

The version moves with the changes, not at release time. When a user-visible change lands and the current version already has a `v<version>` tag, bump the version in the same change: a fix bumps the patch version, and a new feature bumps the minor version. If the current version is not released yet, keep it, unless a feature lands in a pending patch version. Then change it to the next minor version, for example 1.0.1 to 1.1.0. Documentation and internal changes do not bump the version.

`src-tauri/Cargo.lock` records the crate version, so build once after a bump and commit the lock file with it. The About screen and the launch logic that opens the settings window after an update read the bundle version. The commercial license covers a major release series, which `coveredMajor` in `config/product.jsonc` sets. Change it only for a new major version, and read [Licensing](../licensing/) first.

## Build a signed release

<!-- os:macos -->
Release builds run on macOS 14 or later with the prerequisites from [Development setup](../setup/). The bundle sets `minimumSystemVersion` to 14.0.

Every public build must use the same Developer ID Application certificate. macOS ties Screen Recording permission and Keychain authorization to the code signature. A build signed by a different certificate makes users grant Screen Recording again and confirm Keychain access.

### Build a notarized release

`make release` runs `scripts/release-macos.sh`. The script:

1. Checks that the three version fields match, that `CHANGELOG.md` has a section for the version, and, when `RELEASE_TAG` is set, that the tag is `v` plus the version.
2. Checks that `APPLE_SIGNING_IDENTITY` names a Developer ID Application identity in a keychain, and that `TAURI_SIGNING_PRIVATE_KEY` is set.
3. Builds the app, DMG and update archive. Tauri signs the app with the hardened runtime, notarizes and staples it, and signs the update archive with the updater key.
4. Signs, notarizes and staples the DMG.
5. Verifies both signatures, both stapled tickets and the Gatekeeper assessment.
6. Writes the release assets to `src-tauri/target/release/bundle/release/`: the DMG, `isoboard_<version>_aarch64.app.tar.gz`, and `latest.json`, the update manifest with the archive URL, its signature and the changelog notes.

Notarization uses an App Store Connect API key or an Apple ID. Set one group:

| Method | Variables |
|---|---|
| API key (preferred) | `APPLE_API_KEY` (key ID), `APPLE_API_ISSUER` (issuer ID), `APPLE_API_KEY_PATH` (path to the `.p8` file) |
| Apple ID | `APPLE_ID`, `APPLE_PASSWORD` (an app-specific password), `APPLE_TEAM_ID` |

```bash
make release APPLE_SIGNING_IDENTITY="Developer ID Application: <name> (<team ID>)"
```

The DMG is `src-tauri/target/release/bundle/dmg/isoboard_<version>_aarch64.dmg`. The app is in `src-tauri/target/release/bundle/macos/`.

### Updater signing key

Installed apps accept an update only when its archive is signed by the updater key whose public half is `plugins.updater.pubkey` in `tauri.conf.json`. This minisign key is separate from the Apple certificate. Generate it once and keep it with a password:

```bash
bunx tauri signer generate -w ~/.tauri/isoboard-updater.key
```

Back up the private key and its password. If either is lost, installed apps can no longer be updated and users must reinstall by hand. Local `make release` reads the key from `TAURI_SIGNING_PRIVATE_KEY` (the key contents or a path) and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. `make bundle` and `make beta` do not build update archives and need no key.

`requireSignedVersion` makes the app reject an archive whose signed version differs from the version in `latest.json`, so a tampered manifest cannot pair a newer version number with an older signed archive.

### Release workflow

`.github/workflows/release.yml` runs `make release` when a `v*.*.*` tag is pushed. It runs on `macos-15` in the `release` environment, imports the certificate into a temporary keychain, and deletes the keychain and API key at the end. It uses no shared build cache. When the build passes, it creates a draft GitHub release for the tag with the release assets, and uses the version's `CHANGELOG.md` section as the notes. A version with a suffix, such as `1.1.0-rc.1`, becomes a prerelease. Review the draft and publish it by hand.

Installed apps read `releases/latest/download/latest.json` from the repository named by `updates.manifestUrl` in `config/services.jsonc`, and `make release` publishes the archive URL for the same repository. A release only reaches apps built with that URL. GitHub resolves the manifest to the newest published release that is not a prerelease. Publishing the draft therefore ships the update to every installed app with automatic checks on. Drafts and prereleases are never offered. The app offers only newer releases in its own major version.

The `release` environment needs these secrets:

| Secret | Value |
|---|---|
| `APPLE_CERTIFICATE` | The Developer ID Application certificate and private key, exported as `.p12` and base64-encoded |
| `APPLE_CERTIFICATE_PASSWORD` | The `.p12` export password |
| `APPLE_API_KEY` | App Store Connect API key ID |
| `APPLE_API_ISSUER` | App Store Connect issuer ID |
| `APPLE_API_PRIVATE_KEY` | Contents of the `.p8` API key file |
| `TAURI_SIGNING_PRIVATE_KEY` | Contents of the updater private key file |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | The updater key password |

Restrict the environment to version tags and require a reviewer in the repository settings, so only an approved tag can use the certificate.

### Signed test builds

`make beta` builds a signed app and DMG without notarization, for local testing with a stable certificate such as your Developer ID certificate:

```bash
make beta APPLE_SIGNING_IDENTITY="<certificate name>"
```

`make beta` fails when `APPLE_SIGNING_IDENTITY` is empty or `-`. It runs `bun run tauri build --bundles app,dmg`, and `beforeBuildCommand` runs `bun run build`, which regenerates bindings and builds the frontend. Do not distribute these builds.

### Unsigned local packaging

`make bundle` (`bun run tauri build`) builds every bundle target for the host without requiring a certificate. Use it for local packaging. The result is ad-hoc signed, so Keychain authorizations and privacy grants do not persist across rebuilds.

### If the llama.cpp build fails

If a build reports `std::filesystem` availability errors after the minimum macOS version changed, clear the cached CMake configuration with `cargo clean --manifest-path src-tauri/Cargo.toml --release -p llama-cpp-sys-2` and rebuild.

## macOS private API

<!-- os:macos -->
The settings window is decorated, transparent, and uses the native Popover backdrop material. Transparent windows on macOS need Tauri's private API, enabled by `"macOSPrivateApi": true` in `tauri.conf.json` and the `macos-private-api` Cargo feature on `tauri`. Apps that use private APIs cannot be distributed through the Mac App Store, so isoboard is not App Store compatible.

## Release checklist

1. Confirm that the version in the three files is the one to release, and that its `CHANGELOG.md` section lists every user-visible change since the previous release (see below). Both are updated as changes land.
2. Run `make check`, `make lint`, `make format-check`, and `make test`. See [Testing and CI](../testing-and-ci/).
3. Review the third-party notices if dependencies, assets, or targets changed (below).
4. Commit, then push a tag `v<version>` on that commit. The release workflow builds, notarizes and drafts the GitHub release. To build locally instead, run `make release`.
5. Install the DMG from the draft release. Verify that it opens without a Gatekeeper warning, and check the packaged legal files and both **Settings** → **About** actions (below).
6. Publish the draft release, and update the download on [isoboard.app](https://isoboard.app). Installed apps then find the update.
7. With the previous release installed, choose **Check for updates**, then **Restart to update**, and confirm the new version in **About**.
8. Retain the exact MPL source archives with every distributed release.

## Changelog

`CHANGELOG.md` at the repository root lists user-visible changes by version. Write entries as short, user-facing sentences, grouped under the version heading. Add each entry in the change that makes it, under the current version (see [Versioning](#versioning)). There is no `Unreleased` section, because the build rejects any heading that is not a version. The changelog starts at version 1.0.

The app renders the changelog in **Settings** → **About** → **What's new**, so its format is strict. `build.rs` parses it with `src-tauri/src/changelog/parse.rs` and fails every build, including `make release`, when it breaks these rules:

- The first line is `# Changelog`.
- Each version is a `## X.Y.Z` heading, optionally followed by ` — ` and a short label. Prerelease versions such as `1.1.0-rc.1` are allowed.
- Versions are listed newest first, without duplicates, and each has at least one item.
- Items are `- ` lines. They may use `` `code` `` and `**bold**`. No other Markdown, prose or wrapped lines.

The app bundles the changelog for the installed version and older ones. `make release` also writes the whole file into the `changelog` field of `latest.json`, so an installed app shows the notes of every newer version as soon as a check finds the update, before it downloads.

## Third-party notices and source access

**Settings** → **About** provides **Third-party notices** and **MPL source code** actions. They open the bundled text documents in the system's default text viewer.

<!-- os:macos -->
On macOS you can also right-click `isoboard.app`, choose **Show Package Contents**, and open `Contents/Resources/legal`.

`THIRD_PARTY_NOTICES.txt` at the repository root retains dependency and asset notices. The conservative list includes build dependencies, and inclusion does not assert that every component is linked. Third-party rights remain governed by their own licenses.

The legal resource directory includes the MPL 2.0 license, source-access instructions, and exact-version `.crate` source archives. These are gzip tar archives and can be extracted to a writable directory with `tar -xzf`. Source access requires neither an account nor network access. The list includes the build-time `cssparser-macros` dependency conservatively. No local modifications were made to these registry sources when the snapshot was assembled.

### How the legal folder is built

The source of the bundled legal folder is `src-tauri/legal/`, plus two root files. `src-tauri/build.rs` stages them into `src-tauri/gen/legal/` on every build, and `tauri.conf.json` maps that folder to `legal/` in the bundle's resources. The dev bundle from `make dev` includes the same resources.

- All Markdown documents, including the Spanish copies in `legal/es/`, are copied with links rewritten for the bundle layout.
- The root `LICENSE.md` is copied in as `SOURCE-LICENSE.md`.
- `THIRD_PARTY_NOTICES.txt` is copied from the repository root.
- The build fails when a bundled Markdown document links to a file the bundle lacks.

Key files in `src-tauri/legal/`:

| File or folder | Purpose |
|---|---|
| `MPL-2.0.txt`, `MPL-SOURCE-ACCESS.txt` | License text and source-access instructions |
| `mpl-sources/` | Exact-version `.crate` archives for MPL-licensed crates |
| `mpl-sources.json` | Name, version, archive path, SHA-256, download URL, and repository for each archive |
| `notice-manifest.json` | Scope, lock file hashes, and the source and text hash of each notice |
| `toolchain/` | Rust standard library and LLVM notices, and license texts |
| `COMMERCIAL-TERMS.md`, `PRIVACY-NOTICE.md`, and related documents | Binding product terms. Do not reword them in code or docs |

### Release checks

Build the app and verify that `Contents/Resources/legal` contains the notices, license texts, source-access instructions, and exact-version source archives. Compare those files with the repository copies, and exercise both **Settings** → **About** actions in the installed app. These packaging checks do not replace review of license obligations or dependency coverage.

### Update the snapshot

When dependencies, assets, or release targets change:

1. Resolve the locked target dependency graph and inspect the emitted client bundle and CSS inputs. Include native vendored code and relevant generated code, not only package-level license metadata.
2. Preserve full applicable license texts, credits, and notices. For packages missing license files, consult the exact upstream revision, and do not invent copyright holders. Record sources in `notice-manifest.json`.
3. Copy each exact MPL source archive from the Cargo registry cache, verify its SHA-256 against `Cargo.lock`, and compare its contents with the registry source used by the build. If modified, deliver the modified source instead and update the instructions. Refresh `mpl-sources.json` and `MPL-SOURCE-ACCESS.txt`.
4. Review the resulting notices and source coverage.
5. Build and verify the packaged files and the **Settings** actions. Retain the exact source archives with every distributed release.

This snapshot targets default-feature Apple Silicon macOS builds. Other targets and feature selections require a corresponding review. Downloaded model weights and their license and provenance information are separate from this software notice snapshot.

## License of the source

The repository uses the custom source-available license in [LICENSE.md](https://github.com/carlesonielfa/isoboard-app/blob/main/LICENSE.md). The commercial terms and the English and Spanish legal package are in [src-tauri/legal](https://github.com/carlesonielfa/isoboard-app/tree/main/src-tauri/legal). Modified builds must be clearly identified as unofficial. Third-party software and model weights keep their own licenses.
