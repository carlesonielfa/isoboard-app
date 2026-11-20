//! Native visibility and placement for the transient capture mini window.

use crate::application_config::APPLICATION;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Mutex};
use std::time::{Duration, Instant};

use tauri::{AppHandle, LogicalSize, Manager, PhysicalPosition, Runtime};
use tauri_specta::Event;

use crate::capture::CaptureStatus;
use crate::config::{OverlayPosition, SharedConfig};

mod dismissal;
use dismissal::{Dismissal, DismissalAction};

const WINDOW_LABEL: &str = "capture-overlay";
const WINDOW_WIDTH: f64 = 288.0;
const WINDOW_HEIGHT: f64 = 60.0;

#[derive(Default)]
pub(crate) struct CaptureOverlay {
    generation: AtomicU64,
    presentation_generation: AtomicU64,
    positioned_sequence: AtomicU64,
    opened_sequence: AtomicU64,
    history_entry: Mutex<Option<(u64, String)>>,
    dismissal: Mutex<Dismissal>,
}

#[derive(Clone, serde::Serialize, specta::Type, tauri_specta::Event)]
#[tauri_specta(event_name = "capture-overlay-presentation")]
pub(crate) struct CaptureOverlayPresentation {
    sequence: u64,
    generation: u64,
    leaving: bool,
}

impl CaptureOverlay {
    fn next_generation(&self) -> u64 {
        self.generation.fetch_add(1, Ordering::AcqRel) + 1
    }
}

#[tauri::command]
#[specta::specta]
pub(crate) fn open_capture_overlay_history(app: AppHandle, sequence: u64) -> Result<(), String> {
    let state = app.state::<CaptureOverlay>();
    if state.positioned_sequence.load(Ordering::Acquire) != sequence {
        return Err("This capture is no longer displayed.".into());
    }
    let entry_id = state
        .history_entry
        .lock()
        .map_err(|_| "Overlay History target lock poisoned.".to_string())?
        .as_ref()
        .filter(|(target_sequence, _)| *target_sequence == sequence)
        .map(|(_, id)| id.clone());
    crate::history::open_history_window(&app, entry_id)?;
    if let Some(overlay) = app.get_webview_window(WINDOW_LABEL) {
        overlay.hide().map_err(|error| error.to_string())?;
    }
    state.opened_sequence.store(sequence, Ordering::Release);
    Ok(())
}

fn set_leaving<R: Runtime>(overlay: &tauri::WebviewWindow<R>, sequence: u64, leaving: bool) {
    let generation = overlay
        .state::<CaptureOverlay>()
        .presentation_generation
        .fetch_add(1, Ordering::AcqRel)
        + 1;
    let event = CaptureOverlayPresentation {
        sequence,
        generation,
        leaving,
    };
    if let Err(error) = event.emit(overlay) {
        eprintln!("isoboard: could not update capture overlay presentation: {error}");
    }
    let (opacity, duration) = if leaving {
        (
            0.0,
            Duration::from_millis(APPLICATION.feedback.capture_fade_milliseconds),
        )
    } else {
        (1.0, Duration::ZERO)
    };
    if let Err(error) = crate::window_presentation::set_overlay_opacity(overlay, opacity, duration)
    {
        eprintln!("isoboard: could not update capture overlay opacity: {error}");
    }
}

fn monitor_dismissal<R: Runtime>(app: &AppHandle<R>, sequence: u64, generation: u64) {
    let app = app.clone();
    std::thread::spawn(move || loop {
        let (sender, receiver) = mpsc::channel();
        with_current_overlay(&app, generation, move |overlay| {
            let keep_monitoring = poll_dismissal(&overlay, sequence, generation);
            let _ = sender.send(keep_monitoring);
        });
        // Wait for each UI sample so a busy main thread cannot accumulate polls.
        if receiver.recv() != Ok(true) {
            break;
        }
        std::thread::sleep(Duration::from_millis(
            APPLICATION.feedback.capture_cursor_poll_milliseconds,
        ));
    });
}

