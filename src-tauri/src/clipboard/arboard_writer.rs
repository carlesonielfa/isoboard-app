//! Shared arboard clipboard adapter.
//!
//! The clipboard object is initialized on the first write and retained for
//! the lifetime of the app. This matters on Linux, where the process owns the
//! selection, and keeps Windows writes serialized as arboard requires.

#[cfg(not(target_os = "macos"))]
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

use super::{ClipboardError, ClipboardFuture, ClipboardPayload, ClipboardWriter};

pub(crate) struct ArboardClipboardWriter<R: tauri::Runtime> {
    #[cfg(target_os = "macos")]
    app: tauri::AppHandle<R>,
    #[cfg(not(target_os = "macos"))]
    _runtime: PhantomData<R>,
    clipboard: Arc<Mutex<Option<arboard::Clipboard>>>,
}

impl<R: tauri::Runtime> ArboardClipboardWriter<R> {
    pub(crate) fn new(_app: tauri::AppHandle<R>) -> Self {
        Self {
            #[cfg(target_os = "macos")]
            app: _app,
            #[cfg(not(target_os = "macos"))]
            _runtime: PhantomData,
            clipboard: Arc::new(Mutex::new(None)),
        }
    }
}

impl<R: tauri::Runtime> ClipboardWriter for ArboardClipboardWriter<R> {
    fn write<'a>(&'a self, payload: ClipboardPayload) -> ClipboardFuture<'a> {
        let clipboard = self.clipboard.clone();

        #[cfg(target_os = "macos")]
        {
            // AppKit/WebView pasteboard serialization stays on Tauri's main
            // thread. Other platforms use a blocking worker below.
            let app = self.app.clone();
            Box::pin(async move {
                let (sender, receiver) = tokio::sync::oneshot::channel();
                app.run_on_main_thread(move || {
                    let result = write_with_arboard(&clipboard, payload);
                    let _ = sender.send(result);
                })
                .map_err(|_| ClipboardError::Write)?;
                receiver.await.map_err(|_| ClipboardError::Write)?
            })
        }

        #[cfg(not(target_os = "macos"))]
        {
            Box::pin(async move {
                tokio::task::spawn_blocking(move || write_with_arboard(&clipboard, payload))
                    .await
                    .map_err(|_| ClipboardError::Write)?
            })
        }
    }

    fn write_plain_text<'a>(&'a self, text: String) -> ClipboardFuture<'a> {
        let clipboard = self.clipboard.clone();

        #[cfg(target_os = "macos")]
        {
            let app = self.app.clone();
            Box::pin(async move {
                let (sender, receiver) = tokio::sync::oneshot::channel();
                app.run_on_main_thread(move || {
                    let result = write_plain_text_with_arboard(&clipboard, text);
                    let _ = sender.send(result);
                })
                .map_err(|_| ClipboardError::Write)?;
                receiver.await.map_err(|_| ClipboardError::Write)?
            })
        }

        #[cfg(not(target_os = "macos"))]
        {
            Box::pin(async move {
                tokio::task::spawn_blocking(move || write_plain_text_with_arboard(&clipboard, text))
                    .await
                    .map_err(|_| ClipboardError::Write)?
            })
        }
    }
}

fn write_with_arboard(
    clipboard: &Mutex<Option<arboard::Clipboard>>,
    payload: ClipboardPayload,
) -> Result<(), ClipboardError> {
    let mut guard = clipboard.lock().map_err(|_| ClipboardError::Write)?;
    if guard.is_none() {
        *guard = Some(arboard::Clipboard::new().map_err(|_| ClipboardError::Write)?);
    }
    guard
        .as_mut()
        .expect("clipboard is initialized above")
        .set_html(payload.html, Some(payload.plain_text))
        .map_err(|_| ClipboardError::Write)
}

fn write_plain_text_with_arboard(
    clipboard: &Mutex<Option<arboard::Clipboard>>,
    text: String,
) -> Result<(), ClipboardError> {
    let mut guard = clipboard.lock().map_err(|_| ClipboardError::Write)?;
    if guard.is_none() {
        *guard = Some(arboard::Clipboard::new().map_err(|_| ClipboardError::Write)?);
    }
    guard
        .as_mut()
        .expect("clipboard is initialized above")
        .set_text(text)
        .map_err(|_| ClipboardError::Write)
}
