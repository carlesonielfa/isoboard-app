//! Clipboard preparation and the platform-neutral write boundary.

mod arboard_writer;

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::profiles::{ConversionSettings, ConversionSettingsApplicationMode};
use crate::rendering::{self, RenderingError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ClipboardPayload {
    pub(crate) html: String,
    pub(crate) plain_text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ClipboardError {
    Preparation(RenderingError),
    Write,
}

pub(crate) type ClipboardFuture<'a> =
    Pin<Box<dyn Future<Output = Result<(), ClipboardError>> + Send + 'a>>;

/// Native clipboard access used by capture orchestration.
pub(crate) trait ClipboardWriter: Send + Sync {
    fn write<'a>(&'a self, payload: ClipboardPayload) -> ClipboardFuture<'a>;

    fn write_plain_text<'a>(&'a self, text: String) -> ClipboardFuture<'a>;
}

pub(crate) fn system<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> Arc<dyn ClipboardWriter> {
    Arc::new(arboard_writer::ArboardClipboardWriter::new(app))
}

/// The complete rendering result needed to persist history after clipboard
/// publication. The raw provider value is retained separately from the
/// normalized document so debug actions can inspect both representations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PreparedClipboard {
    pub(crate) raw_inference_html: String,
    pub(crate) first_inference_html: Option<String>,
    pub(crate) second_inference_html: Option<String>,
    pub(crate) payload: ClipboardPayload,
    pub(crate) document_html: String,
    pub(crate) formatting_body_html: Option<String>,
}

#[cfg(test)]
pub(crate) async fn prepare_and_write_with_details(
    writer: &dyn ClipboardWriter,
    input: &str,
    image_bytes: &[u8],
    width: u32,
    height: u32,
) -> Result<PreparedClipboard, ClipboardError> {
    prepare_and_write_with_conversion(
        writer,
        input,
        image_bytes,
        width,
        height,
        ConversionSettings::default(),
        ConversionSettingsApplicationMode::PromptAndHtmlCleanup,
    )
    .await
}

pub(crate) async fn prepare_and_write_with_conversion(
    writer: &dyn ClipboardWriter,
    input: &str,
    image_bytes: &[u8],
    width: u32,
    height: u32,
    settings: ConversionSettings,
    mode: ConversionSettingsApplicationMode,
) -> Result<PreparedClipboard, ClipboardError> {
    let normalized = rendering::normalize_with_conversion(
        input,
        rendering::SourceImage {
            bytes: image_bytes,
            width,
            height,
        },
        settings,
        mode,
    )
    .map_err(ClipboardError::Preparation)?;
    write_normalized(writer, input, normalized).await
}

/// Publish a finalized document without parsing its app-owned embedded images again.
pub(crate) async fn write_normalized(
    writer: &dyn ClipboardWriter,
    raw_inference_html: &str,
    normalized: rendering::NormalizedHtml,
) -> Result<PreparedClipboard, ClipboardError> {
    write_normalized_stages(writer, raw_inference_html, None, None, normalized).await
}

/// Publish a split-pipeline result while retaining each exact provider
/// response for advanced history diagnostics.
pub(crate) async fn write_normalized_with_stages(
    writer: &dyn ClipboardWriter,
    first_inference_html: &str,
    second_inference_html: &str,
    normalized: rendering::NormalizedHtml,
) -> Result<PreparedClipboard, ClipboardError> {
    write_normalized_stages(
        writer,
        second_inference_html,
        Some(first_inference_html),
        Some(second_inference_html),
        normalized,
    )
    .await
}

async fn write_normalized_stages(
    writer: &dyn ClipboardWriter,
    raw_inference_html: &str,
    first_inference_html: Option<&str>,
    second_inference_html: Option<&str>,
    normalized: rendering::NormalizedHtml,
) -> Result<PreparedClipboard, ClipboardError> {
    let prepared = PreparedClipboard {
        raw_inference_html: raw_inference_html.to_owned(),
        first_inference_html: first_inference_html.map(str::to_owned),
        second_inference_html: second_inference_html.map(str::to_owned),
        payload: ClipboardPayload {
            html: normalized.body_html,
            plain_text: normalized.plain_text,
        },
        document_html: normalized.document_html,
        formatting_body_html: normalized.formatting_body_html,
    };
    writer.write(prepared.payload.clone()).await?;
    Ok(prepared)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct FakeWriter {
        writes: AtomicUsize,
        result: Result<(), ClipboardError>,
    }

    impl ClipboardWriter for FakeWriter {
        fn write<'a>(&'a self, _payload: ClipboardPayload) -> ClipboardFuture<'a> {
            self.writes.fetch_add(1, Ordering::Relaxed);
            let result = self.result;
            Box::pin(async move { result })
        }

        fn write_plain_text<'a>(&'a self, _text: String) -> ClipboardFuture<'a> {
            let result = self.result;
            Box::pin(async move { result })
        }
    }

    #[tokio::test]
    async fn normalized_html_is_returned_after_one_successful_write() {
        let writer = FakeWriter {
            writes: AtomicUsize::new(0),
            result: Ok(()),
        };
        let html =
            prepare_and_write_with_details(&writer, "<p>hello <em>world</em></p>", b"", 1, 1)
                .await
                .unwrap()
                .document_html;

        assert_eq!(
            html,
            "<html><body><p>hello <em>world</em></p></body></html>"
        );
        assert_eq!(writer.writes.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn normalization_failure_does_not_call_the_writer() {
        let writer = FakeWriter {
            writes: AtomicUsize::new(0),
            result: Ok(()),
        };

        assert!(matches!(
            prepare_and_write_with_details(&writer, "<script>unsafe</script>", b"", 1, 1).await,
            Err(ClipboardError::Preparation(RenderingError::EmptyOutput))
        ));
        assert_eq!(writer.writes.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn writer_failure_is_returned_after_one_attempt() {
        let writer = FakeWriter {
            writes: AtomicUsize::new(0),
            result: Err(ClipboardError::Write),
        };

        assert_eq!(
            prepare_and_write_with_details(&writer, "<p>hello</p>", b"", 1, 1).await,
            Err(ClipboardError::Write)
        );
        assert_eq!(writer.writes.load(Ordering::Relaxed), 1);
    }
}