fn poll_dismissal<R: Runtime>(
    overlay: &tauri::WebviewWindow<R>,
    sequence: u64,
    generation: u64,
) -> bool {
    if !overlay.is_visible().unwrap_or(false) {
        return false;
    }
    let hovered = crate::window_presentation::cursor_over_window(overlay).unwrap_or(false);
    let state = overlay.state::<CaptureOverlay>();
    let action = {
        let mut clock = state
            .dismissal
            .lock()
            .expect("overlay dismissal lock poisoned");
        if state.generation.load(Ordering::Acquire) != generation {
            return false;
        }
        clock.poll(
            sequence,
            hovered,
            Instant::now(),
            Duration::from_millis(APPLICATION.feedback.capture_exit_grace_milliseconds),
        )
    };
    match action {
        DismissalAction::Wait => {}
        DismissalAction::Fade | DismissalAction::Restore => {
            set_leaving(overlay, sequence, action == DismissalAction::Fade);
        }
        DismissalAction::Hide => {
            if let Err(error) = overlay.hide() {
                eprintln!("isoboard: could not dismiss capture overlay: {error}");
            }
            return false;
        }
        DismissalAction::Stop => return false,
    }
    true
}

fn with_current_overlay<R: Runtime>(
    app: &AppHandle<R>,
    generation: u64,
    action: impl FnOnce(tauri::WebviewWindow<R>) + Send + 'static,
) {
    let handle = app.clone();
    if let Err(error) = app.run_on_main_thread(move || {
        // Recheck after dispatch: a new capture can supersede a queued fade or
        // hide before the main thread executes it.
        if handle
            .try_state::<CaptureOverlay>()
            .is_some_and(|state| state.generation.load(Ordering::Acquire) == generation)
        {
            if let Some(overlay) = handle.get_webview_window(WINDOW_LABEL) {
                action(overlay);
            }
        }
    }) {
        eprintln!("isoboard: could not update capture overlay window: {error}");
    }
}

/// Hides the overlay and forgets the capture it was showing.
pub(crate) fn hide<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<CaptureOverlay>();
    let generation = {
        let mut clock = state
            .dismissal
            .lock()
            .expect("overlay dismissal lock poisoned");
        *clock = Dismissal::default();
        state.next_generation()
    };
    state.positioned_sequence.store(0, Ordering::Release);
    state.opened_sequence.store(0, Ordering::Release);
    *state
        .history_entry
        .lock()
        .expect("overlay History target lock poisoned") = None;
    with_current_overlay(app, generation, |overlay| {
        if let Err(error) = overlay.hide() {
            eprintln!("isoboard: could not hide capture overlay: {error}");
        }
    });
}

pub(crate) fn update<R: Runtime>(app: &AppHandle<R>, status: &CaptureStatus) {
    let Some(overlay) = app.get_webview_window(WINDOW_LABEL) else {
        eprintln!("isoboard: capture overlay window is unavailable");
        return;
    };
    let state = app.state::<CaptureOverlay>();

    let enabled = app
        .state::<SharedConfig>()
        .lock()
        .expect("config lock poisoned")
        .overlay_enabled();
    if !enabled
        || matches!(
            status,
            CaptureStatus::Idle { .. }
                | CaptureStatus::Started { .. }
                | CaptureStatus::Cancelled { .. }
        )
    {
        hide(app);
        return;
    }

    let sequence = match status {
        CaptureStatus::ImageReady { sequence, .. }
        | CaptureStatus::LoadingModel { sequence, .. }
        | CaptureStatus::DownloadingModel { sequence, .. }
        | CaptureStatus::Processing { sequence, .. }
        | CaptureStatus::Completed { sequence, .. }
        | CaptureStatus::Failed { sequence, .. } => *sequence,
        _ => return,
    };
    if state.opened_sequence.load(Ordering::Acquire) == sequence {
        return;
    }
    if matches!(status, CaptureStatus::ImageReady { .. }) {
        *state
            .history_entry
            .lock()
            .expect("overlay History target lock poisoned") =
            crate::history::pending_identity(app, sequence).map(|pending| (sequence, pending.id));
    }
    let duration = match status {
        CaptureStatus::Completed { .. } => Some(Duration::from_millis(
            APPLICATION.feedback.capture_success_milliseconds,
        )),
        CaptureStatus::Failed { .. } => Some(Duration::from_millis(
            APPLICATION.feedback.capture_failure_milliseconds,
        )),
        _ => None,
    };
    let generation = {
        let mut clock = state
            .dismissal
            .lock()
            .expect("overlay dismissal lock poisoned");
        let now = Instant::now();
        clock.start(sequence, duration, now);
        state.next_generation()
    };
    if state.positioned_sequence.swap(sequence, Ordering::AcqRel) != sequence {
        if let Err(error) = overlay.set_size(LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT)) {
            eprintln!("isoboard: could not size capture overlay: {error}");
        }
        let position = app
            .state::<SharedConfig>()
            .lock()
            .expect("config lock poisoned")
            .snapshot()
            .overlay_position;
        reposition(app, position);
    }
    with_current_overlay(app, generation, move |overlay| {
        set_leaving(&overlay, sequence, false);
        if let Err(error) = overlay.show() {
            eprintln!("isoboard: could not show capture overlay: {error}");
        }
    });

    if duration.is_some() {
        monitor_dismissal(app, sequence, generation);
    }
}

