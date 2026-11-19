use std::time::Duration;

use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSAnimatablePropertyContainer, NSAnimationContext, NSView, NSWindow, NSWindowButton,
    NSWorkspace,
};
use tauri::{Runtime, WebviewWindow};

fn native_window<R: Runtime>(window: &WebviewWindow<R>) -> Result<&NSWindow, String> {
    let _main_thread = MainThreadMarker::new()
        .ok_or_else(|| "Native window access requires the main thread.".to_string())?;
    let pointer = window.ns_window().map_err(|error| error.to_string())?;
    // Tauri owns the NSWindow for the lifetime of this handle. AppKit access is
    // confined to the main thread, including the animation proxy.
    unsafe { pointer.cast::<NSWindow>().as_ref() }
        .ok_or_else(|| "Native overlay window is unavailable.".to_string())
}

pub(super) fn cursor_over_window<R: Runtime>(window: &WebviewWindow<R>) -> Result<bool, String> {
    let native = native_window(window)?;
    // Query the cursor in the frameless window's own coordinates, without
    // relying on mouse events, focus, or mixed-display scale conversions.
    let cursor = native.mouseLocationOutsideOfEventStream();
    let size = native.frame().size;
    Ok(cursor.x >= 0.0 && cursor.x < size.width && cursor.y >= 0.0 && cursor.y < size.height)
}

pub(super) async fn titlebar_controls_center_y(
    window: WebviewWindow,
) -> Result<Option<f64>, String> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    window
        .with_webview(move |webview| {
            // with_webview runs on the main thread. Tauri owns both native
            // objects; borrow them only for this coordinate conversion.
            let center = unsafe {
                let native = &*webview.ns_window().cast::<NSWindow>();
                let view = &*webview.inner().cast::<NSView>();
                native
                    .standardWindowButton(NSWindowButton::CloseButton)
                    .filter(|button| !button.isHiddenOrHasHiddenAncestor())
                    .map(|button| {
                        let rect = view.convertRect_fromView(button.bounds(), Some(&button));
                        let bounds = view.bounds();
                        let center_y = rect.origin.y + rect.size.height / 2.0;
                        if view.isFlipped() {
                            center_y - bounds.origin.y
                        } else {
                            bounds.origin.y + bounds.size.height - center_y
                        }
                    })
            };
            let _ = sender.send(center);
        })
        .map_err(|error| error.to_string())?;
    receiver.await.map_err(|error| error.to_string())
}

pub(super) fn set_opacity<R: Runtime>(
    window: &WebviewWindow<R>,
    opacity: f64,
    duration: Duration,
) -> Result<(), String> {
    let native = native_window(window)?;
    let duration = if NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion() {
        Duration::ZERO
    } else {
        duration
    };
    NSAnimationContext::beginGrouping();
    NSAnimationContext::currentContext().setDuration(duration.as_secs_f64());
    // A zero-duration animation also supersedes an in-flight fade when the
    // window is reused for a new capture.
    native.animator().setAlphaValue(opacity);
    NSAnimationContext::endGrouping();
    Ok(())
}
