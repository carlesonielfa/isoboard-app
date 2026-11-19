//! Persisted capture outcomes and their Tauri IPC adapter.
//!
//! The store owns the durable representation of a successful clipboard
//! publication. Capture code only submits a complete record after the
//! clipboard write succeeds, so a history failure cannot turn a usable
//! clipboard result into a capture failure.

use crate::application_config::APPLICATION;
use crate::clipboard::{ClipboardError, ClipboardPayload, ClipboardWriter};
use crate::rendering;
use base64::Engine as _;
use chrono::{DateTime, Datelike, Local};
use image::{imageops::FilterType, DynamicImage, GenericImageView, ImageFormat, ImageReader};
use rusqlite::{params, types::Value, Connection, OptionalExtension, Row, TransactionBehavior};
use rusqlite_migration::{Migrations, M};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::menu::{CheckMenuItemBuilder, Menu, MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::{AppHandle, Manager, Runtime, State};
use tauri_specta::Event;
use uuid::Uuid;

mod navigation;
mod originals;
pub(crate) use navigation::HistoryNavigationState;
use originals::OriginalsDirectory;

pub const MAX_THUMBNAIL_EDGE: u32 = 256;
pub const MAX_THUMBNAIL_BYTES: usize = 256 * 1024;
pub const MAX_PREVIEW_HTML_BYTES: usize = rendering::MAX_DOCUMENT_BYTES;
const MAX_CAPTURE_SOURCE_BYTES: usize = 64;
const MAX_TIMESTAMP_BYTES: usize = 64;
const MAX_ENTRY_ID_BYTES: usize = 64;
const MAX_PROFILE_ID_BYTES: usize = 256;
const MAX_PROFILE_NAME_BYTES: usize = 512;
const MAX_THUMBNAIL_SOURCE_BYTES: usize = 20 * 1024 * 1024;
const MAX_THUMBNAIL_PIXELS: u64 = 100_000_000;
const MAX_THUMBNAIL_DIMENSION: u32 = 10_000;
const MAX_THUMBNAIL_DECODE_BYTES: u64 = 100 * 1024 * 1024;
const MAX_FAILED_ATTEMPTS: usize = 20;
pub(crate) const MAX_HISTORY_PAGE_SIZE: usize = 500;
const MAX_FAILURE_MESSAGE_BYTES: usize = 4096;
const MAX_SETTINGS_SNAPSHOT_BYTES: usize = 2048;
const HISTORY_MENU_PREFIX: &str = "history-entry-";
const CAPTURE_PROFILE_MENU_PREFIX: &str = "capture-profile:";
const HISTORY_MENU_PREVIEW_CHARS: usize = 72;
const TRAY_HISTORY_LIMIT: usize = 5;
const TRAY_MENU_PREVIEW_CHARS: usize = 40;
const VOID_TAGS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

const SCHEMA_VERSION: i64 = 10;
const SCHEMA_V1: &str = r#"
CREATE TABLE IF NOT EXISTS history_entries (
    sequence INTEGER NOT NULL UNIQUE,
    id TEXT PRIMARY KEY NOT NULL,
    timestamp TEXT NOT NULL,
    capture_source TEXT NOT NULL,
    processing_profile_id TEXT NOT NULL,
    processing_profile_name TEXT NOT NULL,
    thumbnail_png BLOB NOT NULL,
    raw_inference_html TEXT NOT NULL,
    clean_document_html TEXT NOT NULL,
    clipboard_body_html TEXT NOT NULL,
    plain_text TEXT NOT NULL,
    preview_html TEXT NOT NULL,
    image_width INTEGER,
    image_height INTEGER,
    stored_bytes INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS history_entries_sequence_order_idx
    ON history_entries(sequence DESC);
"#;

const SCHEMA_V3: &str = r#"
CREATE TABLE new_history_entries (
    sequence INTEGER NOT NULL UNIQUE,
    id TEXT PRIMARY KEY NOT NULL,
    outcome TEXT NOT NULL CHECK (outcome IN ('success', 'failure')),
    timestamp TEXT NOT NULL,
    capture_source TEXT NOT NULL,
    processing_profile_id TEXT NOT NULL,
    processing_profile_name TEXT NOT NULL,
    thumbnail_png BLOB,
    raw_inference_html TEXT,
    clean_document_html TEXT,
    clipboard_body_html TEXT,
    plain_text TEXT,
    preview_html TEXT,
    image_width INTEGER,
    image_height INTEGER,
    stored_bytes INTEGER NOT NULL DEFAULT 0,
    first_inference_html TEXT,
    second_inference_html TEXT,
    error_code TEXT,
    error_message TEXT,
    CHECK (
        (outcome = 'success' AND thumbnail_png IS NOT NULL
         AND raw_inference_html IS NOT NULL AND clean_document_html IS NOT NULL
         AND clipboard_body_html IS NOT NULL AND plain_text IS NOT NULL
         AND preview_html IS NOT NULL AND error_code IS NULL AND error_message IS NULL)
        OR
        (outcome = 'failure' AND error_code IS NOT NULL AND error_message IS NOT NULL
         AND raw_inference_html IS NULL AND clean_document_html IS NULL
         AND clipboard_body_html IS NULL AND plain_text IS NULL
         AND preview_html IS NULL AND first_inference_html IS NULL
         AND second_inference_html IS NULL AND stored_bytes = 0)
    )
);
INSERT INTO new_history_entries (
    sequence, id, outcome, timestamp, capture_source, processing_profile_id,
    processing_profile_name, thumbnail_png, raw_inference_html, clean_document_html,
    clipboard_body_html, plain_text, preview_html, image_width, image_height,
    stored_bytes, first_inference_html, second_inference_html
) SELECT sequence, id, 'success', timestamp, capture_source, processing_profile_id,
         processing_profile_name, thumbnail_png, raw_inference_html, clean_document_html,
         clipboard_body_html, plain_text, preview_html, image_width, image_height,
         stored_bytes, first_inference_html, second_inference_html
  FROM history_entries;
DROP TABLE history_entries;
ALTER TABLE new_history_entries RENAME TO history_entries;
CREATE INDEX history_entries_sequence_order_idx ON history_entries(sequence DESC);
CREATE INDEX history_entries_success_order_idx ON history_entries(sequence DESC)
    WHERE outcome = 'success';
"#;

/// Full-text search over each entry's text and profile name. The search table
/// keeps its own copy of the text, keyed by `sequence`, which unlike the
/// implicit rowid survives VACUUM; triggers keep it in step with inserts and
/// deletes.
const SCHEMA_V9_SEARCH: &str = r#"
CREATE VIRTUAL TABLE history_search USING fts5(
    body, profile_name, tokenize = 'unicode61 remove_diacritics 2'
);
INSERT INTO history_search (rowid, body, profile_name)
    SELECT sequence, COALESCE(plain_text, error_message, ''), processing_profile_name
    FROM history_entries;
CREATE TRIGGER history_search_insert AFTER INSERT ON history_entries BEGIN
    INSERT INTO history_search (rowid, body, profile_name)
    VALUES (new.sequence, COALESCE(new.plain_text, new.error_message, ''),
            new.processing_profile_name);
END;
CREATE TRIGGER history_search_delete AFTER DELETE ON history_entries BEGIN
    DELETE FROM history_search WHERE rowid = old.sequence;
END;
"#;

fn migrations() -> Migrations<'static> {
    Migrations::new(vec![
        M::up(SCHEMA_V1),
        M::up(
            "ALTER TABLE history_entries ADD COLUMN first_inference_html TEXT;\n\
             ALTER TABLE history_entries ADD COLUMN second_inference_html TEXT;",
        ),
        M::up(SCHEMA_V3),
        M::up("ALTER TABLE history_entries ADD COLUMN settings_snapshot TEXT;"),
        M::up("ALTER TABLE history_entries ADD COLUMN formatting_body_html TEXT;"),
        M::up("ALTER TABLE history_entries ADD COLUMN error_details TEXT;"),
        M::up("ALTER TABLE history_entries ADD COLUMN failure_raw_html TEXT;"),
        // Size of the original capture file kept for this entry, or NULL.
        M::up("ALTER TABLE history_entries ADD COLUMN original_bytes INTEGER;"),
        M::up(SCHEMA_V9_SEARCH),
        // Bitmask of `rendering::ContentTag` bits found in a result's body,
        // detected when it is stored; NULL for failures and older entries.
        M::up("ALTER TABLE history_entries ADD COLUMN content_tags INTEGER;"),
    ])
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, specta::Type, tauri_specta::Event)]
#[tauri_specta(event_name = "history-changed")]
pub(crate) struct HistoryChangedEvent;

#[derive(Clone, Debug, Deserialize, Serialize, specta::Type, tauri_specta::Event)]
#[tauri_specta(event_name = "open-history")]
#[serde(rename_all = "camelCase")]
pub(crate) struct OpenHistoryEvent {
    pub(crate) request_id: u64,
    pub(crate) entry_id: Option<String>,
}

