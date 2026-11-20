---
title: Licensing implementation
description: How isoboard stores the seven-day evaluation, validates commercial keys through Polar's client API, caches rights offline and handles support deadlines.
sidebar:
  label: Licensing
  order: 9
---

isoboard licensing is two independent local records. A seven-day evaluation lives in `licensing.json`, and a verified commercial license lives in `commercial-license.json` with its key in the Keychain. Rust owns both. Validation calls Polar's public customer-portal endpoint with no administrative token, and cached rights keep working offline.

The terms themselves are in the bundled legal documents and `LICENSE.md`. This page covers only the implementation. For how users see it, see the [user docs](https://docs.isoboard.app/license/pricing-and-licensing/). For the intended behavior, see [Product specification](../spec/#licensing-setup-and-evaluation).

## Code map

| File | Responsibility |
|---|---|
| [`src-tauri/src/licensing.rs`](https://github.com/carlesonielfa/isoboard-app/blob/main/src-tauri/src/licensing.rs) | Evaluation record, status rules, Tauri commands |
| [`src-tauri/src/licensing/commercial.rs`](https://github.com/carlesonielfa/isoboard-app/blob/main/src-tauri/src/licensing/commercial.rs) | Commercial record, activation, removal, scheduled checks |
| [`src-tauri/src/licensing/polar.rs`](https://github.com/carlesonielfa/isoboard-app/blob/main/src-tauri/src/licensing/polar.rs) | Polar request, response classification, support deadline |
| [`config/product.jsonc`](https://github.com/carlesonielfa/isoboard-app/blob/main/config/product.jsonc) | Trial length, intervals, Polar identifiers, checkout URL |

The frontend components are described in [UI system](../ui/#licensing-ui).

## Product configuration

All licensing constants are build-time values in `config/product.jsonc`, parsed by Rust (`product_config.rs`) and exported to the frontend with the generated bindings. They are not customer-editable and must not hold secrets. Rebuild after editing. See [Configuration](../configuration/).

| Key | Meaning |
|---|---|
| `licensing.trialDays` | Evaluation length (7). Shared by the Rust calculation and the UI copy |
| `licensing.supportMonths` | Calendar months of support from license creation (12) |
| `licensing.coveredMajor` | Major version the entitlement covers (1) |
| `licensing.keyPrefix` | Expected key prefix, `ISOAPP-` |
| `licensing.validationIntervalSeconds` | Minimum time between scheduled Polar checks (86,400) |
| `licensing.backgroundPollSeconds` | How often the scheduler checks whether a check is due (60) |
| `licensing.statusRefreshMilliseconds` | UI-only status refresh, never calls Polar (60,000) |
| `polar.organizationId`, `polar.benefitId` | The identities a key must match |
| `polar.validationUrl` | Customer-portal validation endpoint |
| `polar.requestTimeoutSeconds`, `polar.connectTimeoutSeconds` | 15 s overall, 5 s connect |
| `checkoutUrl` | Polar hosted checkout opened by Get a license |

The production Polar environment is used in development builds too. Local storage stays isolated.

## Evaluation storage

`licensing.json` sits beside `config.toml`. A development run writes it under `.dev/`, so it has its own evaluation. The file holds three fields:

```json
{ "started_at": 1768000000, "last_seen": 1768000000, "notice_dismissed": false }
```

- Timestamps are UTC epoch seconds. Settings converts them to local time for display.
- The evaluation starts once, when the user picks Continue without a license key. The start is `started_at`, and the end is `started_at + trialDays` (168 hours for 7 days).
- Writes are atomic: a temporary file in the same directory is synced and then persisted over the old file.
- No account, card, or network request is involved.

### Status rules

`LicenseStore::status` returns `NotStarted`, `Active`, `Expired`, or `Unavailable`.

- A record with no start and no history is `NotStarted`, and the UI shows the first-run choice.
- `last_seen` is advanced on each status read while the evaluation is active or expired. Once `last_seen` reaches the end, the evaluation stays `Expired` even if the clock moves back.
- A start of zero, a `last_seen` before the start, a current time before the start or before `last_seen`, an unreadable file, or timestamps beyond year 9999 give `Unavailable`. The UI shows the dates as unavailable and personal use remains available.
- Clock rollback and corrupt dates never start a fresh evaluation. Corrupt files are preserved, not rewritten.
- Starting is refused when the clock reads zero or would overflow the representable range.
- Deleting app data can remove the record. The intended terms do not grant another evaluation in that case, and the storage is explicitly not tamper-resistant.

### Expiry notice

On an ordinary launch where the status is `Expired` and `notice_dismissed` is false, the snapshot sets `show_expiry_notice`. Login launches pass `ordinary_launch = false` and never schedule it. Dismissal is persisted. The notice never appears during capture, and capture, History, and Settings keep working after expiry.

## Commercial license storage

`commercial-license.json` (also beside `config.toml`) stores a `Record`:

| Field | Content |
|---|---|
| `license.reference` | Opaque managed-secret reference for the key |
| `license.masked_key` | `••••-` plus the last four characters |
| `license.license_id` | Polar license identity (UUID) |
| `license.organization_id`, `license.benefit_id` | What was validated |
| `license.covered_major` | Major version covered |
| `license.verified_at`, `license.last_attempt_at` | UTC epoch seconds |
| `license.granted`, `license.rejection` | Cached entitlement and the last confirmed rejection message |
| `license.support_expires_at` | Cached support deadline, if known |
| `choice_completed` | The first-run choice was made |
| `pending_cleanup` | Keychain references still to delete |

The file never contains the full key or customer billing data. Unknown fields are rejected on load. An unreadable file blocks changes and shows a restore message, so the app does not overwrite state it cannot parse.

The key itself is stored as a `license-key` item in the managed Keychain namespace. See [Secrets and Keychain](../secrets/).

The commercial snapshot is `active` only when the stored organization, benefit, and covered major match the build, `granted` is true, and no load error exists. `current_major()` is the crate's major version with a floor of 1, so development builds at 0.x exercise the 1.x entitlement. Other major versions do not inherit it.

## Polar validation

Validation is one public-client request. There is no administrative token, no device activation record, and no usage increment.

1. The app sends `POST` to `polar.validationUrl` with JSON `{ key, organization_id, benefit_id }`.
2. The client sets a `isoboard/<version>` user agent, a 15 s timeout, a 5 s connect timeout, and refuses redirects. The response body is read in chunks and capped at 64 KiB. Keys and responses are never logged.
3. `classify` maps the result to `Granted`, `Rejected`, or `Unavailable`.

Polar's Current API is used with no version header.

### Response classification

| Response | Result |
|---|---|
| 200, organization or benefit does not match the configured ones | `Rejected`: key does not cover this edition |
| 200, status `revoked` or `disabled` | `Rejected`: license no longer active |
| 200, status `granted`, valid UUID, no `expires_at`, no activation limit, no usage limit | `Granted` |
| 200, status `granted` but an expiry or limit is set | `Unavailable`: settings do not match this product. A changed benefit is reconciled, never silently narrowed |
| 200, unknown status, malformed body, or non-UUID id | `Unavailable` |
| 404 with `error: ResourceNotFound` and detail `Not found`, `License key is no longer active.`, `License key has expired.`, or `License key does not match given benefit.` | `Rejected` |
| 404 with any other body, including the retired-version `{"detail":"Not Found"}` | `Unavailable` |
| 429 | `Unavailable`: rate limited |
| Network error, 400, 401, 403, 422, 5xx, HTML from a proxy | `Unavailable` |

Only a recognized Polar rejection changes cached permission. Network, rate-limit, authentication, malformed-response, and service failures preserve it. No offline expiry is imposed.

## Activation

`activate_commercial_license` takes the key from the UI input, which trims and uppercases it and enforces the `ISOAPP-` prefix and UUID shape before submitting. Polar stays authoritative. The backend trims again and rejects empty keys, keys over 1,024 bytes, and control characters.

1. Validate with Polar. First activation requires a successful online check.
2. Reject if `current_major()` differs from `coveredMajor`.
3. Stage the key in the Keychain as a new immutable item.
4. Write `commercial-license.json` atomically with the new reference. Any previous reference goes into `pending_cleanup`.
5. If the file write fails, delete the staged item. If that also fails, the error says so and the previous license is unchanged.
6. Delete the old key items listed in `pending_cleanup`.

A failed replacement never disturbs an existing license. A commercial entitlement takes precedence over evaluation status in the snapshot. It hides the first-run choice and the expiry notice without starting or resetting the evaluation.

## Background checks

`start_background` spawns a loop that wakes every `backgroundPollSeconds` and calls `recheck(false)` unless a capture is busy. The loop is independent of the webview.

- A check is due when at least `validationIntervalSeconds` have passed since `last_attempt_at`. The attempt time is persisted before the request, so failed attempts count and the limit holds across launches.
- Non-forced checks skip silently when another license operation is running.
- Manual retry after a verification error calls `recheck(true)`.
- A `Granted` result for the same `license_id` updates `verified_at` and clears any rejection. A different identity is treated as an error and changes nothing.
- A `Rejected` result sets `granted = false` and stores the message. The revocation holds in memory even if persisting it fails.
- An `Unavailable` result only sets a verification error shown with Retry. Cached entitlement is untouched.

## Support deadline

Priority Email Support is independent of perpetual commercial permission.

- `support_expiry` takes the license's top-level `created_at`, adds `supportMonths` calendar months, and returns UTC epoch seconds. Month arithmetic clamps a leap day to the last day of February.
- The period starts at license issuance, regardless of purchase or device activation. Rotating a key on the same Polar record keeps the original `created_at`.
- A missing or malformed `created_at` produces no deadline. On first activation the UI shows support as unavailable and the license is still granted.
- On later refreshes, a missing deadline never erases a verified one for the same license.
- The deadline refreshes only with license validation. There is no separate request or refresh control.
- The Email support button opens a draft addressed to `supportEmail` in the default email app. It does not send anything.

## Removal

`remove_commercial_license` clears the local entitlement and queues the Keychain item for deletion without calling Polar. It does not revoke the purchase and does not reset the evaluation. A failed Keychain cleanup stays in `pending_cleanup`, shows a retryable message, and cannot restore the removed entitlement.

## Tests

Offline lifecycle and protocol tests run with the ordinary suite:

```sh
cargo test --manifest-path src-tauri/Cargo.toml licensing::
```

They use a fake provider and an in-memory credential store, and cover:

- the evaluation boundary, expiry persistence, corrupt dates, clock rollback, and failed writes
- Polar response classification, support deadline arithmetic, and which rejections remove permission
- activation surviving a reopen offline, removal without contacting Polar, and failed replacement preserving the old license
- the 24-hour throttle and a confirmed rejection surviving restart
- retryable Keychain cleanup that never restores the entitlement
- commercial status overriding the evaluation without starting or resetting it

One test, `production_validation_and_native_keychain_lifecycle`, is `#[ignore]`d. It is a maintainer-only release check that validates once against production Polar and exercises isolated macOS Keychain storage, offline restoration, and local removal. It needs an authorized key supplied on stdin, so it never runs in CI or in ordinary contributions. Do not run it unless you hold an authorized key, and never put a key in a command history, a script, or a log. Contributors do not need it.

Release verification still has to confirm company seat delivery, refund workflows (managed in Polar), and the native UI lifecycle. See [Releases and notices](../releases-and-notices/) and [Testing and CI](../testing-and-ci/).

## Related

- [Product specification](../spec/#licensing-setup-and-evaluation)
- [Secrets and Keychain](../secrets/)
- [UI system](../ui/#licensing-ui)
- [Configuration](../configuration/)
