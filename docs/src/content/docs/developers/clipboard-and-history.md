---
title: Clipboard and history
description: How isoboard publishes sanitized HTML with a plain-text fallback to the clipboard and stores capture history in SQLite with count and byte limits.
sidebar:
  label: Clipboard and history
  order: 8
---

isoboard publishes each result to the system clipboard as one item with two representations, a sanitized HTML body fragment and a plain-text fallback. After native publication succeeds, it stores the capture in a local SQLite history that keeps up to 1,000 successful results under a 500 MiB cap. A failed capture is stored separately, and a history write failure never undoes a clipboard write.

For what the HTML may contain see [HTML output](../html-output/). For the capture flow around these modules see [Architecture](../architecture/). For user behavior see [Product specification](../spec/#clipboard-history).

## Clipboard publication

### What is published

The renderer returns a `NormalizedHtml` with a canonical document, a sanitized body, and derived plain text. The clipboard module in [`src-tauri/src/clipboard/`](https://github.com/carlesonielfa/isoboard-app/tree/main/src-tauri/src/clipboard) builds a `ClipboardPayload` from two of them:

| Field | Source | Why |
|---|---|---|
| `html` | The sanitized **body fragment** | Native clipboard APIs must not get a nested `<html><body>` wrapper |
| `plain_text` | Derived from the final body | Fallback for targets that only read text |

The canonical full document goes to capture state, preview, and History, not to the clipboard. Embedded images are generated PNG data URLs, so they stay usable after pasting without temporary files. RTF is not published and remains later work.

### The write boundary

Capture orchestration depends on the `ClipboardWriter` trait (`write` for a payload, `write_plain_text` for text only), not on an operating-system API. `prepare_and_write_with_conversion` normalizes and writes. `write_normalized` publishes a finalized document without parsing its embedded images again. `write_normalized_with_stages` does the same for a split run and keeps each raw stage response for History. All return a `PreparedClipboard` with everything History needs.

A single shared `arboard` adapter implements the trait for every platform:

- It creates the `arboard::Clipboard` on first write and keeps it for the lifetime of the app. This matters on Linux, where the process owns the selection, and it serializes Windows writes as arboard requires.
- HTML is written with `set_html(html, Some(plain_text))`, so both representations land in one item. Plain text uses `set_text`.

<!-- os:macos -->
On macOS the write runs on Tauri's main thread (`run_on_main_thread`), because AppKit and WebView pasteboard serialization stay there. Other platforms use a blocking worker.

The shared path is not runtime-validated on Windows or Linux, and pure Wayland support is deferred. Keep platform behavior behind this interface, as the repository's platform rule requires.

### Failure semantics

- Validation, inference, and preparation failures never call the clipboard writer.
- Cancellation before publication leaves the clipboard unchanged.
- After native publication starts, an operating-system failure may leave a partial write, and restoration is not guaranteed.
- `ClipboardError` is either `Preparation` (a rendering error) or `Write`.

## Capture to history sequence

History insertion begins only after clipboard publication succeeds, so a stored success always matches something that was published.

| Moment | History effect |
|---|---|
| Image captured | A **pending** item appears in memory with a thumbnail. It has no clipboard or delete action |
| Clipboard publication succeeds | The pending item becomes a saved success with the same identity |
| Processing or publication fails | The pending item becomes a saved failed attempt with a sanitized error code and message |
| Capture cancelled | The pending item is dropped. Nothing is stored |
| History write fails | A separate `history-error` event is shown. The clipboard result stands |

Restoring a successful entry writes its saved data to the clipboard and runs no inference.

## History storage

[`src-tauri/src/history.rs`](https://github.com/carlesonielfa/isoboard-app/blob/main/src-tauri/src/history.rs) defines a `HistoryStore` interface and a `SqliteHistoryStore` adapter. The interface is independent of TOML configuration, and SQLite specifics stay in the adapter.

| Item | Value |
|---|---|
| File | `history.sqlite3` beside `config.toml` |
| Permissions | `0600` on Unix |
| Busy timeout | 5 seconds |
| Table | `history_entries`, one row per completed attempt, with `outcome` of `success` or `failure` |
| Order | A monotonically increasing `sequence` |

Development runs use `.dev/history.sqlite3`. See [Setup](../setup/).

### Limits and retention

Retention defaults come from [`config/application.jsonc`](https://github.com/carlesonielfa/isoboard-app/blob/main/config/application.jsonc) under `history`.

| Limit | Value |
|---|---|
| Successful entries | Newest 1,000 (`maxEntries`), a safety cap behind the byte budget |
| Failed attempts | Newest 20, counted separately, never affecting the success count |
| Total stored bytes | 500 MiB (`maxBytes`) |
| Original captures | Newest 50 (`maxOriginals`) within 1 GiB (`maxOriginalBytes`), counted separately from `maxBytes` |
| Thumbnail | At most 256 px on the longest edge and 256 KiB |
| Failure message | 4,096 bytes |
| Settings snapshot | 2,048 bytes |

Insertion and pruning run in one transaction at the storage boundary. `prune` deletes successes beyond the count, deletes failures beyond their own count, and then removes the oldest rows of either outcome until the byte total, including retained raw failure output, is at or below the cap. The database runs in WAL mode. An entry that alone exceeds the cap is rejected. The menu bar lists the five most recent successes.

Thumbnail decoding is bounded (20 MiB source, 100 million pixels, 10,000 px per side).

### Original captures

When `[history] keep_originals` is on (the default), the captured image bytes are kept unchanged for successes and failed attempts, as `history-originals/<id>` beside the database. The directory is `0700` and each file `0600`. The database is the source of truth: `original_bytes` is the file size, or NULL when no original is kept. Inside the connection lock, an insert writes the file through a temporary name and a rename, then commits the row. `prune` then clears `original_bytes` on originals beyond the newest `maxOriginals` or `maxOriginalBytes`, without removing the entry. After every write, delete, clear or discard, and when the store opens, a sweep deletes each file that no row lists, so a failed transaction, an expired original or a crash all end the same way. Turning the setting off calls `discard_originals`, which clears every `original_bytes` and sweeps. The pending capture holds its original in memory until it settles.

### What a success stores

- The exact raw inference HTML, the canonical clean document, the sanitized body, and the derived plain text
- Raw responses from each stage of a split run (`first_inference_html`, `second_inference_html`), never rendered
- A preview HTML capped at 8 MiB for storage. The in-window clean rich preview is capped at 32 KiB and rendered only from sanitized HTML in an isolated surface
- The generated thumbnail, the original image dimensions, the timestamp, the source kind, and a snapshot of the Profile ID and display name
- A compact settings snapshot, frozen at capture start (see below)

Raw inference HTML is source text only. Debug actions copy full raw or clean HTML as plain text to the clipboard and never render it.

### Reading history

`list_history` returns pages of light summary rows, newest first, keyed by `sequence`: `beforeSequence` pages towards older rows and `minSequence` re-reads everything from a known row onwards, at most 500 rows per call. The History view loads 50 rows at a time as the list scrolls, and on `history-changed` re-reads only from the oldest row it shows, which picks up inserts, deletes and pruning together. Summary rows carry no preview HTML or image data. The selected entry's preview comes from `get_history_preview`, and thumbnails load through the `history-thumbnail://` URI scheme by entry id, cached by the webview because ids are never reused. The pending capture's thumbnail is still sent inline.

### Search and filters

`list_history` also takes a `HistoryFilter`: `query`, `outcome`, `profileId`, `contains`, and the RFC 3339 UTC timestamps `since` (inclusive) and `until` (exclusive), compared as text. `contains` lists content tags that a result must all have. The view turns a chosen day range into local midnight of the first day and local midnight after the last one, so a day that changes daylight saving time still counts whole. Every sequence bound and the `hasMore` check apply within the filter, so paging and refreshes work unchanged on a filtered list. Results stay in newest-first order, not relevance order.

The text index is the FTS5 table `history_search`, with the `unicode61` tokenizer and diacritics removed. It holds each entry's plain text, or its error message for a failed attempt, and its profile name, under `rowid = sequence`, because the implicit rowid of `history_entries` may change on VACUUM. Insert and delete triggers keep it current, and no code updates those columns afterwards. The query is built by quoting every typed word as a prefix term, so FTS5 operators and syntax in the input are searched for literally. Matches come back as `matchSnippet`, with each match between U+E000 and U+E001, which the list renders as highlights.

Content tags are detected once, when a result is stored, from its sanitized clipboard body (`rendering::content_tags`), and kept in `content_tags` as a bitmask. A tokenizer reads the tags and attributes, so escaped text that looks like markup never counts. `tables` is a `<table>`, `images` an `<img>`, `code` a `<pre>`, `<code>` or `<samp>`, and `styling` an emphasis tag (`b`, `strong`, `i`, `em`, `u`, `s`, `strike`, `del`, `sub`, `sup`) or an inline `color`, `background-color`, `font-size`, `font-style`, `font-weight` or `text-decoration`. Layout styles such as alignment, spacing and borders do not count. Each tag keeps a fixed bit (`ContentTag::bit`), so a new tag takes the next free bit without a migration. Failed attempts and results stored before detection have NULL and never match a contents filter; there is no backfill.

`get_history_facets` returns the profiles present in History, each with the name of its latest entry, which the Profile filter offers. The pending capture is left out of a filtered list.

### Settings snapshot

Successful and failed rows both store the provider and model for each used pass, the Tables, Colors, and Images flags, the effective application mode, and whether Additional Rules and a separate pass were used. The snapshot holds no credentials, endpoints, or rule text. Older rows without a snapshot stay available and show that settings are unavailable.

### Failed attempts

A failed row holds the Profile snapshot, source kind, timestamp, a bounded thumbnail and dimensions when available, a sanitized error code and message, and optional bounded diagnostic details. It has no result to restore. Preparation and publication failures also keep the exact response from the stage that failed (`failure_raw_html`), within the provider output limit, and that text counts toward the byte cap. The History view shows diagnostic details only in Debug mode, and the copied error report includes them.

### Formatting baseline

A successful capture can keep a sanitized formatting baseline, taken after the last model pass and before feature cleanup, with generated crops embedded. It lets History toggle Tables, Colors, and Images on a saved entry without inference. The baseline counts toward the byte cap and is never the raw model response.

How toggling behaves depends on the stored mode:

- With a baseline, a change applies deterministic cleanup to the baseline. For Prompt only and Prompt + cleanup entries, options that were disabled at capture time cannot be enabled, and enabled ones can be disabled and restored.
- Cleanup-only entries with a baseline allow every option to toggle. Restoration depends on what the model produced.
- Older entries without a baseline can only remove features from the saved output.
- Empty variations disable copying and leave the clipboard alone.

Neither the saved result nor its capture-time settings change. Both copy actions derive text from the saved sanitized body or the current variation.

## Schema migrations

Migrations use `rusqlite_migration` and the SQLite `user_version` pragma. The current schema version is 10.

| Version | Change |
|---|---|
| 1 | `history_entries` for successful captures |
| 2 | `first_inference_html`, `second_inference_html` |
| 3 | Rebuild with `outcome`, nullable result columns, error columns, and a `CHECK` that ties each outcome to the columns it may fill |
| 4 | `settings_snapshot` |
| 5 | `formatting_body_html` |
| 6 | `error_details` |
| 7 | `failure_raw_html` |
| 8 | `original_bytes` |
| 9 | `history_search` FTS5 table backfilled from existing rows, and its insert and delete triggers |
| 10 | `content_tags` |

A database whose `user_version` is newer than the build supports is refused with a schema error instead of being touched. To add a migration, append an `M::up` entry, bump `SCHEMA_VERSION`, and add a test that builds the previous schema and upgrades it. The existing tests do this for versions 1 to 3.

This is an exception to the pre-release rule that TOML carries no migrations. History is user data on disk, so schema upgrades are real migrations, while the models.dev cache is disposable and has none.

## Privacy

History holds captured content and may include sensitive text, image thumbnails and full-resolution original captures. It is local application data under the app's storage permissions. Deletion is logical from the history API's point of view, and SQLite and filesystem recovery behavior is not a secure-erasure guarantee. The History tab offers single-entry deletion only. The history API also has a clear-all operation that no UI control calls.

## Tests

The history tests in `history.rs` cover limits, transactional pruning, failed attempts, thumbnails, and migrations. The clipboard tests use a counting `ClipboardWriter` fake. Run both with:

```sh
cargo test --manifest-path src-tauri/Cargo.toml history::
cargo test --manifest-path src-tauri/Cargo.toml clipboard::
```

Native clipboard behavior needs verification in the desktop app, because a browser preview cannot validate it. See [Testing and CI](../testing-and-ci/).

## Related

- [HTML output](../html-output/)
- [Inference and prompts](../inference/#raw-stage-responses)
- [UI system](../ui/)
- [Architecture](../architecture/)
