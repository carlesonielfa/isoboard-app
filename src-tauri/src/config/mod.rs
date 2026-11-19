//! Configuration subsystem facade.
//!
//! `types` owns the version-one schema and errors, `storage` owns TOML I/O,
//! `controller` owns transactional state, `watcher` owns external reloads,
//! `tauri` owns native startup/runtime adapters, and `commands` owns IPC.

mod bindings;
pub(crate) mod commands;
mod controller;
pub(crate) mod inference_commands;
mod provider_auth;
mod storage;
mod tauri;
mod types;
mod watcher;

use crate::managed_credentials::ManagedCredentialStore;
use std::sync::Arc;

pub(crate) struct ManagedCredentialsState(pub(crate) Option<Arc<ManagedCredentialStore>>);

pub(crate) use bindings::DEFAULT_BINDING_ID;
pub use controller::SharedConfig;
pub use tauri::{global_shortcut_plugin, setup};
pub(crate) use types::{
    ConfigError, ConfigSnapshot, OverlayPosition, ShortcutRecordingState, UiMode,
};

#[cfg(test)]
#[path = "inference_transaction_tests.rs"]
mod inference_transaction_tests;