pub(crate) fn reposition<R: Runtime>(app: &AppHandle<R>, position: OverlayPosition) {
    let Some(overlay) = app.get_webview_window(WINDOW_LABEL) else {
        return;
    };
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|cursor| app.monitor_from_point(cursor.x, cursor.y).ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else {
        eprintln!("isoboard: could not find a display for capture overlay");
        return;
    };
    let Ok(size) = overlay.inner_size() else {
        eprintln!("isoboard: could not read capture overlay size");
        return;
    };
    let area = monitor.work_area();
    let coordinates = coordinates(
        position,
        area.position.x,
        area.position.y,
        area.size.width,
        area.size.height,
        size.width,
        size.height,
        monitor.scale_factor(),
    );
    if let Err(error) = overlay.set_position(coordinates) {
        eprintln!("isoboard: could not position capture overlay: {error}");
    }
}

#[allow(clippy::too_many_arguments)]
fn coordinates(
    preset: OverlayPosition,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    overlay_width: u32,
    overlay_height: u32,
    scale: f64,
) -> PhysicalPosition<i32> {
    let margin = (24.0 * scale).round() as i64;
    let x = i64::from(x);
    let y = i64::from(y);
    let width = i64::from(width);
    let height = i64::from(height);
    let overlay_width = i64::from(overlay_width);
    let overlay_height = i64::from(overlay_height);
    let horizontal = match preset {
        OverlayPosition::TopLeft | OverlayPosition::MiddleLeft | OverlayPosition::BottomLeft => {
            x + margin
        }
        OverlayPosition::TopRight | OverlayPosition::MiddleRight | OverlayPosition::BottomRight => {
            x + width - overlay_width - margin
        }
        _ => x + (width - overlay_width) / 2,
    };
    let vertical = match preset {
        OverlayPosition::TopLeft | OverlayPosition::TopCenter | OverlayPosition::TopRight => {
            y + margin
        }
        OverlayPosition::BottomLeft
        | OverlayPosition::BottomCenter
        | OverlayPosition::BottomRight => y + height - overlay_height - margin,
        _ => y + (height - overlay_height) / 2,
    };
    PhysicalPosition::new(
        horizontal.clamp(x, (x + width - overlay_width).max(x)) as i32,
        vertical.clamp(y, (y + height - overlay_height).max(y)) as i32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_inside_scaled_secondary_display_work_area() {
        assert_eq!(
            coordinates(
                OverlayPosition::TopLeft,
                -2560,
                30,
                2560,
                1400,
                720,
                224,
                2.0,
            ),
            PhysicalPosition::new(-2512, 78)
        );
        assert_eq!(
            coordinates(
                OverlayPosition::BottomCenter,
                -2560,
                30,
                2560,
                1400,
                720,
                224,
                2.0,
            ),
            PhysicalPosition::new(-1640, 1158)
        );
    }

    #[test]
    fn keeps_the_window_in_a_narrow_work_area() {
        assert_eq!(
            coordinates(OverlayPosition::TopRight, 100, 50, 300, 140, 360, 112, 1.0),
            PhysicalPosition::new(100, 74)
        );
    }
}
