#[derive(Clone, Copy, Debug)]
pub(crate) enum CaptureRequest {
    Region,
}

#[derive(Debug)]
pub(crate) struct CaptureOutcome {
    pub(crate) bytes: Vec<u8>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub(crate) enum CaptureError {
    Cancelled,
    PermissionDenied,
    Unsupported,
    Io(String),
}

/// Supplies encoded image bytes for a capture request.
///
/// The coordinator owns capture sequencing, status events, and image
/// validation. Implementations own only platform interaction and report its
/// result through this contract.
pub(crate) trait CaptureBackend: Send + Sync {
    fn capture(&self, request: CaptureRequest) -> Result<CaptureOutcome, CaptureError>;
}

pub(crate) fn system() -> Box<dyn CaptureBackend> {
    Box::new(platform::PlatformCaptureBackend)
}

#[cfg(target_os = "macos")]
mod platform {
    use std::process::Command;

    use super::CaptureBackend;
    use super::{CaptureError, CaptureOutcome, CaptureRequest};

    pub(super) struct PlatformCaptureBackend;

    impl CaptureBackend for PlatformCaptureBackend {
        fn capture(&self, request: CaptureRequest) -> Result<CaptureOutcome, CaptureError> {
            match request {
                CaptureRequest::Region => self.capture_region(),
            }
        }
    }

    impl PlatformCaptureBackend {
        fn capture_region(&self) -> Result<CaptureOutcome, CaptureError> {
            if !crate::screen_capture_permission::granted() {
                return Err(CaptureError::PermissionDenied);
            }

            let directory =
                tempfile::tempdir().map_err(|error| CaptureError::Io(error.to_string()))?;
            let output_path = directory.path().join("capture.png");
            let output = Command::new("/usr/sbin/screencapture")
                .args(["-i", "-s", "-t", "png", "-x"])
                .arg(&output_path)
                .output()
                .map_err(|error| CaptureError::Io(error.to_string()))?;

            capture_outcome(
                output,
                std::fs::read(output_path),
                crate::screen_capture_permission::granted(),
            )
        }
    }
    fn capture_outcome(
        output: std::process::Output,
        image: std::io::Result<Vec<u8>>,
        permission_granted: bool,
    ) -> Result<CaptureOutcome, CaptureError> {
        if !permission_granted {
            return Err(CaptureError::PermissionDenied);
        }
        let no_image = image
            .as_ref()
            .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound);
        let diagnostic = String::from_utf8_lossy(&output.stderr);
        let diagnostic = diagnostic.trim();
        // The interactive picker can report no selection with exit 0, a quiet
        // exit 1, or its no-image diagnostic. None produces a screenshot file.
        let no_selection = matches!(output.status.code(), Some(0 | 1))
            && output.stdout.is_empty()
            && matches!(diagnostic, "" | "could not create image from display");
        if no_image && no_selection {
            return Err(CaptureError::Cancelled);
        }
        if !output.status.success() {
            return Err(CaptureError::Io("macOS screen capture failed.".into()));
        }
        image
            .map(|bytes| CaptureOutcome { bytes })
            .map_err(|error| CaptureError::Io(error.to_string()))
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::os::unix::process::ExitStatusExt;

        fn output(code: i32, diagnostic: &str) -> std::process::Output {
            std::process::Output {
                status: std::process::ExitStatus::from_raw(code << 8),
                stdout: Vec::new(),
                stderr: diagnostic.as_bytes().to_vec(),
            }
        }

        fn missing_image() -> std::io::Result<Vec<u8>> {
            Err(std::io::ErrorKind::NotFound.into())
        }

        #[test]
        fn dismissed_region_selection_is_cancelled_without_a_screenshot() {
            for (code, diagnostic) in [
                (0, ""),
                (0, "could not create image from display\n"),
                (1, ""),
                (1, "  \n"),
                (1, "could not create image from display\n"),
            ] {
                assert!(
                    matches!(
                        capture_outcome(output(code, diagnostic), missing_image(), true),
                        Err(CaptureError::Cancelled)
                    ),
                    "{code}: {diagnostic}"
                );
            }
        }

        #[test]
        fn permission_process_and_file_errors_are_not_cancellation() {
            assert!(matches!(
                capture_outcome(output(0, ""), missing_image(), false),
                Err(CaptureError::PermissionDenied)
            ));
            assert!(matches!(
                capture_outcome(
                    output(1, "Failure to write image data."),
                    missing_image(),
                    true
                ),
                Err(CaptureError::Io(_))
            ));
            assert!(matches!(
                capture_outcome(
                    output(0, "Failure to write image data."),
                    missing_image(),
                    true
                ),
                Err(CaptureError::Io(_))
            ));
            assert!(matches!(
                capture_outcome(output(2, ""), missing_image(), true),
                Err(CaptureError::Io(_))
            ));
            assert!(matches!(
                capture_outcome(output(1, ""), Ok(vec![1]), true),
                Err(CaptureError::Io(_))
            ));
            assert!(matches!(
                capture_outcome(
                    output(0, ""),
                    Err(std::io::ErrorKind::PermissionDenied.into()),
                    true
                ),
                Err(CaptureError::Io(_))
            ));
            let signal = std::process::Output {
                status: std::process::ExitStatus::from_raw(9),
                stdout: Vec::new(),
                stderr: Vec::new(),
            };
            assert!(matches!(
                capture_outcome(signal, missing_image(), true),
                Err(CaptureError::Io(_))
            ));
            assert_eq!(
                capture_outcome(output(0, ""), Ok(vec![1, 2]), true)
                    .unwrap()
                    .bytes,
                vec![1, 2]
            );
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    use super::CaptureBackend;
    use super::{CaptureError, CaptureOutcome, CaptureRequest};

    pub(super) struct PlatformCaptureBackend;

    impl CaptureBackend for PlatformCaptureBackend {
        fn capture(&self, request: CaptureRequest) -> Result<CaptureOutcome, CaptureError> {
            match request {
                CaptureRequest::Region => Err(CaptureError::Unsupported),
            }
        }
    }
}