pub(crate) fn open_history_window<R: Runtime>(
    app: &AppHandle<R>,
    entry_id: Option<String>,
) -> Result<(), String> {
    // Keep the destination until the main webview is ready; the event only wakes it.
    let request = app.state::<HistoryNavigationState>().queue(entry_id)?;
    crate::window_presentation::reveal_main_window(app)?;
    if let Err(error) = request.emit(app) {
        eprintln!("isoboard: could not notify main window of History navigation: {error}");
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub(crate) fn get_pending_history_navigation(
    state: State<'_, HistoryNavigationState>,
) -> Result<Option<OpenHistoryEvent>, String> {
    state.pending()
}

#[tauri::command]
#[specta::specta]
pub(crate) fn acknowledge_history_navigation(
    state: State<'_, HistoryNavigationState>,
    request_id: u64,
) -> Result<(), String> {
    state.acknowledge(request_id)
}

#[derive(Clone, Debug, Serialize, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "history-error")]
pub(crate) struct HistoryErrorEvent {
    pub(crate) code: &'static str,
    pub(crate) message: &'static str,
}

/// Selects which stored inference representation should be copied for debug.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub(crate) enum HistoryDebugKind {
    Raw,
    Clean,
    First,
    Second,
}

/// Records which provider and model produced a history entry. The provider is
/// kept as its stored id rather than `InferenceBackend` so entries made with a
/// provider that a later version no longer supports still load.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HistoryModelSnapshot {
    pub(crate) provider: String,
    pub(crate) model: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HistorySettingsSnapshot {
    pub(crate) first_model: Option<HistoryModelSnapshot>,
    pub(crate) second_model: Option<HistoryModelSnapshot>,
    pub(crate) tables: bool,
    pub(crate) colors: bool,
    pub(crate) images: bool,
    pub(crate) application_mode: crate::profiles::ConversionSettingsApplicationMode,
    pub(crate) additional_rules_applied: bool,
    pub(crate) separate_pass_used: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HistoryEntryInput {
    pub(crate) id: Option<String>,
    pub(crate) timestamp: String,
    pub(crate) capture_source: String,
    pub(crate) processing_profile_id: String,
    pub(crate) processing_profile_name: String,
    pub(crate) settings_snapshot: HistorySettingsSnapshot,
    pub(crate) screenshot_bytes: Vec<u8>,
    /// Keeps `screenshot_bytes` as the entry's original capture.
    pub(crate) keep_original: bool,
    pub(crate) raw_inference_html: String,
    pub(crate) clean_document_html: String,
    pub(crate) clipboard_body_html: String,
    pub(crate) formatting_body_html: Option<String>,
    pub(crate) plain_text: String,
    /// Exact provider response from the first inference stage, when this
    /// capture used a split pipeline.
    pub(crate) first_inference_html: Option<String>,
    /// Exact provider response from the second inference stage, when this
    /// capture used a split pipeline.
    pub(crate) second_inference_html: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HistoryEntry {
    pub(crate) sequence: i64,
    pub(crate) id: String,
    pub(crate) timestamp: String,
    pub(crate) capture_source: String,
    pub(crate) processing_profile_id: String,
    pub(crate) processing_profile_name: String,
    pub(crate) settings_snapshot: Option<HistorySettingsSnapshot>,
    pub(crate) thumbnail_png: Vec<u8>,
    pub(crate) raw_inference_html: String,
    pub(crate) clean_document_html: String,
    pub(crate) clipboard_body_html: String,
    pub(crate) formatting_body_html: Option<String>,
    pub(crate) plain_text: String,
    pub(crate) first_inference_html: Option<String>,
    pub(crate) second_inference_html: Option<String>,
    pub(crate) preview_html: String,
    pub(crate) image_width: Option<u32>,
    pub(crate) image_height: Option<u32>,
    pub(crate) stored_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HistorySummaryRow {
    pub(crate) sequence: i64,
    pub(crate) id: String,
    pub(crate) timestamp: String,
    pub(crate) capture_source: String,
    pub(crate) processing_profile_id: String,
    pub(crate) processing_profile_name: String,
    pub(crate) settings_snapshot: Option<HistorySettingsSnapshot>,
    pub(crate) has_thumbnail: bool,
    pub(crate) has_original: bool,
    pub(crate) plain_text_preview: String,
    /// Matched text around a search hit, with matches wrapped in
    /// `SEARCH_MATCH_START` and `SEARCH_MATCH_END`.
    pub(crate) match_snippet: Option<String>,
    /// Kinds of content in the result; empty for entries stored before
    /// detection existed.
    pub(crate) content_tags: Vec<rendering::ContentTag>,
    pub(crate) has_plain_text: bool,
    pub(crate) can_restore_formatting: bool,
    pub(crate) image_width: Option<u32>,
    pub(crate) image_height: Option<u32>,
    pub(crate) stored_bytes: u64,
    pub(crate) has_first_inference_output: bool,
    pub(crate) has_second_inference_output: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FailedHistoryInput {
    pub(crate) id: String,
    pub(crate) timestamp: String,
    pub(crate) capture_source: String,
    pub(crate) processing_profile_id: String,
    pub(crate) processing_profile_name: String,
    pub(crate) settings_snapshot: HistorySettingsSnapshot,
    pub(crate) thumbnail_png: Option<Vec<u8>>,
    pub(crate) image_width: Option<u32>,
    pub(crate) image_height: Option<u32>,
    pub(crate) error_code: String,
    pub(crate) error_message: String,
    pub(crate) error_details: Option<String>,
    pub(crate) raw_inference_html: Option<String>,
    /// The original capture to keep with this failed attempt.
    pub(crate) original: Option<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FailedHistoryRow {
    pub(crate) sequence: i64,
    pub(crate) id: String,
    pub(crate) timestamp: String,
    pub(crate) capture_source: String,
    pub(crate) processing_profile_name: String,
    pub(crate) settings_snapshot: Option<HistorySettingsSnapshot>,
    pub(crate) has_thumbnail: bool,
    pub(crate) has_original: bool,
    pub(crate) match_snippet: Option<String>,
    pub(crate) image_width: Option<u32>,
    pub(crate) image_height: Option<u32>,
    pub(crate) error_code: String,
    pub(crate) error_message: String,
    pub(crate) error_details: Option<String>,
    pub(crate) has_raw_inference_output: bool,
}

impl CompletedHistoryRow {
    fn sequence(&self) -> i64 {
        match self {
            Self::Success(row) => row.sequence,
            Self::Failure(row) => row.sequence,
        }
    }
}

impl FailedHistoryRow {
    fn error_report(&self) -> String {
        let mut details = format!(
            concat!(
                "isoboard capture failed\nTime: {}\nSource: {}",
                "\nProfile: {}\nCode: {}\nError: {}"
            ),
            self.timestamp,
            self.capture_source,
            self.processing_profile_name,
            self.error_code,
            self.error_message,
        );
        if let Some(cause) = &self.error_details {
            details.push_str("\nDetails: ");
            details.push_str(cause);
        }
        details
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(tag = "outcome", rename_all = "lowercase")]
pub(crate) enum CompletedHistoryRow {
    Success(HistorySummaryRow),
    Failure(FailedHistoryRow),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PendingHistoryRow {
    pub(crate) id: String,
    pub(crate) timestamp: String,
    pub(crate) capture_source: String,
    pub(crate) processing_profile_name: String,
    pub(crate) thumbnail_data_url: Option<String>,
    pub(crate) image_width: Option<u32>,
    pub(crate) image_height: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HistoryWindowSnapshot {
    pub(crate) entries: Vec<CompletedHistoryRow>,
    pub(crate) pending: Option<PendingHistoryRow>,
    /// Whether entries older than the last returned row exist.
    pub(crate) has_more: bool,
}

/// Selects a window of completed entries, newest first. `before_sequence`
/// pages towards older entries; `min_sequence` re-reads everything from a
/// known row onwards so the frontend can refresh what it already shows.
/// Every bound applies within `filter`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct HistoryPageRequest {
    pub(crate) before_sequence: Option<i64>,
    pub(crate) min_sequence: Option<i64>,
    pub(crate) limit: usize,
    pub(crate) filter: HistoryFilter,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub(crate) enum HistoryOutcome {
    Success,
    Failure,
}

impl HistoryOutcome {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failure => "failure",
        }
    }
}

/// Narrows the history list. Absent fields do not filter.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HistoryFilter {
    /// Words matched as prefixes against the entry text and profile name.
    pub(crate) query: Option<String>,
    pub(crate) outcome: Option<HistoryOutcome>,
    pub(crate) profile_id: Option<String>,
    /// Earliest timestamp to include, in the stored RFC 3339 UTC format.
    pub(crate) since: Option<String>,
    /// First timestamp to exclude, in the same format as `since`.
    pub(crate) until: Option<String>,
    /// Content every matching result must contain. Failed attempts and
    /// entries stored before detection never match a non-empty list.
    pub(crate) contains: Vec<rendering::ContentTag>,
}

/// Values present in History, so filters only offer choices that match.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HistoryFacets {
    /// Profiles newest first, each with the name of its latest entry.
    pub(crate) profiles: Vec<HistoryProfileFacet>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HistoryProfileFacet {
    pub(crate) id: String,
    pub(crate) name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HistoryPage {
    pub(crate) entries: Vec<CompletedHistoryRow>,
    pub(crate) has_more: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PendingHistory {
    sequence: u64,
    id: String,
    timestamp: String,
    capture_source: String,
    processing_profile_id: String,
    processing_profile_name: String,
    settings_snapshot: HistorySettingsSnapshot,
    thumbnail_png: Option<Vec<u8>>,
    image_width: Option<u32>,
    image_height: Option<u32>,
    /// The capture itself, held only when originals are kept.
    original: Option<Vec<u8>>,
}

/// What a capture's saved result inherits from its pending entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PendingIdentity {
    pub(crate) id: String,
    pub(crate) timestamp: String,
    pub(crate) keep_original: bool,
}

#[derive(Default)]
pub(crate) struct PendingHistoryState {
    current: Mutex<Option<PendingHistory>>,
}

pub(crate) struct PendingHistoryInput<'a> {
    pub(crate) sequence: u64,
    pub(crate) source: &'a str,
    pub(crate) profile_id: &'a str,
    pub(crate) profile_name: &'a str,
    pub(crate) settings_snapshot: &'a HistorySettingsSnapshot,
    pub(crate) screenshot: &'a [u8],
    pub(crate) width: u32,
    pub(crate) height: u32,
    /// Decided at capture start from the user's setting.
    pub(crate) keep_original: bool,
}

impl PendingHistoryState {
    fn start(&self, pending: PendingHistory) {
        *self.current.lock().expect("pending history lock poisoned") = Some(pending);
    }

    fn identity(&self, sequence: u64) -> Option<PendingIdentity> {
        self.current
            .lock()
            .expect("pending history lock poisoned")
            .as_ref()
            .filter(|pending| pending.sequence == sequence)
            .map(|pending| PendingIdentity {
                id: pending.id.clone(),
                timestamp: pending.timestamp.clone(),
                keep_original: pending.original.is_some(),
            })
    }

    fn take(&self, sequence: u64) -> Option<PendingHistory> {
        let mut current = self.current.lock().expect("pending history lock poisoned");
        if current
            .as_ref()
            .is_some_and(|pending| pending.sequence == sequence)
        {
            current.take()
        } else {
            None
        }
    }

    fn snapshot(&self) -> Option<PendingHistoryRow> {
        self.current
            .lock()
            .expect("pending history lock poisoned")
            .as_ref()
            .map(|pending| PendingHistoryRow {
                id: pending.id.clone(),
                timestamp: pending.timestamp.clone(),
                capture_source: pending.capture_source.clone(),
                processing_profile_name: pending.processing_profile_name.clone(),
                thumbnail_data_url: thumbnail_data_url(pending.thumbnail_png.as_deref()),
                image_width: pending.image_width,
                image_height: pending.image_height,
            })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TrayHistoryRow {
    pub(crate) id: String,
    pub(crate) timestamp: String,
    pub(crate) plain_text_preview: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RetentionPolicy {
    pub(crate) max_entries: usize,
    pub(crate) max_bytes: u64,
    /// Originals have their own budget so keeping them never evicts entries.
    pub(crate) max_originals: usize,
    pub(crate) max_original_bytes: u64,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            max_entries: APPLICATION.history.max_entries,
            max_bytes: APPLICATION.history.max_bytes,
            max_originals: APPLICATION.history.max_originals,
            max_original_bytes: APPLICATION.history.max_original_bytes,
        }
    }
}

#[derive(Debug)]
pub(crate) enum HistoryError {
    Io(String),
    Database(String),
    Schema(String),
    InvalidEntry(String),
    NotFound,
}

impl Display for HistoryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(message) => write!(formatter, "history I/O failed: {message}"),
            Self::Database(message) => write!(formatter, "history database failed: {message}"),
            Self::Schema(message) => write!(formatter, "history schema failed: {message}"),
            Self::InvalidEntry(message) => write!(formatter, "invalid history entry: {message}"),
            Self::NotFound => formatter.write_str("history entry was not found"),
        }
    }
}

impl std::error::Error for HistoryError {}

pub(crate) trait HistoryStore: Send + Sync {
    fn insert(&self, input: HistoryEntryInput) -> Result<HistoryEntry, HistoryError>;
    fn list(&self, request: HistoryPageRequest) -> Result<HistoryPage, HistoryError>;
    fn facets(&self) -> Result<HistoryFacets, HistoryError>;
    fn get_preview_html(&self, id: &str) -> Result<String, HistoryError>;
    fn get_thumbnail(&self, id: &str) -> Result<Vec<u8>, HistoryError>;
    fn insert_failure(&self, input: FailedHistoryInput) -> Result<(), HistoryError>;
    fn get_failure(&self, id: &str) -> Result<FailedHistoryRow, HistoryError>;
    fn get_raw_html(&self, id: &str) -> Result<String, HistoryError>;
    fn list_tray(&self) -> Result<Vec<TrayHistoryRow>, HistoryError>;
    fn get(&self, id: &str) -> Result<HistoryEntry, HistoryError>;
    fn delete(&self, id: &str) -> Result<bool, HistoryError>;
    fn clear(&self) -> Result<(), HistoryError>;
    /// Forgets every kept original while leaving the entries themselves.
    fn discard_originals(&self) -> Result<(), HistoryError>;
}

pub(crate) struct SqliteHistoryStore {
    connection: Mutex<Connection>,
    retention: RetentionPolicy,
    originals: Option<OriginalsDirectory>,
}

impl SqliteHistoryStore {
    pub(crate) fn open(path: impl Into<PathBuf>) -> Result<Self, HistoryError> {
        Self::open_with_policy(path.into(), RetentionPolicy::default())
    }

    fn open_with_policy(path: PathBuf, retention: RetentionPolicy) -> Result<Self, HistoryError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| HistoryError::Io(format!("{}: {error}", parent.display())))?;
        }
        let mut connection = Connection::open(&path).map_err(database_error)?;
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(database_error)?;
        restrict_database_file(&path)?;
        // WAL makes each insert a single append instead of a rollback journal
        // rewrite; SQLite creates the -wal and -shm files with the database's
        // permissions. NORMAL sync is durable across app crashes in WAL mode.
        connection
            .pragma_update(None, "journal_mode", "WAL")
            .map_err(database_error)?;
        connection
            .pragma_update(None, "synchronous", "NORMAL")
            .map_err(database_error)?;
        initialize(&mut connection)?;
        let store = Self {
            connection: Mutex::new(connection),
            retention,
            originals: Some(OriginalsDirectory::open(originals_path_for_db(&path))?),
        };
        // Clears files left by a crash between a write and its transaction.
        store.sweep_originals(&*store.lock()?);
        Ok(store)
    }

    #[cfg(test)]
    fn in_memory_with_policy(retention: RetentionPolicy) -> Result<Self, HistoryError> {
        let mut connection = Connection::open_in_memory().map_err(database_error)?;
        initialize(&mut connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
            retention,
            originals: None,
        })
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, HistoryError> {
        self.connection
            .lock()
            .map_err(|_| HistoryError::Database("history database lock was poisoned".into()))
    }

    /// Writes an original before its row commits. The caller holds the
    /// connection lock, so no sweep can run between the write and the commit.
    fn write_original(&self, id: &str, bytes: Option<&[u8]>) -> Option<i64> {
        let (directory, bytes) = (self.originals.as_ref()?, bytes?);
        match directory.write(id, bytes) {
            Ok(()) => i64::try_from(bytes.len()).ok(),
            Err(error) => {
                eprintln!("isoboard: could not keep history original: {error}");
                None
            }
        }
    }

    /// Makes the originals directory match the rows that list an original.
    /// Failures are logged; a later sweep retries them.
    fn sweep_originals(&self, connection: &Connection) {
        let Some(directory) = &self.originals else {
            return;
        };
        let kept = connection
            .prepare("SELECT id FROM history_entries WHERE original_bytes IS NOT NULL")
            .and_then(|mut statement| {
                statement
                    .query_map([], |row| row.get::<_, String>(0))?
                    .collect::<rusqlite::Result<std::collections::HashSet<_>>>()
            });
        let result = match kept {
            Ok(kept) => directory.retain_only(&kept),
            Err(error) => Err(database_error(error)),
        };
        if let Err(error) = result {
            eprintln!("isoboard: could not tidy history originals: {error}");
        }
    }

    #[cfg(test)]
    fn read_original(&self, id: &str) -> Result<Vec<u8>, HistoryError> {
        self.originals
            .as_ref()
            .ok_or(HistoryError::NotFound)?
            .read(id)
    }
}

fn originals_path_for_db(db_path: &Path) -> PathBuf {
    db_path.with_file_name("history-originals")
}

impl HistoryStore for SqliteHistoryStore {
    fn insert(&self, input: HistoryEntryInput) -> Result<HistoryEntry, HistoryError> {
        validate_input(&input, self.retention.max_bytes)?;
        let id = input
            .id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let (thumbnail_png, image_width, image_height) = encode_thumbnail(&input.screenshot_bytes)?;
        let preview_html = make_preview(&input.clipboard_body_html);
        let mut entry = HistoryEntry {
            sequence: 0,
            id,
            timestamp: input.timestamp,
            capture_source: input.capture_source,
            processing_profile_id: input.processing_profile_id,
            processing_profile_name: input.processing_profile_name,
            settings_snapshot: Some(input.settings_snapshot),
            thumbnail_png,
            raw_inference_html: input.raw_inference_html,
            clean_document_html: input.clean_document_html,
            clipboard_body_html: input.clipboard_body_html,
            formatting_body_html: input.formatting_body_html,
            plain_text: input.plain_text,
            first_inference_html: input.first_inference_html,
            second_inference_html: input.second_inference_html,
            preview_html,
            image_width: Some(image_width),
            image_height: Some(image_height),
            stored_bytes: 0,
        };
        entry.stored_bytes = stored_bytes(&entry)?;
        if entry.stored_bytes > self.retention.max_bytes {
            return Err(HistoryError::InvalidEntry(format!(
                "history entry exceeds {} bytes",
                self.retention.max_bytes
            )));
        }

        let mut connection = self.lock()?;
        let original_bytes = self.write_original(
            &entry.id,
            input
                .keep_original
                .then_some(input.screenshot_bytes.as_slice()),
        );
        let result = (|| {
            let transaction = connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(database_error)?;
            entry.sequence = next_sequence(&transaction)?;
            entry.stored_bytes = stored_bytes(&entry)?;
            insert_entry(&transaction, &entry)?;
            set_original_bytes(&transaction, &entry.id, original_bytes)?;
            prune(&transaction, self.retention)?;
            transaction.commit().map_err(database_error)
        })();
        self.sweep_originals(&connection);
        result.map(|()| entry)
    }

    fn list(&self, request: HistoryPageRequest) -> Result<HistoryPage, HistoryError> {
        if request.limit == 0 || request.limit > MAX_HISTORY_PAGE_SIZE {
            return Err(HistoryError::InvalidEntry(format!(
                "history page size must be between 1 and {MAX_HISTORY_PAGE_SIZE}"
            )));
        }
        let filter = FilterSql::new(&request.filter)?;
        let connection = self.lock()?;
        let mut bounds = Vec::new();
        let mut bound_params = Vec::new();
        if let Some(sequence) = request.before_sequence {
            bounds.push("sequence < ?");
            bound_params.push(Value::Integer(sequence));
        }
        if let Some(sequence) = request.min_sequence {
            bounds.push("sequence >= ?");
            bound_params.push(Value::Integer(sequence));
        }
        let sql = format!(
            "SELECT id, timestamp, capture_source, processing_profile_id,
                    processing_profile_name, thumbnail_png IS NOT NULL, sequence,
                    image_width, image_height, plain_text, stored_bytes,
                    first_inference_html IS NOT NULL,
                    second_inference_html IS NOT NULL, error_code, error_message, outcome,
                    settings_snapshot, formatting_body_html IS NOT NULL, error_details,
                    failure_raw_html IS NOT NULL, original_bytes IS NOT NULL, {snippet},
                    content_tags
             FROM {from} {where_clause}
             ORDER BY sequence DESC LIMIT ?",
            snippet = filter.snippet,
            from = filter.from,
            where_clause = filter.where_clause(&bounds),
        );
        let mut params = filter.params.clone();
        params.extend(bound_params);
        params.push(Value::Integer(request.limit as i64));
        let entries = connection
            .prepare_cached(&sql)
            .map_err(database_error)?
            .query_map(
                rusqlite::params_from_iter(params.iter()),
                completed_from_compact_row,
            )
            .map_err(database_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(database_error)?;
        // A refresh whose rows were all deleted still reports older entries.
        let oldest_covered = entries
            .last()
            .map(CompletedHistoryRow::sequence)
            .or(request.min_sequence);
        let has_more = match oldest_covered {
            Some(sequence) => {
                let mut params = filter.params.clone();
                params.push(Value::Integer(sequence));
                connection
                    .prepare_cached(&format!(
                        "SELECT EXISTS(SELECT 1 FROM {} {})",
                        filter.from,
                        filter.where_clause(&["sequence < ?"])
                    ))
                    .map_err(database_error)?
                    .query_row(rusqlite::params_from_iter(params.iter()), |row| row.get(0))
                    .map_err(database_error)?
            }
            None => false,
        };
        Ok(HistoryPage { entries, has_more })
    }

    fn facets(&self) -> Result<HistoryFacets, HistoryError> {
        let connection = self.lock()?;
        // SQLite takes bare columns from the row that holds MAX(sequence), so
        // each profile is named as in its latest entry.
        let profiles = connection
            .prepare_cached(
                "SELECT processing_profile_id, processing_profile_name, MAX(sequence)
                 FROM history_entries GROUP BY processing_profile_id
                 ORDER BY MAX(sequence) DESC",
            )
            .map_err(database_error)?
            .query_map([], |row| {
                Ok(HistoryProfileFacet {
                    id: row.get(0)?,
                    name: row.get(1)?,
                })
            })
            .map_err(database_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(database_error)?;
        Ok(HistoryFacets { profiles })
    }

    fn get_preview_html(&self, id: &str) -> Result<String, HistoryError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| HistoryError::Database("history database lock was poisoned".into()))?;
        connection
            .query_row(
                "SELECT preview_html FROM history_entries WHERE id = ?1 AND outcome = 'success'",
                [id],
                |row| row.get(0),
            )
            .optional()
            .map_err(database_error)?
            .ok_or(HistoryError::NotFound)
    }

    fn get_thumbnail(&self, id: &str) -> Result<Vec<u8>, HistoryError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| HistoryError::Database("history database lock was poisoned".into()))?;
        connection
            .query_row(
                "SELECT thumbnail_png FROM history_entries WHERE id = ?1",
                [id],
                |row| row.get::<_, Option<Vec<u8>>>(0),
            )
            .optional()
            .map_err(database_error)?
            .flatten()
            .ok_or(HistoryError::NotFound)
    }

    fn insert_failure(&self, input: FailedHistoryInput) -> Result<(), HistoryError> {
        validate_field("timestamp", &input.timestamp, MAX_TIMESTAMP_BYTES, false)?;
        validate_field(
            "capture source",
            &input.capture_source,
            MAX_CAPTURE_SOURCE_BYTES,
            false,
        )?;
        validate_field(
            "profile ID",
            &input.processing_profile_id,
            MAX_PROFILE_ID_BYTES,
            false,
        )?;
        validate_field(
            "profile name",
            &input.processing_profile_name,
            MAX_PROFILE_NAME_BYTES,
            false,
        )?;
        let settings_json = serialize_settings_snapshot(&input.settings_snapshot)?;
        if !input
            .id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            || input.id.is_empty()
            || input.id.len() > MAX_ENTRY_ID_BYTES
            || input.error_code.is_empty()
            || input.error_code.len() > 64
            || input
                .raw_inference_html
                .as_ref()
                .is_some_and(|html| html.len() > rendering::MAX_PROVIDER_OUTPUT_BYTES)
            || input.error_message.is_empty()
            || input.error_message.len() > MAX_FAILURE_MESSAGE_BYTES
            || input
                .error_details
                .as_ref()
                .is_some_and(|details| details.len() > MAX_FAILURE_MESSAGE_BYTES)
            || input
                .thumbnail_png
                .as_ref()
                .is_some_and(|png| png.len() > MAX_THUMBNAIL_BYTES)
            || input
                .original
                .as_ref()
                .is_some_and(|original| original.len() > MAX_THUMBNAIL_SOURCE_BYTES)
        {
            return Err(HistoryError::InvalidEntry(
                "failed attempt fields are invalid".into(),
            ));
        }
        let mut connection = self.lock()?;
        let original_bytes = self.write_original(&input.id, input.original.as_deref());
        let result = (|| {
            let transaction = connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(database_error)?;
            transaction
                .execute(
                    "INSERT INTO history_entries (
                    sequence, id, outcome, timestamp, capture_source, processing_profile_id,
                    processing_profile_name, thumbnail_png, image_width, image_height,
                    error_code, error_message, settings_snapshot, error_details, failure_raw_html
                 ) VALUES (?1, ?2, 'failure', ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                    params![
                        next_sequence(&transaction)?,
                        input.id,
                        input.timestamp,
                        input.capture_source,
                        input.processing_profile_id,
                        input.processing_profile_name,
                        input.thumbnail_png,
                        input.image_width,
                        input.image_height,
                        input.error_code,
                        input.error_message,
                        settings_json,
                        input.error_details,
                        input.raw_inference_html,
                    ],
                )
                .map_err(database_error)?;
            set_original_bytes(&transaction, &input.id, original_bytes)?;
            prune(&transaction, self.retention)?;
            transaction.commit().map_err(database_error)
        })();
        self.sweep_originals(&connection);
        result
    }

    fn get_raw_html(&self, id: &str) -> Result<String, HistoryError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| HistoryError::Database("history database lock was poisoned".into()))?;
        let source: Option<String> = connection.query_row(
            "SELECT COALESCE(raw_inference_html, failure_raw_html) FROM history_entries WHERE id = ?1",
            [id], |row| row.get(0),
        ).optional().map_err(database_error)?.flatten();
        source.ok_or(HistoryError::NotFound)
    }

    fn get_failure(&self, id: &str) -> Result<FailedHistoryRow, HistoryError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| HistoryError::Database("history database lock was poisoned".into()))?;
        connection
            .query_row(
                "SELECT id, timestamp, capture_source, processing_profile_name,
                        thumbnail_png IS NOT NULL, image_width, image_height, error_code,
                        error_message, settings_snapshot, error_details,
                        failure_raw_html IS NOT NULL, sequence, original_bytes IS NOT NULL
                 FROM history_entries WHERE id = ?1 AND outcome = 'failure'",
                [id],
                failure_from_row,
            )
            .optional()
            .map_err(database_error)?
            .ok_or(HistoryError::NotFound)
    }

    fn list_tray(&self) -> Result<Vec<TrayHistoryRow>, HistoryError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| HistoryError::Database("history database lock was poisoned".into()))?;
        let mut statement = connection
            .prepare(
                "SELECT id, timestamp, plain_text
                 FROM history_entries WHERE outcome = 'success'
                 ORDER BY sequence DESC LIMIT ?1",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map([TRAY_HISTORY_LIMIT as i64], |row| {
                let plain_text: String = row.get(2)?;
                Ok(TrayHistoryRow {
                    id: row.get(0)?,
                    timestamp: row.get(1)?,
                    plain_text_preview: bounded_text(&plain_text, HISTORY_MENU_PREVIEW_CHARS * 4),
                })
            })
            .map_err(database_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(database_error)
    }

    fn get(&self, id: &str) -> Result<HistoryEntry, HistoryError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| HistoryError::Database("history database lock was poisoned".into()))?;
        connection
            .query_row(
                "SELECT sequence, id, timestamp, capture_source, processing_profile_id,
                        processing_profile_name, thumbnail_png, raw_inference_html,
                        clean_document_html, clipboard_body_html, plain_text,
                        first_inference_html, second_inference_html,
                        preview_html, image_width, image_height, stored_bytes, settings_snapshot, formatting_body_html
                 FROM history_entries WHERE id = ?1 AND outcome = 'success'",
                [id],
                entry_from_row,
            )
            .optional()
            .map_err(database_error)?
            .ok_or(HistoryError::NotFound)
    }

    fn delete(&self, id: &str) -> Result<bool, HistoryError> {
        let connection = self.lock()?;
        let result = connection
            .execute("DELETE FROM history_entries WHERE id = ?1", [id])
            .map(|removed| removed != 0)
            .map_err(database_error);
        self.sweep_originals(&connection);
        result
    }

    fn clear(&self) -> Result<(), HistoryError> {
        let connection = self.lock()?;
        let result = connection
            .execute("DELETE FROM history_entries", [])
            .map(|_| ())
            .map_err(database_error);
        self.sweep_originals(&connection);
        result
    }

    fn discard_originals(&self) -> Result<(), HistoryError> {
        let connection = self.lock()?;
        let result = connection
            .execute(
                "UPDATE history_entries SET original_bytes = NULL WHERE original_bytes IS NOT NULL",
                [],
            )
            .map(|_| ())
            .map_err(database_error);
        self.sweep_originals(&connection);
        result
    }
}

/// Marks the start and end of each search match inside `match_snippet`.
/// Private-use characters cannot occur in extracted text.
const SEARCH_MATCH_START: char = '\u{E000}';
const SEARCH_MATCH_END: char = '\u{E001}';
const MAX_SEARCH_QUERY_CHARS: usize = 256;
const MAX_SEARCH_TERMS: usize = 16;

/// The SQL that applies one `HistoryFilter`, shared by a page query and its
/// `has_more` check so both see the same rows.
struct FilterSql {
    from: &'static str,
    snippet: &'static str,
    conditions: Vec<&'static str>,
    params: Vec<Value>,
}

impl FilterSql {
    fn new(filter: &HistoryFilter) -> Result<Self, HistoryError> {
        let mut sql = Self {
            from: "history_entries",
            snippet: "NULL",
            conditions: Vec::new(),
            params: Vec::new(),
        };
        if let Some(expression) = filter.query.as_deref().and_then(search_expression) {
            sql.from = "history_entries JOIN (
                    SELECT rowid AS search_sequence,
                           snippet(history_search, 0, ?, ?, '…', 16) AS search_snippet
                    FROM history_search WHERE history_search MATCH ?
                ) AS search ON search.search_sequence = history_entries.sequence";
            sql.snippet = "search.search_snippet";
            sql.params.extend([
                Value::Text(SEARCH_MATCH_START.into()),
                Value::Text(SEARCH_MATCH_END.into()),
                Value::Text(expression),
            ]);
        }
        if let Some(outcome) = filter.outcome {
            sql.push("outcome = ?", outcome.as_str().to_owned());
        }
        if let Some(profile_id) = &filter.profile_id {
            validate_field("profile filter", profile_id, MAX_PROFILE_ID_BYTES, false)?;
            sql.push("processing_profile_id = ?", profile_id.clone());
        }
        if let Some(since) = &filter.since {
            validate_field("date filter", since, MAX_TIMESTAMP_BYTES, false)?;
            sql.push("timestamp >= ?", since.clone());
        }
        if let Some(until) = &filter.until {
            validate_field("date filter", until, MAX_TIMESTAMP_BYTES, false)?;
            sql.push("timestamp < ?", until.clone());
        }
        if !filter.contains.is_empty() {
            let mask = rendering::ContentTag::mask(&filter.contains);
            sql.conditions.push("content_tags & ? = ?");
            sql.params
                .extend([Value::Integer(mask), Value::Integer(mask)]);
        }
        Ok(sql)
    }

    fn push(&mut self, condition: &'static str, value: String) {
        self.conditions.push(condition);
        self.params.push(Value::Text(value));
    }

    /// The WHERE clause for these conditions plus `extra`, whose parameters
    /// the caller appends after `params`.
    fn where_clause(&self, extra: &[&str]) -> String {
        let conditions: Vec<&str> = self
            .conditions
            .iter()
            .copied()
            .chain(extra.iter().copied())
            .collect();
        if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        }
    }
}

/// Turns typed text into an FTS5 expression that matches every word as a
/// prefix. Each word is quoted, so FTS5 operators and syntax in the input are
/// searched for literally instead of parsed. Words without a letter or digit
/// would not produce a token and are skipped.
fn search_expression(text: &str) -> Option<String> {
    let text: String = text.chars().take(MAX_SEARCH_QUERY_CHARS).collect();
    let terms: Vec<String> = text
        .split_whitespace()
        .filter(|word| word.chars().any(char::is_alphanumeric))
        .take(MAX_SEARCH_TERMS)
        .map(|word| format!("\"{}\"*", word.replace('"', "\"\"")))
        .collect();
    (!terms.is_empty()).then(|| terms.join(" "))
}

fn set_original_bytes(
    connection: &rusqlite::Transaction<'_>,
    id: &str,
    original_bytes: Option<i64>,
) -> Result<(), HistoryError> {
    if original_bytes.is_some() {
        connection
            .execute(
                "UPDATE history_entries SET original_bytes = ?1 WHERE id = ?2",
                params![original_bytes, id],
            )
            .map_err(database_error)?;
    }
    Ok(())
}

fn initialize(connection: &mut Connection) -> Result<(), HistoryError> {
    let version = connection
        .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
        .map_err(database_error)?;
    if version > SCHEMA_VERSION {
        return Err(HistoryError::Schema(format!(
            "database version {version} is newer than supported version {SCHEMA_VERSION}"
        )));
    }
    migrations()
        .to_latest(connection)
        .map_err(|error| HistoryError::Schema(format!("database migration failed: {error}")))
}

fn restrict_database_file(path: &Path) -> Result<(), HistoryError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .map_err(|error| HistoryError::Io(format!("{}: {error}", path.display())))?;
    }
    Ok(())
}

fn insert_entry(
    connection: &rusqlite::Transaction<'_>,
    entry: &HistoryEntry,
) -> Result<(), HistoryError> {
    let stored_bytes = i64::try_from(entry.stored_bytes).map_err(|_| {
        HistoryError::InvalidEntry("stored byte count exceeds SQLite INTEGER".into())
    })?;
    let settings_json = entry
        .settings_snapshot
        .as_ref()
        .map(serialize_settings_snapshot)
        .transpose()?;
    connection
        .execute(
            "INSERT INTO history_entries (
                sequence, id, outcome, timestamp, capture_source, processing_profile_id,
                processing_profile_name, thumbnail_png, raw_inference_html,
                clean_document_html, clipboard_body_html, plain_text,
                first_inference_html, second_inference_html,
                preview_html, image_width, image_height, stored_bytes, settings_snapshot, formatting_body_html,
                content_tags
             ) VALUES (?1, ?2, 'success', ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                       ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
            params![
                entry.sequence,
                entry.id,
                entry.timestamp,
                entry.capture_source,
                entry.processing_profile_id,
                entry.processing_profile_name,
                entry.thumbnail_png,
                entry.raw_inference_html,
                entry.clean_document_html,
                entry.clipboard_body_html,
                entry.plain_text,
                entry.first_inference_html,
                entry.second_inference_html,
                entry.preview_html,
                entry.image_width,
                entry.image_height,
                stored_bytes,
                settings_json,
                entry.formatting_body_html,
                rendering::content_tags(&entry.clipboard_body_html),
            ],
        )
        .map(|_| ())
        .map_err(database_error)
}

fn next_sequence(connection: &rusqlite::Transaction<'_>) -> Result<i64, HistoryError> {
    connection
        .query_row(
            "SELECT COALESCE(MAX(sequence), 0) + 1 FROM history_entries",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(database_error)
}

fn prune(
    connection: &rusqlite::Transaction<'_>,
    policy: RetentionPolicy,
) -> Result<(), HistoryError> {
    let max_entries = i64::try_from(policy.max_entries)
        .map_err(|_| HistoryError::InvalidEntry("history entry limit is too large".into()))?;
    connection
        .execute(
            "DELETE FROM history_entries WHERE outcome = 'success' AND id NOT IN
             (SELECT id FROM history_entries WHERE outcome = 'success' ORDER BY sequence DESC LIMIT ?1)",
            [max_entries],
        )
        .map_err(database_error)?;
    connection
        .execute(
            "DELETE FROM history_entries WHERE outcome = 'failure' AND id NOT IN
             (SELECT id FROM history_entries WHERE outcome = 'failure' ORDER BY sequence DESC LIMIT ?1)",
            [MAX_FAILED_ATTEMPTS as i64],
        )
        .map_err(database_error)?;

    // Keep the newest entries whose running size fits the budget, which is the
    // same as removing the oldest entries until the total fits.
    connection
        .execute(
            "DELETE FROM history_entries WHERE id IN (
                SELECT id FROM (
                    SELECT id, SUM(stored_bytes + COALESCE(length(CAST(failure_raw_html AS BLOB)), 0))
                        OVER (ORDER BY sequence DESC) AS retained_bytes
                    FROM history_entries
                ) WHERE retained_bytes > ?1
             )",
            [i64::try_from(policy.max_bytes).unwrap_or(i64::MAX)],
        )
        .map_err(database_error)?;
    // Originals expire oldest-first by their own count and size budget; the
    // entry stays and only loses its original. Files follow on the next sweep.
    connection
        .execute(
            "UPDATE history_entries SET original_bytes = NULL WHERE id IN (
                SELECT id FROM (
                    SELECT id,
                           ROW_NUMBER() OVER (ORDER BY sequence DESC) AS kept_count,
                           SUM(original_bytes) OVER (ORDER BY sequence DESC) AS kept_bytes
                    FROM history_entries WHERE original_bytes IS NOT NULL
                ) WHERE kept_count > ?1 OR kept_bytes > ?2
             )",
            params![
                i64::try_from(policy.max_originals).unwrap_or(i64::MAX),
                i64::try_from(policy.max_original_bytes).unwrap_or(i64::MAX)
            ],
        )
        .map_err(database_error)?;
    Ok(())
}

fn stored_bytes(entry: &HistoryEntry) -> Result<u64, HistoryError> {
    [
        entry.id.as_str(),
        entry.timestamp.as_str(),
        entry.capture_source.as_str(),
        entry.processing_profile_id.as_str(),
        entry.processing_profile_name.as_str(),
        entry.raw_inference_html.as_str(),
        entry.clean_document_html.as_str(),
        entry.clipboard_body_html.as_str(),
        entry.plain_text.as_str(),
        entry.preview_html.as_str(),
    ]
    .iter()
    .try_fold(entry.thumbnail_png.len() as u64, |total, value| {
        total
            .checked_add(value.len() as u64)
            .ok_or_else(|| HistoryError::InvalidEntry("stored byte count overflowed".into()))
    })
    .and_then(|total| {
        [
            entry.formatting_body_html.as_deref(),
            entry.first_inference_html.as_deref(),
            entry.second_inference_html.as_deref(),
        ]
        .into_iter()
        .flatten()
        .try_fold(total, |total, value| {
            total
                .checked_add(value.len() as u64)
                .ok_or_else(|| HistoryError::InvalidEntry("stored byte count overflowed".into()))
        })
    })
    .and_then(|total| {
        let settings_bytes = entry
            .settings_snapshot
            .as_ref()
            .map(serialize_settings_snapshot)
            .transpose()?
            .map_or(0, |json| json.len() as u64);
        total
            .checked_add(settings_bytes)
            .ok_or_else(|| HistoryError::InvalidEntry("stored byte count overflowed".into()))
    })
}

fn summary_from_compact_row(row: &Row<'_>) -> rusqlite::Result<HistorySummaryRow> {
    let plain_text: String = row.get(9)?;
    Ok(HistorySummaryRow {
        sequence: row.get(6)?,
        id: row.get(0)?,
        timestamp: row.get(1)?,
        capture_source: row.get(2)?,
        processing_profile_id: row.get(3)?,
        processing_profile_name: row.get(4)?,
        settings_snapshot: settings_from_row(row, 16)?,
        has_thumbnail: row.get(5)?,
        has_original: row.get(20)?,
        match_snippet: row.get(21)?,
        content_tags: rendering::ContentTag::from_mask(row.get::<_, Option<i64>>(22)?.unwrap_or(0)),
        plain_text_preview: bounded_text(&plain_text, HISTORY_MENU_PREVIEW_CHARS * 4),
        has_plain_text: !plain_text.trim().is_empty(),
        can_restore_formatting: row.get(17)?,
        image_width: checked_optional_u32(row, 7)?,
        image_height: checked_optional_u32(row, 8)?,
        stored_bytes: checked_u64(row, 10)?,
        has_first_inference_output: row.get(11)?,
        has_second_inference_output: row.get(12)?,
    })
}

fn completed_from_compact_row(row: &Row<'_>) -> rusqlite::Result<CompletedHistoryRow> {
    match row.get::<_, String>(15)?.as_str() {
        "success" => Ok(CompletedHistoryRow::Success(summary_from_compact_row(row)?)),
        "failure" => Ok(CompletedHistoryRow::Failure(FailedHistoryRow {
            sequence: row.get(6)?,
            id: row.get(0)?,
            timestamp: row.get(1)?,
            capture_source: row.get(2)?,
            processing_profile_name: row.get(4)?,
            settings_snapshot: settings_from_row(row, 16)?,
            has_thumbnail: row.get(5)?,
            has_original: row.get(20)?,
            match_snippet: row.get(21)?,
            image_width: checked_optional_u32(row, 7)?,
            image_height: checked_optional_u32(row, 8)?,
            error_code: row.get(13)?,
            error_message: row.get(14)?,
            error_details: row.get(18)?,
            has_raw_inference_output: row.get(19)?,
        })),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn thumbnail_data_url(thumbnail: Option<&[u8]>) -> Option<String> {
    thumbnail.map(|bytes| {
        format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(bytes)
        )
    })
}

fn failure_from_row(row: &Row<'_>) -> rusqlite::Result<FailedHistoryRow> {
    Ok(FailedHistoryRow {
        sequence: row.get(12)?,
        id: row.get(0)?,
        timestamp: row.get(1)?,
        capture_source: row.get(2)?,
        processing_profile_name: row.get(3)?,
        settings_snapshot: settings_from_row(row, 9)?,
        has_thumbnail: row.get(4)?,
        has_original: row.get(13)?,
        match_snippet: None,
        image_width: checked_optional_u32(row, 5)?,
        image_height: checked_optional_u32(row, 6)?,
        error_code: row.get(7)?,
        error_message: row.get(8)?,
        error_details: row.get(10)?,
        has_raw_inference_output: row.get(11)?,
    })
}

fn entry_from_row(row: &Row<'_>) -> rusqlite::Result<HistoryEntry> {
    Ok(HistoryEntry {
        sequence: row.get(0)?,
        id: row.get(1)?,
        timestamp: row.get(2)?,
        capture_source: row.get(3)?,
        processing_profile_id: row.get(4)?,
        processing_profile_name: row.get(5)?,
        settings_snapshot: settings_from_row(row, 17)?,
        thumbnail_png: row.get(6)?,
        raw_inference_html: row.get(7)?,
        clean_document_html: row.get(8)?,
        clipboard_body_html: row.get(9)?,
        formatting_body_html: row.get(18)?,
        plain_text: row.get(10)?,
        first_inference_html: row.get(11)?,
        second_inference_html: row.get(12)?,
        preview_html: row.get(13)?,
        image_width: checked_optional_u32(row, 14)?,
        image_height: checked_optional_u32(row, 15)?,
        stored_bytes: checked_u64(row, 16)?,
    })
}

/// Reads the display-only settings snapshot. A snapshot that no longer parses
/// is reported as unavailable instead of failing the whole history query.
fn settings_from_row(
    row: &Row<'_>,
    index: usize,
) -> rusqlite::Result<Option<HistorySettingsSnapshot>> {
    Ok(row
        .get::<_, Option<String>>(index)?
        .and_then(|json| match serde_json::from_str(&json) {
            Ok(snapshot) => Some(snapshot),
            Err(error) => {
                eprintln!("isoboard: ignoring unreadable history settings snapshot: {error}");
                None
            }
        }))
}

fn serialize_settings_snapshot(snapshot: &HistorySettingsSnapshot) -> Result<String, HistoryError> {
    let json = serde_json::to_string(snapshot).map_err(|error| {
        HistoryError::InvalidEntry(format!("invalid settings snapshot: {error}"))
    })?;
    if json.len() > MAX_SETTINGS_SNAPSHOT_BYTES {
        return Err(HistoryError::InvalidEntry(
            "settings snapshot is too large".into(),
        ));
    }
    Ok(json)
}

fn checked_optional_u32(row: &Row<'_>, index: usize) -> rusqlite::Result<Option<u32>> {
    let value = row.get::<_, Option<i64>>(index)?;
    value
        .map(|value| {
            u32::try_from(value).map_err(|_| {
                rusqlite::Error::FromSqlConversionFailure(
                    index,
                    rusqlite::types::Type::Integer,
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "invalid history image dimension",
                    )),
                )
            })
        })
        .transpose()
}

fn checked_u64(row: &Row<'_>, index: usize) -> rusqlite::Result<u64> {
    let value = row.get::<_, i64>(index)?;
    u64::try_from(value).map_err(|_| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Integer,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "negative history byte count",
            )),
        )
    })
}

fn database_error(error: rusqlite::Error) -> HistoryError {
    HistoryError::Database(error.to_string())
}

fn validate_input(input: &HistoryEntryInput, max_entry_bytes: u64) -> Result<(), HistoryError> {
    if let Some(id) = &input.id {
        if safe_menu_id(id).is_none() || id.len() > MAX_ENTRY_ID_BYTES {
            return Err(HistoryError::InvalidEntry("history ID is invalid".into()));
        }
    }
    validate_field("timestamp", &input.timestamp, MAX_TIMESTAMP_BYTES, false)?;
    validate_field(
        "capture source",
        &input.capture_source,
        MAX_CAPTURE_SOURCE_BYTES,
        false,
    )?;
    validate_field(
        "profile ID",
        &input.processing_profile_id,
        MAX_PROFILE_ID_BYTES,
        false,
    )?;
    validate_field(
        "profile name",
        &input.processing_profile_name,
        MAX_PROFILE_NAME_BYTES,
        false,
    )?;
    validate_field(
        "raw inference HTML",
        &input.raw_inference_html,
        rendering::MAX_PROVIDER_OUTPUT_BYTES,
        false,
    )?;
    validate_optional_field(
        "first inference HTML",
        input.first_inference_html.as_deref(),
        rendering::MAX_PROVIDER_OUTPUT_BYTES,
    )?;
    validate_optional_field(
        "second inference HTML",
        input.second_inference_html.as_deref(),
        rendering::MAX_PROVIDER_OUTPUT_BYTES,
    )?;
    validate_field(
        "clean document HTML",
        &input.clean_document_html,
        rendering::MAX_DOCUMENT_BYTES,
        false,
    )?;
    validate_optional_field(
        "formatting baseline HTML",
        input.formatting_body_html.as_deref(),
        rendering::MAX_DOCUMENT_BYTES,
    )?;
    validate_field(
        "clipboard body HTML",
        &input.clipboard_body_html,
        rendering::MAX_DOCUMENT_BYTES,
        false,
    )?;
    validate_field(
        "plain text",
        &input.plain_text,
        rendering::policy::MAX_PLAIN_TEXT_BYTES,
        true,
    )?;
    if input.screenshot_bytes.len() > MAX_THUMBNAIL_SOURCE_BYTES {
        return Err(HistoryError::InvalidEntry(format!(
            "screenshot exceeds {MAX_THUMBNAIL_SOURCE_BYTES} bytes"
        )));
    }
    if max_entry_bytes == 0 {
        return Err(HistoryError::InvalidEntry(
            "history entry byte limit is zero".into(),
        ));
    }
    Ok(())
}

fn validate_optional_field(
    name: &str,
    value: Option<&str>,
    max_bytes: usize,
) -> Result<(), HistoryError> {
    if let Some(value) = value {
        validate_field(name, value, max_bytes, false)?;
    }
    Ok(())
}

fn validate_field(
    name: &str,
    value: &str,
    max_bytes: usize,
    allow_empty: bool,
) -> Result<(), HistoryError> {
    if !allow_empty && value.is_empty() {
        return Err(HistoryError::InvalidEntry(format!("{name} is empty")));
    }
    if value.len() > max_bytes {
        return Err(HistoryError::InvalidEntry(format!(
            "{name} exceeds {max_bytes} bytes"
        )));
    }
    Ok(())
}

fn bounded_text(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_owned();
    }
    let mut end = max_bytes.saturating_sub("…".len());
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &value[..end])
}

fn truncate_html(html: &str, max_bytes: usize) -> String {
    if html.len() <= max_bytes {
        return html.to_owned();
    }

    let mut output = String::new();
    let mut stack = Vec::<String>::new();
    let mut cursor = 0;
    let mut stopped = false;

    while cursor < html.len() && !stopped {
        if html.as_bytes()[cursor] == b'<' {
            let Some(relative_end) = find_tag_end(&html[cursor..]) else {
                break;
            };
            let end = cursor + relative_end;
            let token = &html[cursor..end];
            let (name, closing, self_closing) = tag_shape(token);
            if name.is_empty() {
                break;
            }
            let mut next_stack = stack.clone();
            if closing {
                if next_stack.last().is_some_and(|current| current == &name) {
                    next_stack.pop();
                } else {
                    stopped = true;
                    continue;
                }
            } else if !self_closing && !VOID_TAGS.contains(&name.as_str()) {
                next_stack.push(name.clone());
            }
            let closing_bytes = closing_bytes(&next_stack);
            if output
                .len()
                .saturating_add(token.len())
                .saturating_add(closing_bytes.len())
                > max_bytes
            {
                break;
            }
            output.push_str(token);
            stack = next_stack;
            cursor = end;
        } else {
            let end = html[cursor..]
                .find('<')
                .map_or(html.len(), |relative| cursor + relative);
            let text = &html[cursor..end];
            let closing_bytes = closing_bytes(&stack);
            let available = max_bytes
                .saturating_sub(output.len())
                .saturating_sub(closing_bytes.len());
            if text.len() <= available {
                output.push_str(text);
                cursor = end;
            } else {
                let mut take = available.min(text.len());
                while take > 0 && !text.is_char_boundary(take) {
                    take -= 1;
                }
                output.push_str(&text[..take]);
                stopped = true;
            }
        }
    }

    output.push_str(&closing_bytes(&stack));
    if output.is_empty() && max_bytes >= 10 {
        "<p>…</p>".to_owned()
    } else {
        output
    }
}

fn find_tag_end(input: &str) -> Option<usize> {
    let mut quote = None;
    for (offset, character) in input.char_indices() {
        match (quote, character) {
            (None, '\'' | '"') => quote = Some(character),
            (Some(current), character) if current == character => quote = None,
            (None, '>') => return Some(offset + 1),
            _ => {}
        }
    }
    None
}

fn tag_shape(token: &str) -> (String, bool, bool) {
    let trimmed = token.trim_end_matches('>').trim_end();
    let closing = trimmed.starts_with("</");
    let start = if closing { 2 } else { 1 };
    let name = trimmed[start..]
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    let self_closing = !closing && trimmed.ends_with('/');
    (name, closing, self_closing)
}

fn closing_bytes(stack: &[String]) -> String {
    stack
        .iter()
        .rev()
        .map(|name| format!("</{name}>"))
        .collect()
}

fn make_preview(body_html: &str) -> String {
    let sanitized = rendering::policy::preview_sanitizer()
        .clean(body_html)
        .to_string();
    truncate_html(&sanitized, MAX_PREVIEW_HTML_BYTES)
}

fn encode_thumbnail(source: &[u8]) -> Result<(Vec<u8>, u32, u32), HistoryError> {
    if source.len() > MAX_THUMBNAIL_SOURCE_BYTES {
        return Err(HistoryError::InvalidEntry(format!(
            "thumbnail source exceeds {MAX_THUMBNAIL_SOURCE_BYTES} bytes"
        )));
    }
    let reader = ImageReader::new(Cursor::new(source))
        .with_guessed_format()
        .map_err(|error| HistoryError::InvalidEntry(format!("thumbnail format: {error}")))?;
    let mut reader = reader;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_THUMBNAIL_DIMENSION);
    limits.max_image_height = Some(MAX_THUMBNAIL_DIMENSION);
    limits.max_alloc = Some(MAX_THUMBNAIL_DECODE_BYTES);
    reader.limits(limits);
    let image = reader
        .decode()
        .map_err(|error| HistoryError::InvalidEntry(format!("thumbnail decode: {error}")))?;
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 {
        return Err(HistoryError::InvalidEntry(
            "thumbnail dimensions are empty".into(),
        ));
    }
    if u64::from(width) * u64::from(height) > MAX_THUMBNAIL_PIXELS {
        return Err(HistoryError::InvalidEntry(
            "thumbnail dimensions exceed the pixel limit".into(),
        ));
    }

    let scale = (MAX_THUMBNAIL_EDGE as f32 / width.max(height) as f32).min(1.0);
    let mut target_width = ((width as f32 * scale).round() as u32).max(1);
    let mut target_height = ((height as f32 * scale).round() as u32).max(1);
    let mut thumbnail = image.resize(target_width, target_height, FilterType::Triangle);
    loop {
        let bytes = encode_png(&thumbnail)?;
        if bytes.len() <= MAX_THUMBNAIL_BYTES {
            return Ok((bytes, width, height));
        }
        let longest = target_width.max(target_height);
        if longest <= 32 {
            return Err(HistoryError::InvalidEntry(format!(
                "thumbnail exceeds {} bytes even at minimum size",
                MAX_THUMBNAIL_BYTES
            )));
        }
        target_width = ((target_width as f32 * 0.8).round() as u32).max(1);
        target_height = ((target_height as f32 * 0.8).round() as u32).max(1);
        thumbnail = thumbnail.resize(target_width, target_height, FilterType::Triangle);
    }
}

fn encode_png(image: &DynamicImage) -> Result<Vec<u8>, HistoryError> {
    let mut output = Cursor::new(Vec::new());
    image
        .write_to(&mut output, ImageFormat::Png)
        .map_err(|error| HistoryError::InvalidEntry(format!("thumbnail encode: {error}")))?;
    Ok(output.into_inner())
}

pub(crate) fn db_path_for_config(config_path: &Path) -> PathBuf {
    config_path.with_file_name("history.sqlite3")
}

pub(crate) fn open_for_config(config_path: &Path) -> Result<Arc<dyn HistoryStore>, HistoryError> {
    Ok(Arc::new(SqliteHistoryStore::open(db_path_for_config(
        config_path,
    ))?))
}

pub(crate) fn begin_pending<R: Runtime>(app: &AppHandle<R>, input: PendingHistoryInput<'_>) {
    let thumbnail_png = match encode_thumbnail(input.screenshot) {
        Ok((png, _, _)) => Some(png),
        Err(error) => {
            eprintln!("isoboard: could not prepare pending history thumbnail: {error}");
            None
        }
    };
    app.state::<PendingHistoryState>().start(PendingHistory {
        sequence: input.sequence,
        id: Uuid::new_v4().to_string(),
        timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        capture_source: input.source.into(),
        processing_profile_id: input.profile_id.into(),
        processing_profile_name: input.profile_name.into(),
        settings_snapshot: input.settings_snapshot.clone(),
        thumbnail_png,
        image_width: Some(input.width),
        image_height: Some(input.height),
        original: input.keep_original.then(|| input.screenshot.to_vec()),
    });
    emit_changed(app);
}

pub(crate) fn pending_identity<R: Runtime>(
    app: &AppHandle<R>,
    sequence: u64,
) -> Option<PendingIdentity> {
    app.state::<PendingHistoryState>().identity(sequence)
}

pub(crate) fn complete_pending<R: Runtime>(app: &AppHandle<R>, sequence: u64) {
    app.state::<PendingHistoryState>().take(sequence);
    emit_changed(app);
}

pub(crate) fn cancel_pending<R: Runtime>(app: &AppHandle<R>, sequence: u64) {
    if app.state::<PendingHistoryState>().take(sequence).is_some() {
        emit_changed(app);
    }
}

pub(crate) fn fail_pending<R: Runtime>(
    app: &AppHandle<R>,
    sequence: u64,
    code: &str,
    message: &str,
    details: Option<&str>,
    raw_html: Option<&str>,
) {
    let Some(pending) = app.state::<PendingHistoryState>().take(sequence) else {
        return;
    };
    if let Some(history) = app.try_state::<Arc<dyn HistoryStore>>() {
        let input = FailedHistoryInput {
            id: pending.id,
            timestamp: pending.timestamp,
            capture_source: pending.capture_source,
            processing_profile_id: pending.processing_profile_id,
            processing_profile_name: pending.processing_profile_name,
            settings_snapshot: pending.settings_snapshot,
            thumbnail_png: pending.thumbnail_png,
            image_width: pending.image_width,
            image_height: pending.image_height,
            error_code: code.into(),
            raw_inference_html: raw_html
                .filter(|html| html.len() <= rendering::MAX_PROVIDER_OUTPUT_BYTES)
                .map(str::to_owned),
            error_message: bounded_failure_message(message),
            error_details: details
                .filter(|details| !details.is_empty())
                .map(bounded_failure_message),
            original: pending.original,
        };
        if let Err(error) = history.insert_failure(input) {
            eprintln!("isoboard: could not persist failed capture: {error}");
            emit_history_error(
                app,
                "persistence",
                "Failed capture could not be saved to History.",
            );
        }
    }
    emit_changed(app);
}

fn bounded_failure_message(message: &str) -> String {
    if message.is_empty() {
        return "Capture failed.".into();
    }
    let mut end = message.len().min(MAX_FAILURE_MESSAGE_BYTES);
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    message[..end].to_owned()
}

pub(crate) fn build_tray_menu<R: Runtime>(
    app: &AppHandle<R>,
    store: &dyn HistoryStore,
    config: &crate::config::ConfigSnapshot,
    capture_status: &crate::capture::CaptureStatus,
) -> Result<Menu<R>, String> {
    let rows = store.list_tray().map_err(|error| error.to_string())?;
    let open_item = MenuItemBuilder::with_id("open", "Open isoboard")
        .accelerator(&config.open_shortcut)
        .build(app)
        .map_err(|error| error.to_string())?;
    let status_item = MenuItemBuilder::with_id(
        "capture-status",
        crate::capture_feedback::status_label(capture_status),
    )
    .enabled(false)
    .build(app)
    .map_err(|error| error.to_string())?;
    let capture_busy = matches!(
        capture_status,
        crate::capture::CaptureStatus::Started { .. }
            | crate::capture::CaptureStatus::ImageReady { .. }
            | crate::capture::CaptureStatus::LoadingModel { .. }
            | crate::capture::CaptureStatus::DownloadingModel { .. }
            | crate::capture::CaptureStatus::Processing { .. }
    );
    let capture_shortcut = &config
        .capture_bindings
        .iter()
        .find(|binding| binding.id == crate::config::DEFAULT_BINDING_ID)
        .expect("validated config includes the default capture binding")
        .shortcut;
    let capture_item = MenuItemBuilder::with_id("capture-region", "Capture region")
        .accelerator(capture_shortcut)
        .enabled(!capture_busy && crate::capture::active_region_capture_ready(app))
        .build(app)
        .map_err(|error| error.to_string())?;
    // Profile entries start a one-shot capture. The checkmark marks the active
    // Profile; selecting an entry does not change it.
    let mut profile_builder =
        SubmenuBuilder::with_id(app, "capture-profile", "Capture with profile")
            .enabled(!capture_busy);
    for profile in &config.processing.profiles {
        let item = CheckMenuItemBuilder::with_id(
            format!("{CAPTURE_PROFILE_MENU_PREFIX}{}", profile.id),
            compact_menu_text(&profile.name, TRAY_MENU_PREVIEW_CHARS),
        )
        .checked(profile.id == config.processing.active_profile_id)
        .enabled(!capture_busy && crate::capture::profile_region_capture_ready(app, &profile.id))
        .build(app)
        .map_err(|error| error.to_string())?;
        profile_builder = profile_builder.item(&item);
    }
    let profile_submenu = profile_builder.build().map_err(|error| error.to_string())?;
    let mut history_builder = SubmenuBuilder::with_id(app, "history", "History");
    let now = Local::now();
    let mut has_rows = false;
    for row in rows {
        let Some(id) = safe_menu_id(&row.id) else {
            continue;
        };
        let label = format!(
            "{} · {}",
            format_tray_timestamp(&row.timestamp, now),
            compact_menu_text(&row.plain_text_preview, TRAY_MENU_PREVIEW_CHARS),
        );
        history_builder = history_builder.text(id, label);
        has_rows = true;
    }
    if has_rows {
        history_builder = history_builder.separator();
    }
    let history_submenu = history_builder
        .text("view-all-history", "View all history")
        .build()
        .map_err(|error| error.to_string())?;
    let mut menu = MenuBuilder::new(app)
        .item(&status_item)
        .separator()
        .item(&capture_item)
        .item(&profile_submenu)
        .item(&open_item)
        .item(&history_submenu)
        .separator();
    let ready_update = app
        .try_state::<Arc<crate::updates::UpdateManager>>()
        .and_then(|manager| manager.ready_version());
    if let Some(version) = ready_update {
        let update_item =
            MenuItemBuilder::with_id("install-update", format!("Restart to update to {version}"))
                .enabled(!capture_busy)
                .build(app)
                .map_err(|error| error.to_string())?;
        menu = menu.item(&update_item);
    }
    menu.text("quit", "Quit isoboard")
        .build()
        .map_err(|error| error.to_string())
}

pub(crate) fn refresh_tray_menu<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let config = app
        .state::<crate::config::SharedConfig>()
        .lock()
        .expect("config lock poisoned")
        .snapshot();
    refresh_tray_menu_with_config(app, &config)
}

pub(crate) fn refresh_tray_menu_with_config<R: Runtime>(
    app: &AppHandle<R>,
    config: &crate::config::ConfigSnapshot,
) -> Result<(), String> {
    let Some(store) = app.try_state::<Arc<dyn HistoryStore>>() else {
        return Ok(());
    };
    let Some(tray) = app.tray_by_id("main") else {
        return Ok(());
    };
    let capture_status = app.state::<crate::capture::CaptureCoordinator>().snapshot();
    let menu = build_tray_menu(app, store.inner().as_ref(), config, &capture_status)?;
    crate::tray_menu_icons::set_menu(&tray, menu, capture_status)
}

pub(crate) fn restore_from_tray<R: Runtime>(app: &AppHandle<R>, id: &str) {
    let Some(store) = app.try_state::<Arc<dyn HistoryStore>>() else {
        return;
    };
    let Some(clipboard) = app.try_state::<Arc<dyn ClipboardWriter>>() else {
        return;
    };
    let store = store.inner().clone();
    let clipboard = clipboard.inner().clone();
    let id = id.to_owned();
    let error_app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(error) = restore(store.as_ref(), clipboard.as_ref(), &id).await {
            eprintln!("isoboard: could not restore tray history entry: {error}");
            emit_restore_error(&error_app);
        }
    });
}

pub(crate) fn safe_menu_id(id: &str) -> Option<String> {
    if id.is_empty()
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return None;
    }
    Some(format!("{HISTORY_MENU_PREFIX}{id}"))
}

pub(crate) fn profile_id_from_menu(id: &str) -> Option<&str> {
    id.strip_prefix(CAPTURE_PROFILE_MENU_PREFIX)
        .filter(|profile_id| !profile_id.is_empty())
}

pub(crate) fn history_id_from_menu(id: &str) -> Option<&str> {
    let value = id.strip_prefix(HISTORY_MENU_PREFIX)?;
    safe_menu_id(value).is_some().then_some(value)
}

fn compact_menu_text(value: &str, max_chars: usize) -> String {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut result = normalized.chars().take(max_chars).collect::<String>();
    if normalized.chars().count() > max_chars {
        result.push('…');
    }
    if result.is_empty() {
        "(empty)".into()
    } else {
        result
    }
}

fn format_tray_timestamp(timestamp: &str, now: DateTime<Local>) -> String {
    let Ok(captured) = DateTime::parse_from_rfc3339(timestamp) else {
        return "Unknown time".into();
    };
    let captured = captured.with_timezone(&Local);
    if captured.date_naive() == now.date_naive() {
        captured.format("%H:%M").to_string()
    } else if captured.year() == now.year() {
        captured.format("%-d %b %H:%M").to_string()
    } else {
        captured.format("%-d %b %Y %H:%M").to_string()
    }
}

pub(crate) async fn restore(
    store: &dyn HistoryStore,
    writer: &dyn ClipboardWriter,
    id: &str,
) -> Result<(), String> {
    let entry = store.get(id).map_err(|error| error.to_string())?;
    let plain_text = rendering::plain_text_from_html(&entry.clipboard_body_html)
        .map_err(|error| error.to_string())?;
    writer
        .write(ClipboardPayload {
            html: entry.clipboard_body_html,
            plain_text,
        })
        .await
        .map_err(clipboard_error_message)
}

#[derive(Clone, Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HistoryFormattingPreview {
    pub(crate) preview_html: String,
    pub(crate) has_plain_text: bool,
}

fn formatted_result(
    store: &dyn HistoryStore,
    id: &str,
    settings: crate::profiles::ConversionSettings,
) -> Result<rendering::NormalizedHtml, String> {
    let entry = store.get(id).map_err(|error| error.to_string())?;
    if let Some(snapshot) = &entry.settings_snapshot {
        let restores = snapshot.application_mode
            == crate::profiles::ConversionSettingsApplicationMode::HtmlCleanup
            && entry.formatting_body_html.is_some();
        if !restores
            && ((settings.tables && !snapshot.tables)
                || (settings.colors && !snapshot.colors)
                || (settings.images && !snapshot.images))
        {
            return Err("This option was not retained for this capture.".into());
        }
    }
    rendering::reformat_saved(
        entry
            .formatting_body_html
            .as_deref()
            .unwrap_or(&entry.clipboard_body_html),
        settings,
    )
    .map_err(|error| {
        if error == rendering::RenderingError::EmptyOutput {
            "Nothing to copy with these options. Keep images enabled for an image-only result."
                .into()
        } else {
            error.to_string()
        }
    })
}

pub(crate) async fn copy_formatted(
    store: &dyn HistoryStore,
    writer: &dyn ClipboardWriter,
    id: &str,
    settings: crate::profiles::ConversionSettings,
    plain_text: bool,
) -> Result<(), String> {
    let result = formatted_result(store, id, settings)?;
    if plain_text {
        if result.plain_text.trim().is_empty() {
            return Err("This result has no plain text to copy.".into());
        }
        return writer
            .write_plain_text(result.plain_text)
            .await
            .map_err(clipboard_error_message);
    }
    writer
        .write(ClipboardPayload {
            html: result.body_html,
            plain_text: result.plain_text,
        })
        .await
        .map_err(clipboard_error_message)
}

pub(crate) async fn copy_plain_text(
    store: &dyn HistoryStore,
    writer: &dyn ClipboardWriter,
    id: &str,
) -> Result<(), String> {
    let entry = store.get(id).map_err(|error| error.to_string())?;
    let plain_text = rendering::plain_text_from_html(&entry.clipboard_body_html)
        .map_err(|error| error.to_string())?;
    if plain_text.trim().is_empty() {
        return Err("this result has no plain text to copy".into());
    }
    writer
        .write_plain_text(plain_text)
        .await
        .map_err(clipboard_error_message)
}

pub(crate) async fn copy_debug_html(
    store: &dyn HistoryStore,
    writer: &dyn ClipboardWriter,
    id: &str,
    kind: HistoryDebugKind,
) -> Result<(), String> {
    let source = if matches!(kind, HistoryDebugKind::Raw) {
        store.get_raw_html(id).map_err(|error| error.to_string())?
    } else {
        let entry = store.get(id).map_err(|error| error.to_string())?;
        match kind {
            HistoryDebugKind::Raw => unreachable!(),
            HistoryDebugKind::Clean => entry.clean_document_html,
            HistoryDebugKind::First => entry
                .first_inference_html
                .ok_or_else(|| "first-pass model output is unavailable".to_owned())?,
            HistoryDebugKind::Second => entry
                .second_inference_html
                .ok_or_else(|| "second-pass model output is unavailable".to_owned())?,
        }
    };
    writer
        .write_plain_text(source)
        .await
        .map_err(clipboard_error_message)
}

fn clipboard_error_message(error: ClipboardError) -> String {
    match error {
        ClipboardError::Preparation(error) => error.to_string(),
        ClipboardError::Write => "the clipboard could not be written".into(),
    }
}

fn emit_changed<R: Runtime>(app: &AppHandle<R>) {
    if let Err(error) = HistoryChangedEvent.emit(app) {
        eprintln!("isoboard: could not emit history change: {error}");
    }
}

/// Deletes every kept original, leaving the entries and their thumbnails.
pub(crate) fn discard_originals<R: Runtime>(app: &AppHandle<R>) {
    let Some(store) = app.try_state::<Arc<dyn HistoryStore>>() else {
        return;
    };
    if let Err(error) = store.discard_originals() {
        eprintln!("isoboard: could not discard history originals: {error}");
        emit_history_error(
            app,
            "persistence",
            "Original screenshots could not be deleted.",
        );
    }
    emit_changed(app);
}

pub(crate) fn emit_persistence_error<R: Runtime>(app: &AppHandle<R>) {
    emit_history_error(app, "persistence", "Clipboard history could not be saved.");
}

pub(crate) fn emit_restore_error<R: Runtime>(app: &AppHandle<R>) {
    emit_history_error(app, "restore", "Clipboard history could not be restored.");
}

fn emit_history_error<R: Runtime>(app: &AppHandle<R>, code: &'static str, message: &'static str) {
    if let Err(error) = (HistoryErrorEvent { code, message }).emit(app) {
        eprintln!("isoboard: could not emit history error: {error}");
    }
}

#[tauri::command(rename_all = "camelCase")]
#[specta::specta]
pub(crate) fn list_history(
    state: State<'_, Arc<dyn HistoryStore>>,
    pending: State<'_, PendingHistoryState>,
    before_sequence: Option<i64>,
    min_sequence: Option<i64>,
    limit: u32,
    filter: HistoryFilter,
) -> Result<HistoryWindowSnapshot, String> {
    let page = state
        .list(HistoryPageRequest {
            before_sequence,
            min_sequence,
            limit: limit as usize,
            filter,
        })
        .map_err(|error| error.to_string())?;
    Ok(HistoryWindowSnapshot {
        entries: page.entries,
        pending: pending.snapshot(),
        has_more: page.has_more,
    })
}

#[tauri::command]
#[specta::specta]
pub(crate) fn get_history_facets(
    state: State<'_, Arc<dyn HistoryStore>>,
) -> Result<HistoryFacets, String> {
    state.facets().map_err(|error| error.to_string())
}

/// Returns the rendered preview for one successful entry. Previews are read
/// on selection so listing stays proportional to the number of rows shown.
#[tauri::command(rename_all = "camelCase")]
#[specta::specta]
pub(crate) fn get_history_preview(
    state: State<'_, Arc<dyn HistoryStore>>,
    id: String,
) -> Result<String, String> {
    state
        .get_preview_html(&id)
        .map_err(|error| error.to_string())
}

/// URI scheme that serves stored thumbnails as `<scheme>://localhost/<id>`,
/// so the history list can load images lazily and the webview can cache them.
pub(crate) const THUMBNAIL_PROTOCOL: &str = "history-thumbnail";

/// Answers one thumbnail protocol request. Entry ids are never reused, so a
/// found thumbnail can be cached indefinitely.
pub(crate) fn thumbnail_protocol_response(
    store: &dyn HistoryStore,
    request: &tauri::http::Request<Vec<u8>>,
) -> tauri::http::Response<Vec<u8>> {
    use tauri::http::{header, Response, StatusCode};

    let id = request.uri().path().trim_start_matches('/');
    let valid_id = !id.is_empty()
        && id.len() <= MAX_ENTRY_ID_BYTES
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-');
    let result = if valid_id {
        store.get_thumbnail(id)
    } else {
        Err(HistoryError::NotFound)
    };
    let response = match result {
        Ok(png) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "image/png")
            .header(
                header::CACHE_CONTROL,
                "private, max-age=31536000, immutable",
            )
            .body(png),
        Err(HistoryError::NotFound) => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Vec::new()),
        Err(error) => {
            eprintln!("isoboard: could not read history thumbnail: {error}");
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Vec::new())
        }
    };
    response.expect("static history thumbnail response is valid")
}

#[tauri::command(rename_all = "camelCase")]
#[specta::specta]
pub(crate) async fn copy_failed_history_details(
    state: State<'_, Arc<dyn HistoryStore>>,
    clipboard: State<'_, Arc<dyn ClipboardWriter>>,
    id: String,
) -> Result<(), String> {
    let failure = state.get_failure(&id).map_err(|error| error.to_string())?;
    clipboard
        .write_plain_text(failure.error_report())
        .await
        .map_err(clipboard_error_message)
}

#[tauri::command(rename_all = "camelCase")]
#[specta::specta]
pub(crate) async fn restore_history_entry(
    state: State<'_, Arc<dyn HistoryStore>>,
    clipboard: State<'_, Arc<dyn ClipboardWriter>>,
    id: String,
) -> Result<(), String> {
    restore(state.inner().as_ref(), clipboard.inner().as_ref(), &id).await
}

#[tauri::command(rename_all = "camelCase")]
#[specta::specta]
pub(crate) fn preview_history_formatting(
    state: State<'_, Arc<dyn HistoryStore>>,
    id: String,
    settings: crate::profiles::ProcessingConversionSettings,
) -> Result<HistoryFormattingPreview, String> {
    let result = formatted_result(
        state.inner().as_ref(),
        &id,
        crate::profiles::ConversionSettings {
            tables: settings.tables,
            colors: settings.colors,
            images: settings.images,
        },
    )?;
    Ok(HistoryFormattingPreview {
        preview_html: result.body_html,
        has_plain_text: !result.plain_text.trim().is_empty(),
    })
}

#[tauri::command(rename_all = "camelCase")]
#[specta::specta]
pub(crate) async fn copy_history_formatted(
    state: State<'_, Arc<dyn HistoryStore>>,
    clipboard: State<'_, Arc<dyn ClipboardWriter>>,
    id: String,
    settings: crate::profiles::ProcessingConversionSettings,
    plain_text: bool,
) -> Result<(), String> {
    copy_formatted(
        state.inner().as_ref(),
        clipboard.inner().as_ref(),
        &id,
        crate::profiles::ConversionSettings {
            tables: settings.tables,
            colors: settings.colors,
            images: settings.images,
        },
        plain_text,
    )
    .await
}

#[tauri::command(rename_all = "camelCase")]
#[specta::specta]
pub(crate) async fn copy_history_plain_text(
    state: State<'_, Arc<dyn HistoryStore>>,
    clipboard: State<'_, Arc<dyn ClipboardWriter>>,
    id: String,
) -> Result<(), String> {
    copy_plain_text(state.inner().as_ref(), clipboard.inner().as_ref(), &id).await
}

#[tauri::command(rename_all = "camelCase")]
#[specta::specta]
pub(crate) async fn copy_history_debug_html(
    state: State<'_, Arc<dyn HistoryStore>>,
    clipboard: State<'_, Arc<dyn ClipboardWriter>>,
    id: String,
    kind: HistoryDebugKind,
) -> Result<(), String> {
    copy_debug_html(
        state.inner().as_ref(),
        clipboard.inner().as_ref(),
        &id,
        kind,
    )
    .await
}

#[tauri::command(rename_all = "camelCase")]
#[specta::specta]
pub(crate) fn delete_history_entry(
    app: AppHandle,
    state: State<'_, Arc<dyn HistoryStore>>,
    id: String,
) -> Result<(), String> {
    state.delete(&id).map_err(|error| error.to_string())?;
    emit_changed(&app);
    if let Err(error) = refresh_tray_menu(&app) {
        eprintln!("isoboard: could not refresh history menu: {error}");
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub(crate) fn clear_history(
    app: AppHandle,
    state: State<'_, Arc<dyn HistoryStore>>,
) -> Result<(), String> {
    state.clear().map_err(|error| error.to_string())?;
    emit_changed(&app);
    if let Err(error) = refresh_tray_menu(&app) {
        eprintln!("isoboard: could not refresh history menu: {error}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clipboard::ClipboardFuture;
    use chrono::TimeZone;
    use image::{ImageBuffer, Rgba};
    use std::sync::{Arc, Barrier, Mutex};
    use tempfile::tempdir;

    const ONE_BY_ONE_PNG: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 248, 207, 192, 240,
        31, 0, 5, 0, 1, 255, 137, 153, 61, 29, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];

    fn settings_snapshot() -> HistorySettingsSnapshot {
        HistorySettingsSnapshot {
            first_model: Some(HistoryModelSnapshot {
                provider: "openai_api".into(),
                model: "gpt-test".into(),
            }),
            second_model: None,
            tables: true,
            colors: false,
            images: true,
            application_mode:
                crate::profiles::ConversionSettingsApplicationMode::PromptAndHtmlCleanup,
            additional_rules_applied: false,
            separate_pass_used: false,
        }
    }

    fn input(timestamp: &str, text: &str) -> HistoryEntryInput {
        HistoryEntryInput {
            id: None,
            timestamp: timestamp.into(),
            capture_source: "region".into(),
            processing_profile_id: "default".into(),
            processing_profile_name: "Default".into(),
            settings_snapshot: settings_snapshot(),
            screenshot_bytes: ONE_BY_ONE_PNG.to_vec(),
            keep_original: false,
            raw_inference_html: format!("<p>raw {text}</p>"),
            clean_document_html: format!("<html><body><p>clean {text}</p></body></html>"),
            clipboard_body_html: format!("<p>clean {text}</p>"),
            formatting_body_html: None,
            plain_text: format!("clean {text}"),
            first_inference_html: None,
            second_inference_html: None,
        }
    }

    fn failed_input(id: &str, timestamp: &str) -> FailedHistoryInput {
        FailedHistoryInput {
            id: id.into(),
            timestamp: timestamp.into(),
            capture_source: "region".into(),
            processing_profile_id: "default".into(),
            processing_profile_name: "Default".into(),
            settings_snapshot: settings_snapshot(),
            thumbnail_png: Some(ONE_BY_ONE_PNG.to_vec()),
            image_width: Some(1),
            image_height: Some(1),
            error_code: "Inference".into(),
            raw_inference_html: None,
            error_message: "inference timed out".into(),
            error_details: Some("Inference failed (TimedOut): inference timed out.".into()),
            original: None,
        }
    }

    fn timeline(store: &SqliteHistoryStore) -> Vec<CompletedHistoryRow> {
        store
            .list(HistoryPageRequest {
                limit: MAX_HISTORY_PAGE_SIZE,
                ..HistoryPageRequest::default()
            })
            .unwrap()
            .entries
    }

    fn successful(store: &SqliteHistoryStore) -> Vec<HistorySummaryRow> {
        timeline(store)
            .into_iter()
            .filter_map(|row| match row {
                CompletedHistoryRow::Success(row) => Some(row),
                CompletedHistoryRow::Failure(_) => None,
            })
            .collect()
    }

    fn failed(store: &SqliteHistoryStore) -> Vec<FailedHistoryRow> {
        timeline(store)
            .into_iter()
            .filter_map(|row| match row {
                CompletedHistoryRow::Failure(row) => Some(row),
                CompletedHistoryRow::Success(_) => None,
            })
            .collect()
    }

    #[derive(Default)]
    struct RecordingWriter {
        rich: Mutex<Option<ClipboardPayload>>,
        plain: Mutex<Option<String>>,
    }

    impl ClipboardWriter for RecordingWriter {
        fn write<'a>(&'a self, payload: ClipboardPayload) -> ClipboardFuture<'a> {
            *self.rich.lock().unwrap() = Some(payload);
            Box::pin(async { Ok(()) })
        }

        fn write_plain_text<'a>(&'a self, text: String) -> ClipboardFuture<'a> {
            *self.plain.lock().unwrap() = Some(text);
            Box::pin(async { Ok(()) })
        }
    }

    #[test]
    fn creates_reopens_and_orders_newest_first() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("history.sqlite3");
        {
            let store = SqliteHistoryStore::open(path.clone()).unwrap();
            store
                .insert(input("2026-01-01T00:00:00.000Z", "old"))
                .unwrap();
            store
                .insert(input("2026-01-02T00:00:00.000Z", "new"))
                .unwrap();
        }
        let store = SqliteHistoryStore::open(path).unwrap();
        let rows = successful(&store);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].timestamp, "2026-01-02T00:00:00.000Z");
        assert_eq!(rows[0].settings_snapshot, Some(settings_snapshot()));
        let entry = store.get(&rows[0].id).unwrap();
        assert_eq!(entry.plain_text, "clean new");
        assert_eq!(entry.settings_snapshot, Some(settings_snapshot()));
        assert_eq!((entry.image_width, entry.image_height), (Some(1), Some(1)));
    }

    fn page_ids(page: &HistoryPage) -> Vec<String> {
        page.entries
            .iter()
            .map(|row| match row {
                CompletedHistoryRow::Success(row) => row.id.clone(),
                CompletedHistoryRow::Failure(row) => row.id.clone(),
            })
            .collect()
    }

    #[test]
    fn pages_newest_first_and_refreshes_from_a_known_row() {
        let store = SqliteHistoryStore::in_memory_with_policy(RetentionPolicy::default()).unwrap();
        let ids: Vec<String> = (0..5)
            .map(|index| {
                store
                    .insert(input(
                        &format!("2026-01-0{}T00:00:00.000Z", index + 1),
                        "text",
                    ))
                    .unwrap()
                    .id
            })
            .collect();
        let newest_first: Vec<String> = ids.iter().rev().cloned().collect();

        let first = store
            .list(HistoryPageRequest {
                limit: 2,
                ..HistoryPageRequest::default()
            })
            .unwrap();
        assert_eq!(page_ids(&first), newest_first[..2]);
        assert!(first.has_more);

        let second = store
            .list(HistoryPageRequest {
                before_sequence: Some(first.entries[1].sequence()),
                limit: 5,
                ..HistoryPageRequest::default()
            })
            .unwrap();
        assert_eq!(page_ids(&second), newest_first[2..]);
        assert!(!second.has_more);

        // A refresh from the oldest shown row sees deletions and new rows.
        let oldest_shown = first.entries[1].sequence();
        store.delete(&newest_first[0]).unwrap();
        let added = store
            .insert(input("2026-01-09T00:00:00.000Z", "text"))
            .unwrap();
        let refreshed = store
            .list(HistoryPageRequest {
                min_sequence: Some(oldest_shown),
                limit: 10,
                ..HistoryPageRequest::default()
            })
            .unwrap();
        assert_eq!(page_ids(&refreshed), [added.id, newest_first[1].clone()]);
        assert!(refreshed.has_more);

        // Deleting every shown row still reports the older ones.
        store.delete(&newest_first[1]).unwrap();
        let emptied = store
            .list(HistoryPageRequest {
                min_sequence: Some(oldest_shown + 100),
                limit: 10,
                ..HistoryPageRequest::default()
            })
            .unwrap();
        assert!(emptied.entries.is_empty());
        assert!(emptied.has_more);

        assert!(store
            .list(HistoryPageRequest {
                limit: MAX_HISTORY_PAGE_SIZE + 1,
                ..HistoryPageRequest::default()
            })
            .is_err());
    }

    fn kept_input(timestamp: &str, screenshot: &[u8]) -> HistoryEntryInput {
        HistoryEntryInput {
            keep_original: true,
            screenshot_bytes: screenshot.to_vec(),
            ..input(timestamp, "kept")
        }
    }

    fn original_files(path: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(originals_path_for_db(path))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn keeps_originals_within_their_own_budget() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("history.sqlite3");
        let store = SqliteHistoryStore::open_with_policy(
            path.clone(),
            RetentionPolicy {
                max_originals: 2,
                ..RetentionPolicy::default()
            },
        )
        .unwrap();

        let first = store
            .insert(kept_input("2026-01-01T00:00:00.000Z", ONE_BY_ONE_PNG))
            .unwrap();
        assert_eq!(store.read_original(&first.id).unwrap(), ONE_BY_ONE_PNG);
        assert!(successful(&store)[0].has_original);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode =
                |path: PathBuf| std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode(originals_path_for_db(&path)), 0o700);
            assert_eq!(mode(originals_path_for_db(&path).join(&first.id)), 0o600);
        }

        let unkept = store
            .insert(input("2026-01-02T00:00:00.000Z", "unkept"))
            .unwrap();
        let second = store
            .insert(kept_input("2026-01-03T00:00:00.000Z", ONE_BY_ONE_PNG))
            .unwrap();
        let third = store
            .insert(kept_input("2026-01-04T00:00:00.000Z", ONE_BY_ONE_PNG))
            .unwrap();

        // The oldest original expires; its entry and thumbnail stay.
        let mut expected = vec![second.id.clone(), third.id.clone()];
        expected.sort();
        assert_eq!(original_files(&path), expected);
        let rows = successful(&store);
        assert_eq!(rows.len(), 4);
        let kept: Vec<_> = rows
            .iter()
            .map(|row| (row.id.clone(), row.has_original))
            .collect();
        assert_eq!(
            kept,
            [
                (third.id.clone(), true),
                (second.id.clone(), true),
                (unkept.id, false),
                (first.id.clone(), false),
            ]
        );
        assert!(store.get(&first.id).is_ok());

        store.delete(&second.id).unwrap();
        assert_eq!(original_files(&path), [third.id]);
        store.clear().unwrap();
        assert!(original_files(&path).is_empty());
    }

    #[test]
    fn failed_attempts_keep_originals_and_discarding_keeps_entries() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("history.sqlite3");
        let store = SqliteHistoryStore::open(path.clone()).unwrap();
        store
            .insert_failure(FailedHistoryInput {
                original: Some(ONE_BY_ONE_PNG.to_vec()),
                ..failed_input("attempt-1", "2026-01-01T00:00:00.000Z")
            })
            .unwrap();
        let success = store
            .insert(kept_input("2026-01-02T00:00:00.000Z", ONE_BY_ONE_PNG))
            .unwrap();
        assert!(failed(&store)[0].has_original);
        assert_eq!(original_files(&path).len(), 2);

        store.discard_originals().unwrap();
        assert!(original_files(&path).is_empty());
        assert!(!failed(&store)[0].has_original);
        assert!(!successful(&store)[0].has_original);
        assert!(store.get(&success.id).is_ok());
    }

    #[test]
    fn reopening_removes_originals_without_a_row() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("history.sqlite3");
        let kept = {
            let store = SqliteHistoryStore::open(path.clone()).unwrap();
            store
                .insert(kept_input("2026-01-01T00:00:00.000Z", ONE_BY_ONE_PNG))
                .unwrap()
                .id
        };
        let originals = originals_path_for_db(&path);
        std::fs::write(originals.join("orphan"), b"left by a crash").unwrap();
        std::fs::write(originals.join("orphan.partial"), b"interrupted").unwrap();

        SqliteHistoryStore::open(path.clone()).unwrap();
        assert_eq!(original_files(&path), [kept]);
    }

    fn search(store: &SqliteHistoryStore, filter: HistoryFilter) -> HistoryPage {
        store
            .list(HistoryPageRequest {
                limit: MAX_HISTORY_PAGE_SIZE,
                filter,
                ..HistoryPageRequest::default()
            })
            .unwrap()
    }

    fn matching(query: &str) -> HistoryFilter {
        HistoryFilter {
            query: Some(query.into()),
            ..HistoryFilter::default()
        }
    }

    fn snippet(row: &CompletedHistoryRow) -> Option<&str> {
        match row {
            CompletedHistoryRow::Success(row) => row.match_snippet.as_deref(),
            CompletedHistoryRow::Failure(row) => row.match_snippet.as_deref(),
        }
    }

    #[test]
    fn searches_text_by_word_prefix_and_marks_matches() {
        let store = SqliteHistoryStore::in_memory_with_policy(RetentionPolicy::default()).unwrap();
        let invoice = store
            .insert(input(
                "2026-01-01T00:00:00.000Z",
                "Quarterly Invoice for Café Núñez",
            ))
            .unwrap();
        store
            .insert(input("2026-01-02T00:00:00.000Z", "meeting notes"))
            .unwrap();
        store
            .insert_failure(failed_input("attempt-1", "2026-01-03T00:00:00.000Z"))
            .unwrap();

        let found = search(&store, matching("invo cafe nunez"));
        assert_eq!(page_ids(&found), std::slice::from_ref(&invoice.id));
        assert_eq!(
            snippet(&found.entries[0]),
            Some("clean Quarterly \u{E000}Invoice\u{E001} for \u{E000}Café\u{E001} \u{E000}Núñez\u{E001}")
        );
        assert_eq!(page_ids(&search(&store, matching("timed"))), ["attempt-1"]);
        assert_eq!(search(&store, matching("Default")).entries.len(), 3);
        assert!(search(&store, matching("absent")).entries.is_empty());

        // Blank or punctuation-only text does not filter.
        assert_eq!(search(&store, matching("  -- ")).entries.len(), 3);
        assert!(snippet(&search(&store, matching("")).entries[0]).is_none());

        // FTS5 syntax is searched for literally instead of being parsed.
        for literal in [
            "notes OR invoice",
            "\"",
            "NEAR(notes",
            "notes*",
            "-notes",
            "col:notes",
        ] {
            store
                .list(HistoryPageRequest {
                    limit: 10,
                    filter: matching(literal),
                    ..HistoryPageRequest::default()
                })
                .unwrap_or_else(|error| panic!("{literal}: {error}"));
        }
        assert!(search(&store, matching("notes OR invoice"))
            .entries
            .is_empty());

        store.delete(&invoice.id).unwrap();
        assert!(search(&store, matching("invoice")).entries.is_empty());
        store.clear().unwrap();
        assert!(search(&store, matching("notes")).entries.is_empty());
    }

    #[test]
    fn filters_combine_with_paging() {
        let store = SqliteHistoryStore::in_memory_with_policy(RetentionPolicy::default()).unwrap();
        let mut other_profile = input("2026-01-01T00:00:00.000Z", "report one");
        other_profile.processing_profile_id = "tables".into();
        let tables = store.insert(other_profile).unwrap();
        let reports: Vec<String> = (2..=4)
            .map(|day| {
                store
                    .insert(input(&format!("2026-01-0{day}T00:00:00.000Z"), "report"))
                    .unwrap()
                    .id
            })
            .collect();
        store
            .insert_failure(failed_input("attempt-1", "2026-01-05T00:00:00.000Z"))
            .unwrap();

        let successes = HistoryFilter {
            query: Some("report".into()),
            outcome: Some(HistoryOutcome::Success),
            profile_id: Some("default".into()),
            ..HistoryFilter::default()
        };
        let first = store
            .list(HistoryPageRequest {
                limit: 2,
                filter: successes.clone(),
                ..HistoryPageRequest::default()
            })
            .unwrap();
        assert_eq!(page_ids(&first), [reports[2].clone(), reports[1].clone()]);
        assert!(first.has_more);
        let second = store
            .list(HistoryPageRequest {
                before_sequence: Some(first.entries[1].sequence()),
                limit: 2,
                filter: successes,
                ..HistoryPageRequest::default()
            })
            .unwrap();
        assert_eq!(page_ids(&second), [reports[0].clone()]);
        assert!(
            !second.has_more,
            "the other profile's older entry must not count"
        );

        let failures = HistoryFilter {
            outcome: Some(HistoryOutcome::Failure),
            ..HistoryFilter::default()
        };
        assert_eq!(page_ids(&search(&store, failures)), ["attempt-1"]);
        let recent = HistoryFilter {
            since: Some("2026-01-04T00:00:00.000Z".into()),
            ..HistoryFilter::default()
        };
        assert_eq!(
            page_ids(&search(&store, recent)),
            ["attempt-1".to_owned(), reports[2].clone()]
        );
        // `until` excludes its own instant, so whole days chain without overlap.
        let range = HistoryFilter {
            since: Some("2026-01-01T00:00:00.000Z".into()),
            until: Some("2026-01-03T00:00:00.000Z".into()),
            ..HistoryFilter::default()
        };
        assert_eq!(
            page_ids(&search(&store, range)),
            [reports[0].clone(), tables.id]
        );

        assert_eq!(
            store.facets().unwrap(),
            HistoryFacets {
                profiles: vec![
                    HistoryProfileFacet {
                        id: "default".into(),
                        name: "Default".into()
                    },
                    HistoryProfileFacet {
                        id: "tables".into(),
                        name: "Default".into()
                    },
                ],
            }
        );
    }

    #[test]
    fn tags_content_at_insert_and_filters_by_all_requested_tags() {
        use rendering::ContentTag;

        let store = SqliteHistoryStore::in_memory_with_policy(RetentionPolicy::default()).unwrap();
        let with_body = |timestamp: &str, body: &str| HistoryEntryInput {
            clipboard_body_html: body.into(),
            ..input(timestamp, "tagged")
        };
        let plain = store
            .insert(with_body("2026-01-01T00:00:00.000Z", "<p>plain</p>"))
            .unwrap();
        let table = store
            .insert(with_body(
                "2026-01-02T00:00:00.000Z",
                "<table><tbody><tr><td>1</td></tr></tbody></table>",
            ))
            .unwrap();
        let styled_table = store
            .insert(with_body(
                "2026-01-03T00:00:00.000Z",
                "<table><tbody><tr><td><strong>1</strong></td></tr></tbody></table>",
            ))
            .unwrap();
        store
            .insert_failure(failed_input("attempt-1", "2026-01-04T00:00:00.000Z"))
            .unwrap();
        {
            // An entry stored before detection existed.
            let connection = store.lock().unwrap();
            connection
                .execute(
                    "UPDATE history_entries SET content_tags = NULL WHERE id = ?1",
                    [&plain.id],
                )
                .unwrap();
        }

        let tags: Vec<_> = successful(&store)
            .into_iter()
            .map(|row| (row.id, row.content_tags))
            .collect();
        assert_eq!(
            tags,
            [
                (
                    styled_table.id.clone(),
                    vec![ContentTag::Tables, ContentTag::Styling]
                ),
                (table.id.clone(), vec![ContentTag::Tables]),
                (plain.id, vec![]),
            ]
        );

        let containing = |contains: Vec<ContentTag>| {
            page_ids(&search(
                &store,
                HistoryFilter {
                    contains,
                    ..HistoryFilter::default()
                },
            ))
        };
        assert_eq!(
            containing(vec![ContentTag::Tables]),
            [styled_table.id.clone(), table.id]
        );
        assert_eq!(
            containing(vec![ContentTag::Tables, ContentTag::Styling]),
            [styled_table.id]
        );
        assert!(containing(vec![ContentTag::Code]).is_empty());
        assert_eq!(containing(vec![]).len(), 4);
    }

    #[test]
    fn v8_database_becomes_searchable() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("history.sqlite3");
        let mut connection = Connection::open(&path).unwrap();
        migrations().to_version(&mut connection, 8).unwrap();
        let snapshot = serde_json::to_string(&settings_snapshot()).unwrap();
        for (sequence, id, text, snapshot) in [
            (1, "valid", "legacy invoice", Some(snapshot.as_str())),
            (2, "broken", "legacy receipt", Some("{not json")),
            (3, "missing", "legacy memo", None),
        ] {
            connection
                .execute(
                    "INSERT INTO history_entries (
                        sequence, id, outcome, timestamp, capture_source, processing_profile_id,
                        processing_profile_name, thumbnail_png, raw_inference_html,
                        clean_document_html, clipboard_body_html, plain_text, preview_html,
                        settings_snapshot
                     ) VALUES (?1, ?2, 'success', '2026-01-01T00:00:00.000Z', 'region',
                               'default', 'Default', ?3, '<p>raw</p>', '<p>clean</p>',
                               '<p>clean</p>', ?4, '<p>clean</p>', ?5)",
                    params![sequence, id, ONE_BY_ONE_PNG, text, snapshot],
                )
                .unwrap();
        }
        drop(connection);

        let store = SqliteHistoryStore::open(path).unwrap();
        assert_eq!(search(&store, matching("legacy")).entries.len(), 3);
        assert_eq!(page_ids(&search(&store, matching("receipt"))), ["broken"]);
    }

    #[test]
    fn serves_thumbnails_by_entry_id() {
        let store = SqliteHistoryStore::in_memory_with_policy(RetentionPolicy::default()).unwrap();
        let entry = store
            .insert(input("2026-01-01T00:00:00.000Z", "text"))
            .unwrap();
        let request = |path: &str| {
            tauri::http::Request::builder()
                .uri(format!("{THUMBNAIL_PROTOCOL}://localhost/{path}"))
                .body(Vec::new())
                .unwrap()
        };

        let found = thumbnail_protocol_response(&store, &request(&entry.id));
        assert_eq!(found.status(), 200);
        assert_eq!(found.headers()["content-type"], "image/png");
        assert_eq!(found.body(), &entry.thumbnail_png);

        for path in ["missing-id", "..%2Fhistory.sqlite3", ""] {
            assert_eq!(
                thumbnail_protocol_response(&store, &request(path)).status(),
                404
            );
        }
    }

    #[test]
    fn failed_attempts_persist_in_window_history_but_not_tray_history() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("history.sqlite3");
        let success_id = {
            let store = SqliteHistoryStore::open(path.clone()).unwrap();
            let mut published = input("2026-01-01T00:00:00.000Z", "published");
            published.id = Some("published-attempt".into());
            let success = store.insert(published).unwrap();
            assert_eq!(success.id, "published-attempt");
            store
                .insert_failure(failed_input("attempt-1", "2026-01-02T00:00:00.000Z"))
                .unwrap();
            success.id
        };

        let store = SqliteHistoryStore::open(path).unwrap();
        let failures = failed(&store);
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].id, "attempt-1");
        assert_eq!(failures[0].error_message, "inference timed out");
        assert_eq!(
            failures[0].error_details.as_deref(),
            Some("Inference failed (TimedOut): inference timed out.")
        );
        assert_eq!(
            store.get_failure("attempt-1").unwrap().error_details,
            failures[0].error_details
        );
        assert_eq!(failures[0].settings_snapshot, Some(settings_snapshot()));
        assert_eq!(
            store.get_failure("attempt-1").unwrap().settings_snapshot,
            Some(settings_snapshot())
        );
        assert!(failures[0]
            .error_report()
            .contains("\nDetails: Inference failed (TimedOut): inference timed out."));
        assert!(failures[0].has_thumbnail);
        let timeline = timeline(&store);
        assert!(matches!(&timeline[0], CompletedHistoryRow::Failure(row) if row.id == "attempt-1"));
        assert!(matches!(&timeline[1], CompletedHistoryRow::Success(row) if row.id == success_id));
        assert_eq!(store.list_tray().unwrap().len(), 1);
        assert_eq!(store.list_tray().unwrap()[0].id, success_id);
        assert!(matches!(
            store.get("attempt-1"),
            Err(HistoryError::NotFound)
        ));
        assert!(store.delete("attempt-1").unwrap());
        assert!(failed(&store).is_empty());
        assert_eq!(successful(&store).len(), 1);
    }

    #[test]
    fn failed_attempt_limit_does_not_prune_successful_results() {
        let store = SqliteHistoryStore::in_memory_with_policy(RetentionPolicy::default()).unwrap();
        store
            .insert(input("2026-01-01T00:00:00.000Z", "published"))
            .unwrap();
        for index in 0..MAX_FAILED_ATTEMPTS + 2 {
            store
                .insert_failure(failed_input(
                    &format!("attempt-{index}"),
                    "2026-01-02T00:00:00.000Z",
                ))
                .unwrap();
        }
        let failures = failed(&store);
        assert_eq!(failures.len(), MAX_FAILED_ATTEMPTS);
        assert_eq!(
            failures[0].id,
            format!("attempt-{}", MAX_FAILED_ATTEMPTS + 1)
        );
        assert_eq!(successful(&store).len(), 1);
        assert_eq!(store.list_tray().unwrap().len(), 1);
        store.clear().unwrap();
        assert!(successful(&store).is_empty());
        assert!(failed(&store).is_empty());
    }

    #[test]
    fn completed_outcomes_require_their_respective_payloads() {
        let store = SqliteHistoryStore::in_memory_with_policy(RetentionPolicy::default()).unwrap();
        let connection = store.connection.lock().unwrap();
        for outcome in ["success", "failure"] {
            let result = connection.execute(
                "INSERT INTO history_entries (
                    sequence, id, outcome, timestamp, capture_source,
                    processing_profile_id, processing_profile_name
                 ) VALUES (?1, ?2, ?3, '2026-01-01', 'region', 'default', 'Default')",
                params![if outcome == "success" { 1 } else { 2 }, outcome, outcome],
            );
            assert!(
                result.is_err(),
                "{outcome} row accepted without its payload"
            );
        }
    }

    #[test]
    fn insertion_sequence_wins_when_timestamps_and_ids_do_not() {
        let store = SqliteHistoryStore::in_memory_with_policy(RetentionPolicy::default()).unwrap();
        let first = store
            .insert(input("2026-01-01T00:00:00.000Z", "first"))
            .unwrap();
        let second = store
            .insert(input("2026-01-01T00:00:00.000Z", "second"))
            .unwrap();
        let rows = successful(&store);
        assert_eq!(rows[0].id, second.id);
        assert_eq!(rows[1].id, first.id);
    }

    #[test]
    fn tray_history_shows_only_five_latest_successes() {
        let store = SqliteHistoryStore::in_memory_with_policy(RetentionPolicy::default()).unwrap();
        for index in 0..7 {
            store
                .insert(input(
                    "2026-01-01T00:00:00.000Z",
                    &format!("result-{index}"),
                ))
                .unwrap();
        }
        store
            .insert_failure(failed_input("failure", "2026-01-02T00:00:00.000Z"))
            .unwrap();

        let rows = store.list_tray().unwrap();
        assert_eq!(rows.len(), TRAY_HISTORY_LIMIT);
        assert_eq!(rows[0].plain_text_preview, "clean result-6");
        assert_eq!(rows[4].plain_text_preview, "clean result-2");
        assert_eq!(rows[0].timestamp, "2026-01-01T00:00:00.000Z");
    }

    #[test]
    fn tray_timestamp_uses_local_time_and_adds_date_before_today() {
        let now = Local
            .with_ymd_and_hms(2026, 9, 29, 18, 0, 0)
            .single()
            .unwrap();
        let today = Local
            .with_ymd_and_hms(2026, 9, 29, 14, 5, 0)
            .single()
            .unwrap();
        let yesterday = Local
            .with_ymd_and_hms(2026, 9, 28, 14, 5, 0)
            .single()
            .unwrap();
        let last_year = Local
            .with_ymd_and_hms(2025, 12, 31, 11, 30, 0)
            .single()
            .unwrap();

        assert_eq!(format_tray_timestamp(&today.to_rfc3339(), now), "14:05");
        assert_eq!(
            format_tray_timestamp(&yesterday.to_rfc3339(), now),
            "28 Sep 14:05"
        );
        assert_eq!(
            format_tray_timestamp(&last_year.to_rfc3339(), now),
            "31 Dec 2025 11:30"
        );
    }

    #[test]
    fn image_only_output_can_have_empty_plain_text() {
        let store = SqliteHistoryStore::in_memory_with_policy(RetentionPolicy::default()).unwrap();
        let entry = store
            .insert(HistoryEntryInput {
                plain_text: String::new(),
                ..input("2026-01-01T00:00:00.000Z", "image-only")
            })
            .unwrap();
        assert!(entry.plain_text.is_empty());
        assert!(!successful(&store)[0].has_plain_text);
    }

    #[tokio::test]
    async fn plain_text_copy_derives_full_text_from_saved_html_and_rejects_empty_results() {
        let store = SqliteHistoryStore::in_memory_with_policy(RetentionPolicy::default()).unwrap();
        let mut history_input = input("2026-01-01T00:00:00.000Z", "plain-copy");
        history_input.plain_text = "First line\nSecond line".into();
        history_input.clipboard_body_html = "<p>First line<br>Second line</p>".into();
        let entry = store.insert(history_input).unwrap();
        assert!(successful(&store)[0].has_plain_text);

        let writer = RecordingWriter::default();
        copy_plain_text(&store, &writer, &entry.id).await.unwrap();
        assert_eq!(
            writer.plain.lock().unwrap().as_deref(),
            Some("First line\nSecond line")
        );
        assert!(writer.rich.lock().unwrap().is_none());

        let mut blank_input = input("2026-01-01T00:00:00.000Z", "blank-copy");
        blank_input.plain_text = " \n\t ".into();
        blank_input.clipboard_body_html = "<p> \n\t </p>".into();
        let blank = store.insert(blank_input).unwrap();
        assert!(!successful(&store)[0].has_plain_text);
        assert!(copy_plain_text(&store, &writer, &blank.id).await.is_err());
        assert_eq!(
            writer.plain.lock().unwrap().as_deref(),
            Some("First line\nSecond line")
        );
    }

    #[tokio::test]
    async fn saved_table_copies_derive_clean_text_without_changing_the_saved_result() {
        let store = SqliteHistoryStore::in_memory_with_policy(RetentionPolicy::default()).unwrap();
        let body = "<table><tr><th>Item</th><th>Price</th></tr><tr><td>Apple</td><td>2</td></tr></table><p>A | B │ C</p>";
        let saved_text =
            "─────┬─────\nItem │Price\n─────┼─────\nApple│2    \n─────┴─────\n\nA | B │ C";
        let entry = store
            .insert(HistoryEntryInput {
                clipboard_body_html: body.into(),
                clean_document_html: format!("<html><body>{body}</body></html>"),
                plain_text: saved_text.into(),
                ..input("2026-01-01T00:00:00.000Z", "table-copy")
            })
            .unwrap();
        let writer = RecordingWriter::default();
        let expected = "Item Price\nApple 2\n\nA | B │ C";

        copy_plain_text(&store, &writer, &entry.id).await.unwrap();
        assert_eq!(writer.plain.lock().unwrap().as_deref(), Some(expected));
        assert!(writer.rich.lock().unwrap().is_none());

        restore(&store, &writer, &entry.id).await.unwrap();
        let rich = writer.rich.lock().unwrap().clone().unwrap();
        assert_eq!(rich.html, body);
        assert_eq!(rich.plain_text, expected);
        assert_eq!(store.get(&entry.id).unwrap().plain_text, saved_text);
    }

    #[test]
    fn separate_connections_allocate_unique_sequences_concurrently() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("history.sqlite3");
        let first_store = Arc::new(SqliteHistoryStore::open(path.clone()).unwrap());
        let second_store = Arc::new(SqliteHistoryStore::open(path).unwrap());
        let barrier = Arc::new(Barrier::new(3));

        let first_thread = {
            let store = Arc::clone(&first_store);
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                store
                    .insert(input("2026-01-01T00:00:00.000Z", "first"))
                    .unwrap()
            })
        };
        let second_thread = {
            let store = Arc::clone(&second_store);
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                store
                    .insert(input("2026-01-01T00:00:00.000Z", "second"))
                    .unwrap()
            })
        };
        barrier.wait();
        let first = first_thread.join().unwrap();
        let second = second_thread.join().unwrap();

        assert_ne!(first.id, second.id);
        assert_ne!(first.sequence, second.sequence);
        let rows = successful(&first_store);
        assert_eq!(rows.len(), 2);
        let newest_id = if first.sequence > second.sequence {
            first.id
        } else {
            second.id
        };
        assert_eq!(rows[0].id, newest_id);
    }

    #[test]
    fn retention_prunes_count_and_bytes_transactionally() {
        let store = SqliteHistoryStore::in_memory_with_policy(RetentionPolicy {
            max_entries: 2,
            max_bytes: 1_500,
            ..RetentionPolicy::default()
        })
        .unwrap();
        for (index, timestamp) in [
            "2026-01-01T00:00:00.000Z",
            "2026-01-02T00:00:00.000Z",
            "2026-01-03T00:00:00.000Z",
        ]
        .into_iter()
        .enumerate()
        {
            store
                .insert(input(timestamp, &"x".repeat(150 + index)))
                .unwrap();
        }
        let rows = successful(&store);
        assert_eq!(rows.len(), 1);
        assert!(rows[0].stored_bytes <= 1_500);
    }

    #[test]
    fn thumbnail_respects_edge_byte_bound_and_aspect_ratio() {
        let image = ImageBuffer::from_fn(800, 100, |x, y| {
            Rgba([(x % 251) as u8, (y % 251) as u8, ((x + y) % 251) as u8, 255])
        });
        let mut source = Cursor::new(Vec::new());
        DynamicImage::ImageRgba8(image)
            .write_to(&mut source, ImageFormat::Png)
            .unwrap();
        let (thumbnail, source_width, source_height) =
            encode_thumbnail(&source.into_inner()).unwrap();
        assert!(thumbnail.len() <= MAX_THUMBNAIL_BYTES);
        assert_eq!((source_width, source_height), (800, 100));
        let decoded = image::load_from_memory(&thumbnail).unwrap();
        let (width, height) = decoded.dimensions();
        assert!(width.max(height) <= MAX_THUMBNAIL_EDGE);
        assert!((width as f32 / height as f32 - 8.0).abs() < 0.5);
    }

    #[test]
    fn compact_history_queries_bound_unicode_plain_text() {
        let store = SqliteHistoryStore::in_memory_with_policy(RetentionPolicy::default()).unwrap();
        store
            .insert(HistoryEntryInput {
                plain_text: "🙂".repeat(20_000),
                ..input("2026-01-01T00:00:00.000Z", "unicode")
            })
            .unwrap();

        let tray_row = store.list_tray().unwrap().pop().unwrap();
        assert!(tray_row.plain_text_preview.len() <= HISTORY_MENU_PREVIEW_CHARS * 4);
        assert_eq!(
            tray_row.plain_text_preview.chars().count(),
            HISTORY_MENU_PREVIEW_CHARS
        );
        assert!(tray_row
            .plain_text_preview
            .chars()
            .take(HISTORY_MENU_PREVIEW_CHARS - 1)
            .all(|character| character == '🙂'));
        assert_eq!(tray_row.plain_text_preview.chars().last(), Some('…'));

        let summary_row = successful(&store).pop().unwrap();
        assert!(summary_row.plain_text_preview.len() <= HISTORY_MENU_PREVIEW_CHARS * 4);
        assert_eq!(
            summary_row.plain_text_preview.chars().count(),
            HISTORY_MENU_PREVIEW_CHARS
        );
        assert!(summary_row
            .plain_text_preview
            .chars()
            .take(HISTORY_MENU_PREVIEW_CHARS - 1)
            .all(|character| character == '🙂'));
        assert_eq!(summary_row.plain_text_preview.chars().last(), Some('…'));
    }

    #[test]
    fn history_previews_preserve_plain_text_characters_and_spacing() {
        let store = SqliteHistoryStore::in_memory_with_policy(RetentionPolicy::default()).unwrap();
        let text = "Hotel Price\nNorth 65 €\nA | B │ C ─ D\n┌──┐\n└──┘\n  code  -- keep spacing";
        let entry = store
            .insert(HistoryEntryInput {
                plain_text: text.into(),
                ..input("2026-01-01T00:00:00.000Z", "preview")
            })
            .unwrap();

        assert_eq!(store.get(&entry.id).unwrap().plain_text, text);
        assert_eq!(successful(&store)[0].plain_text_preview, text);
        assert_eq!(store.list_tray().unwrap()[0].plain_text_preview, text);
    }

    #[test]
    fn rich_preview_is_bounded_structurally_and_keeps_sanitized_inline_images() {
        let generated = base64::engine::general_purpose::STANDARD.encode(ONE_BY_ONE_PNG);
        let body = format!(
            r#"<p>before</p><img src="data:image/png;base64,{generated}"><img src="https://external.invalid/image.png"><p>{}</p>"#,
            "x".repeat(MAX_PREVIEW_HTML_BYTES * 2)
        );
        let preview = make_preview(&body);
        assert!(preview.len() <= MAX_PREVIEW_HTML_BYTES);
        assert!(preview.contains("data:image/png;base64,"));
        assert!(!preview.contains("external.invalid"));
        assert!(preview.ends_with("</p>"));
    }

    #[test]
    fn preview_keeps_large_inline_images_and_later_text_without_mutating_input() {
        let image = ImageBuffer::from_fn(256, 128, |x, y| {
            let seed = x
                .wrapping_mul(1_103_515_245)
                .wrapping_add(y.wrapping_mul(12_345));
            Rgba([seed as u8, (seed >> 8) as u8, (seed >> 16) as u8, 255])
        });
        let mut bytes = Cursor::new(Vec::new());
        DynamicImage::ImageRgba8(image)
            .write_to(&mut bytes, ImageFormat::Png)
            .unwrap();
        let encoded = base64::engine::general_purpose::STANDARD.encode(bytes.into_inner());
        let body = format!(
            r#"<div><img src="data:image/png;base64,{encoded}"><img src="data:image/png;base64,{encoded}"><p>Text after both images remains visible.</p></div>"#
        );
        let original = body.clone();
        let preview = make_preview(&body);
        assert!(body.len() > 32 * 1024);
        assert!(body.len() <= MAX_PREVIEW_HTML_BYTES);
        assert_eq!(body, original);
        assert_eq!(preview.matches("data:image/png;base64,").count(), 2);
        assert!(preview.contains(&encoded));
        assert!(preview.contains("Text after both images remains visible."));
        assert!(preview.len() <= MAX_PREVIEW_HTML_BYTES);
    }

    #[test]
    fn oversized_fields_are_rejected_before_thumbnail_work_or_sqlite_insert() {
        let store = SqliteHistoryStore::in_memory_with_policy(RetentionPolicy::default()).unwrap();
        let result = store.insert(HistoryEntryInput {
            raw_inference_html: "x".repeat(rendering::MAX_PROVIDER_OUTPUT_BYTES + 1),
            ..input("2026-01-01T00:00:00.000Z", "bounds")
        });
        assert!(matches!(result, Err(HistoryError::InvalidEntry(_))));
        assert!(successful(&store).is_empty());
    }

    #[test]
    fn baseline_reopens_counts_toward_retention_and_only_cleanup_mode_can_restore_disabled_options()
    {
        let directory = tempdir().unwrap();
        let path = directory.path().join("history.sqlite3");
        let baseline = r#"<table><tr><th style="color: navy">Header</th></tr></table><img src="data:image/png;base64,AA=="><p>Keep</p>"#;
        let mut source = input("2026-01-01T00:00:00.000Z", "baseline");
        source.settings_snapshot.application_mode =
            crate::profiles::ConversionSettingsApplicationMode::HtmlCleanup;
        source.settings_snapshot.tables = false;
        source.settings_snapshot.colors = false;
        source.settings_snapshot.images = false;
        let mut with_baseline = source.clone();
        with_baseline.formatting_body_html = Some(baseline.into());
        let store = SqliteHistoryStore::open(path.clone()).unwrap();
        let original = store.insert(source).unwrap();
        let retained = store.insert(with_baseline.clone()).unwrap();
        assert_eq!(
            retained.stored_bytes - original.stored_bytes,
            baseline.len() as u64
        );
        let id = retained.id.clone();
        drop(store);
        let store = SqliteHistoryStore::open(path).unwrap();
        assert_eq!(
            store.get(&id).unwrap().formatting_body_html.as_deref(),
            Some(baseline)
        );
        assert!(
            successful(&store)
                .iter()
                .find(|row| row.id == id)
                .unwrap()
                .can_restore_formatting
        );
        let restored =
            formatted_result(&store, &id, crate::profiles::ConversionSettings::default()).unwrap();
        assert!(restored.body_html.contains("<table"));
        assert!(restored.body_html.contains("color: navy"));
        assert!(restored.body_html.contains("<img"));
        assert!(formatted_result(
            &store,
            &original.id,
            crate::profiles::ConversionSettings::default()
        )
        .is_err());
        for mode in [
            crate::profiles::ConversionSettingsApplicationMode::Prompt,
            crate::profiles::ConversionSettingsApplicationMode::PromptAndHtmlCleanup,
        ] {
            with_baseline.settings_snapshot.application_mode = mode;
            with_baseline.settings_snapshot.tables = true;
            let entry = store.insert(with_baseline.clone()).unwrap();
            assert!(formatted_result(
                &store,
                &entry.id,
                crate::profiles::ConversionSettings::default()
            )
            .is_err());
            let cleaned = formatted_result(
                &store,
                &entry.id,
                crate::profiles::ConversionSettings {
                    tables: false,
                    colors: false,
                    images: false,
                },
            )
            .unwrap();
            assert!(!cleaned.body_html.contains("<table"));
            assert!(!cleaned.body_html.contains("<img"));
        }
    }

    #[tokio::test]
    async fn formatted_copy_preserves_saved_output_and_rejects_empty_variations() {
        let store = SqliteHistoryStore::in_memory_with_policy(RetentionPolicy::default()).unwrap();
        let mut source = input("2026-01-01T00:00:00.000Z", "formatted");
        source.settings_snapshot.colors = true;
        source.clipboard_body_html = r#"<table><tr><th style="color: navy; background-color: lightblue">Header</th></tr></table><img src="data:image/png;base64,AA==">"#.into();
        let entry = store.insert(source).unwrap();
        let before = store.get(&entry.id).unwrap();
        let writer = RecordingWriter::default();
        copy_formatted(
            &store,
            &writer,
            &entry.id,
            crate::profiles::ConversionSettings {
                tables: false,
                colors: true,
                images: true,
            },
            false,
        )
        .await
        .unwrap();
        let result = writer.rich.lock().unwrap().clone().unwrap();
        assert!(!result.html.contains("<table"));
        assert!(result.html.contains("color: navy"));
        assert!(result.html.contains("data:image/png;base64,AA=="));
        assert!(result.plain_text.contains("Header"));
        copy_formatted(
            &store,
            &writer,
            &entry.id,
            crate::profiles::ConversionSettings {
                tables: true,
                colors: false,
                images: false,
            },
            false,
        )
        .await
        .unwrap();
        let result = writer.rich.lock().unwrap().clone().unwrap();
        assert!(result.html.contains("<table"));
        assert!(!result.html.contains("color:"));
        assert!(!result.html.contains("<img"));
        assert_eq!(
            store.get(&entry.id).unwrap().clipboard_body_html,
            before.clipboard_body_html
        );
        let mut image = input("2026-01-02T00:00:00.000Z", "image");
        image.clipboard_body_html = r#"<img src="data:image/png;base64,AA==">"#.into();
        let image = store.insert(image).unwrap();
        let previous = writer.rich.lock().unwrap().clone().unwrap();
        assert!(copy_formatted(
            &store,
            &writer,
            &image.id,
            crate::profiles::ConversionSettings {
                images: false,
                ..Default::default()
            },
            false
        )
        .await
        .is_err());
        assert_eq!(
            writer.rich.lock().unwrap().as_ref().unwrap().html,
            previous.html
        );
    }

    #[tokio::test]
    async fn failed_raw_output_survives_restart_and_uses_plain_text_debug_copy() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("history.sqlite3");
        let raw = "<script>doNotRun()</script><img data-x1=invalid>é";
        {
            let store = SqliteHistoryStore::open(path.clone()).unwrap();
            let mut input = failed_input("failed-raw", "2026-01-01T00:00:00.000Z");
            input.raw_inference_html = Some(raw.into());
            store.insert_failure(input).unwrap();
        }
        let store = SqliteHistoryStore::open(path).unwrap();
        assert!(
            store
                .get_failure("failed-raw")
                .unwrap()
                .has_raw_inference_output
        );
        assert!(
            matches!(&timeline(&store)[0], CompletedHistoryRow::Failure(row) if row.has_raw_inference_output)
        );
        let writer = RecordingWriter::default();
        copy_debug_html(&store, &writer, "failed-raw", HistoryDebugKind::Raw)
            .await
            .unwrap();
        assert_eq!(writer.plain.lock().unwrap().as_deref(), Some(raw));
        assert!(writer.rich.lock().unwrap().is_none());
        assert!(
            copy_debug_html(&store, &writer, "failed-raw", HistoryDebugKind::Clean)
                .await
                .is_err()
        );
        assert_eq!(writer.plain.lock().unwrap().as_deref(), Some(raw));
    }

    #[test]
    fn failed_raw_output_obeys_size_and_retention_limits() {
        let store = SqliteHistoryStore::in_memory_with_policy(RetentionPolicy {
            max_entries: 20,
            max_bytes: 10,
            ..RetentionPolicy::default()
        })
        .unwrap();
        let mut too_large = failed_input("oversized", "2026-01-01T00:00:00.000Z");
        too_large.raw_inference_html = Some("x".repeat(rendering::MAX_PROVIDER_OUTPUT_BYTES + 1));
        assert!(store.insert_failure(too_large).is_err());
        for id in ["older", "newer"] {
            let mut input = failed_input(id, "2026-01-01T00:00:00.000Z");
            input.raw_inference_html = Some("ééé".into());
            store.insert_failure(input).unwrap();
        }
        assert!(matches!(
            store.get_failure("older"),
            Err(HistoryError::NotFound)
        ));
        assert_eq!(store.get_raw_html("newer").unwrap(), "ééé");
    }

    #[tokio::test]
    async fn restore_and_debug_copy_use_the_stored_clipboard_representations() {
        let store = SqliteHistoryStore::in_memory_with_policy(RetentionPolicy::default()).unwrap();
        let entry = store
            .insert(input("2026-01-01T00:00:00.000Z", "restore"))
            .unwrap();
        let writer = RecordingWriter::default();

        restore(&store, &writer, &entry.id).await.unwrap();
        let rich = writer.rich.lock().unwrap().clone().unwrap();
        assert_eq!(rich.html, "<p>clean restore</p>");
        assert_eq!(rich.plain_text, "clean restore");

        copy_debug_html(&store, &writer, &entry.id, HistoryDebugKind::Raw)
            .await
            .unwrap();
        assert_eq!(
            writer.plain.lock().unwrap().as_deref(),
            Some("<p>raw restore</p>")
        );
        copy_debug_html(&store, &writer, &entry.id, HistoryDebugKind::Clean)
            .await
            .unwrap();
        assert_eq!(
            writer.plain.lock().unwrap().as_deref(),
            Some("<html><body><p>clean restore</p></body></html>")
        );
    }

    #[tokio::test]
    async fn split_stage_outputs_are_persisted_exactly_and_can_be_copied() {
        let store = SqliteHistoryStore::in_memory_with_policy(RetentionPolicy::default()).unwrap();
        let mut history_input = input("2026-01-01T00:00:00.000Z", "split");
        history_input.first_inference_html = Some("<p>first & raw</p>\n".into());
        history_input.second_inference_html = Some("<p>second <em>raw</em></p>".into());
        let entry = store.insert(history_input).unwrap();

        let stored = store.get(&entry.id).unwrap();
        assert_eq!(
            stored.first_inference_html.as_deref(),
            Some("<p>first & raw</p>\n")
        );
        assert_eq!(
            stored.second_inference_html.as_deref(),
            Some("<p>second <em>raw</em></p>")
        );
        let summary = &successful(&store)[0];
        assert!(summary.has_first_inference_output);
        assert!(summary.has_second_inference_output);

        let writer = RecordingWriter::default();
        copy_debug_html(&store, &writer, &entry.id, HistoryDebugKind::First)
            .await
            .unwrap();
        assert_eq!(
            writer.plain.lock().unwrap().as_deref(),
            Some("<p>first & raw</p>\n")
        );
        copy_debug_html(&store, &writer, &entry.id, HistoryDebugKind::Second)
            .await
            .unwrap();
        assert_eq!(
            writer.plain.lock().unwrap().as_deref(),
            Some("<p>second <em>raw</em></p>")
        );
    }

    #[test]
    fn entries_from_removed_providers_or_unreadable_snapshots_still_load() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("history.sqlite3");
        let store = SqliteHistoryStore::open(path.clone()).unwrap();
        let removed = store.insert(input("2026-01-01", "removed")).unwrap();
        let broken = store.insert(input("2026-01-02", "broken")).unwrap();
        drop(store);

        let connection = Connection::open(&path).unwrap();
        let removed_json = serde_json::to_string(&settings_snapshot())
            .unwrap()
            .replace("openai_api", "openai_subscription");
        connection
            .execute(
                "UPDATE history_entries SET settings_snapshot = ?1 WHERE id = ?2",
                rusqlite::params![removed_json, removed.id],
            )
            .unwrap();
        connection
            .execute(
                "UPDATE history_entries SET settings_snapshot = '{\"firstModel\":7}' WHERE id = ?1",
                [&broken.id],
            )
            .unwrap();
        drop(connection);

        let store = SqliteHistoryStore::open(path).unwrap();
        assert_eq!(successful(&store).len(), 2);
        let removed_snapshot = store.get(&removed.id).unwrap().settings_snapshot.unwrap();
        assert_eq!(
            removed_snapshot.first_model.unwrap().provider,
            "openai_subscription"
        );
        assert_eq!(store.get(&broken.id).unwrap().settings_snapshot, None);
    }

    #[test]
    fn v1_database_migrates_without_inventing_stage_outputs() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("history.sqlite3");
        let connection = Connection::open(&path).unwrap();
        connection.execute_batch(SCHEMA_V1).unwrap();
        connection
            .execute(
                "INSERT INTO history_entries (
                    sequence, id, timestamp, capture_source, processing_profile_id,
                    processing_profile_name, thumbnail_png, raw_inference_html,
                    clean_document_html, clipboard_body_html, plain_text,
                    preview_html, stored_bytes
                 ) VALUES (1, 'legacy', '2026-01-01', 'region', 'default', 'Default', ?1,
                           '<p>raw</p>', '<p>clean</p>', '<p>clean</p>', 'clean', '<p>clean</p>', 1)",
                [ONE_BY_ONE_PNG],
            )
            .unwrap();
        connection.pragma_update(None, "user_version", 1).unwrap();
        drop(connection);

        let store = SqliteHistoryStore::open(path).unwrap();
        let entry = store.get("legacy").unwrap();
        assert_eq!(entry.first_inference_html, None);
        assert_eq!(entry.second_inference_html, None);
        assert_eq!(entry.settings_snapshot, None);
        assert!(!successful(&store)[0].has_first_inference_output);
        assert!(!successful(&store)[0].has_second_inference_output);
        assert_eq!(successful(&store)[0].settings_snapshot, None);
    }

    #[test]
    fn v2_database_gains_completed_attempt_storage_without_losing_results() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("history.sqlite3");
        let connection = Connection::open(&path).unwrap();
        connection.execute_batch(SCHEMA_V1).unwrap();
        connection
            .execute_batch(
                "ALTER TABLE history_entries ADD COLUMN first_inference_html TEXT;
                 ALTER TABLE history_entries ADD COLUMN second_inference_html TEXT;",
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO history_entries (
                    sequence, id, timestamp, capture_source, processing_profile_id,
                    processing_profile_name, thumbnail_png, raw_inference_html,
                    clean_document_html, clipboard_body_html, plain_text,
                    preview_html, stored_bytes
                 ) VALUES (1, 'published', '2026-01-01', 'region', 'default', 'Default', ?1,
                           '<p>raw</p>', '<p>clean</p>', '<p>clean</p>', 'clean', '<p>clean</p>', 1)",
                [ONE_BY_ONE_PNG],
            )
            .unwrap();
        connection.pragma_update(None, "user_version", 2).unwrap();
        drop(connection);

        let store = SqliteHistoryStore::open(path).unwrap();
        assert_eq!(successful(&store)[0].id, "published");
        assert_eq!(successful(&store)[0].settings_snapshot, None);
        store
            .insert_failure(failed_input("attempt-1", "2026-01-02T00:00:00.000Z"))
            .unwrap();
        assert_eq!(failed(&store)[0].id, "attempt-1");
    }

    #[test]
    fn v3_database_keeps_existing_outcomes_without_settings_snapshots() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("history.sqlite3");
        let connection = Connection::open(&path).unwrap();
        connection.execute_batch(SCHEMA_V1).unwrap();
        connection
            .execute_batch(
                "ALTER TABLE history_entries ADD COLUMN first_inference_html TEXT;
                 ALTER TABLE history_entries ADD COLUMN second_inference_html TEXT;",
            )
            .unwrap();
        connection.execute_batch(SCHEMA_V3).unwrap();
        connection
            .execute(
                "INSERT INTO history_entries (
                    sequence, id, outcome, timestamp, capture_source, processing_profile_id,
                    processing_profile_name, thumbnail_png, raw_inference_html,
                    clean_document_html, clipboard_body_html, plain_text, preview_html
                 ) VALUES (1, 'published', 'success', '2026-01-01', 'region', 'default',
                           'Default', ?1, '<p>raw</p>', '<p>clean</p>', '<p>clean</p>',
                           'clean', '<p>clean</p>')",
                [ONE_BY_ONE_PNG],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO history_entries (
                    sequence, id, outcome, timestamp, capture_source, processing_profile_id,
                    processing_profile_name, error_code, error_message
                 ) VALUES (2, 'failed', 'failure', '2026-01-02', 'region', 'default',
                           'Default', 'Inference', 'timed out')",
                [],
            )
            .unwrap();
        connection.pragma_update(None, "user_version", 3).unwrap();
        drop(connection);

        let store = SqliteHistoryStore::open(path).unwrap();
        assert_eq!(successful(&store)[0].settings_snapshot, None);
        assert_eq!(failed(&store)[0].settings_snapshot, None);
        assert_eq!(store.get_failure("failed").unwrap().settings_snapshot, None);
        assert_eq!(store.get_failure("failed").unwrap().error_details, None);
        assert!(
            !store
                .get_failure("failed")
                .unwrap()
                .has_raw_inference_output
        );
        assert!(store.get_raw_html("failed").is_err());
        assert!(!store
            .get_failure("failed")
            .unwrap()
            .error_report()
            .contains("\nDetails:"));
        store
            .insert(input("2026-01-03T00:00:00.000Z", "new"))
            .unwrap();
        assert_eq!(
            successful(&store)[0].settings_snapshot,
            Some(settings_snapshot())
        );
    }

    #[test]
    fn migration_is_atomic_when_a_versioned_step_fails() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("history.sqlite3");
        let connection = Connection::open(&path).unwrap();
        connection.execute_batch(SCHEMA_V1).unwrap();
        connection
            .execute(
                "ALTER TABLE history_entries ADD COLUMN second_inference_html TEXT",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO history_entries (
                    sequence, id, timestamp, capture_source, processing_profile_id,
                    processing_profile_name, thumbnail_png, raw_inference_html,
                    clean_document_html, clipboard_body_html, plain_text,
                    preview_html, stored_bytes
                 ) VALUES (1, 'legacy', '2026-01-01', 'region', 'default', 'Default', ?1,
                           '<p>raw</p>', '<p>clean</p>', '<p>clean</p>', 'clean',
                           '<p>clean</p>', 1)",
                [ONE_BY_ONE_PNG],
            )
            .unwrap();
        connection.pragma_update(None, "user_version", 1).unwrap();
        drop(connection);

        let error = SqliteHistoryStore::open(path.clone()).err().unwrap();
        assert!(matches!(error, HistoryError::Schema(_)));

        let connection = Connection::open(path).unwrap();
        assert_eq!(
            connection
                .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
                .unwrap(),
            1
        );
        let columns = connection
            .prepare("PRAGMA table_info(history_entries)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(!columns
            .iter()
            .any(|column| column == "first_inference_html"));
        assert!(columns
            .iter()
            .any(|column| column == "second_inference_html"));
        assert_eq!(
            connection
                .query_row(
                    "SELECT plain_text FROM history_entries WHERE id = 'legacy'",
                    [],
                    |row| row.get::<_, String>(0)
                )
                .unwrap(),
            "clean"
        );
    }

    #[test]
    fn future_database_version_is_rejected_without_migration() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("history.sqlite3");
        let connection = Connection::open(&path).unwrap();
        connection
            .pragma_update(None, "user_version", SCHEMA_VERSION + 1)
            .unwrap();
        drop(connection);

        let error = SqliteHistoryStore::open(path.clone()).err().unwrap();
        assert!(
            matches!(error, HistoryError::Schema(message) if message.contains("newer than supported"))
        );

        let connection = Connection::open(path).unwrap();
        assert_eq!(
            connection
                .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
                .unwrap(),
            SCHEMA_VERSION + 1
        );
    }

    #[test]
    fn config_database_path_is_adjacent_to_config() {
        assert_eq!(
            db_path_for_config(Path::new("/tmp/isoboard/config.toml")),
            PathBuf::from("/tmp/isoboard/history.sqlite3")
        );
    }

    #[cfg(unix)]
    #[test]
    fn sqlite_database_file_is_private() {
        use std::os::unix::fs::PermissionsExt;

        let directory = tempdir().unwrap();
        let path = directory.path().join("history.sqlite3");
        let _store = SqliteHistoryStore::open(path.clone()).unwrap();
        let mode = std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
}
