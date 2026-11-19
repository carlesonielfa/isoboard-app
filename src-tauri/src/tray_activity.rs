use crate::application_config::APPLICATION;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::image::Image;
use tauri::tray::TrayIcon;
use tauri::{Manager, Runtime};

use crate::capture::CaptureStatus;

const FRAME_BYTES: [&[u8]; 4] = [
    include_bytes!("../icons/tray-animation/frame-1.png"),
    include_bytes!("../icons/tray-animation/frame-2.png"),
    include_bytes!("../icons/tray-animation/frame-3.png"),
    include_bytes!("../icons/tray-animation/frame-4.png"),
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ActivityState {
    generation: u64,
    sequence: Option<u64>,
    frame: usize,
}

impl ActivityState {
    fn start(&mut self, sequence: u64, frame_count: usize) -> Option<(u64, usize)> {
        if self.sequence == Some(sequence) {
            return None;
        }

        self.generation = self.generation.wrapping_add(1);
        self.sequence = Some(sequence);
        self.frame = 0;
        Some((self.generation, self.frame % frame_count.max(1)))
    }

    fn stop(&mut self, sequence: u64) -> bool {
        if self.sequence != Some(sequence) {
            return false;
        }

        self.generation = self.generation.wrapping_add(1);
        self.sequence = None;
        self.frame = 0;
        true
    }

    fn next_frame(&mut self, generation: u64, frame_count: usize) -> Option<usize> {
        if self.sequence.is_none() || self.generation != generation || frame_count == 0 {
            return None;
        }

        self.frame = (self.frame + 1) % frame_count;
        Some(self.frame)
    }

    fn current_frame_duration(&self) -> Duration {
        let frames = &APPLICATION.feedback.tray_frame_milliseconds;
        Duration::from_millis(frames[self.frame % frames.len()])
    }
}

trait TrayActivityPresenter: Send + Sync {
    fn frame_count(&self) -> usize;
    fn show_idle(&self);
    fn show_processing_frame(&self, frame: usize);
}

struct TauriTrayActivityPresenter<R: Runtime> {
    tray: TrayIcon<R>,
    idle: Option<Image<'static>>,
    processing_frames: Vec<Image<'static>>,
}

impl<R: Runtime> TrayActivityPresenter for TauriTrayActivityPresenter<R> {
    fn frame_count(&self) -> usize {
        if cfg!(target_os = "macos") {
            self.processing_frames.len()
        } else {
            // Windows and Linux tray implementations do not have a portable
            // animated icon contract, so they retain a visible busy frame.
            usize::from(!self.processing_frames.is_empty())
        }
    }

    fn show_idle(&self) {
        let Some(icon) = self.idle.clone() else {
            return;
        };

        if let Err(error) = self
            .tray
            .set_icon_with_as_template(Some(icon), cfg!(target_os = "macos"))
        {
            eprintln!("isoboard: could not restore tray icon: {error}");
        }
    }

    fn show_processing_frame(&self, frame: usize) {
        let Some(icon) = self.processing_frames.get(frame).cloned() else {
            return;
        };

        if let Err(error) = self
            .tray
            .set_icon_with_as_template(Some(icon), cfg!(target_os = "macos"))
        {
            eprintln!("isoboard: could not update processing tray icon: {error}");
        }
    }
}

struct TrayActivityInner {
    state: Mutex<ActivityState>,
    render_lock: Mutex<()>,
    animation_task: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
    presenter: Arc<dyn TrayActivityPresenter>,
}

#[derive(Clone)]
pub(crate) struct TrayActivityController {
    inner: Arc<TrayActivityInner>,
}

impl TrayActivityController {
    pub(crate) fn new<R: Runtime>(
        tray: TrayIcon<R>,
        idle: Option<Image<'static>>,
    ) -> Result<Self, String> {
        let processing_frames = FRAME_BYTES
            .iter()
            .map(|bytes| -> Result<Image<'static>, String> {
                let decoded = image::load_from_memory(bytes)
                    .map_err(|error| format!("could not load processing tray icon: {error}"))?
                    .into_rgba8();
                let width = decoded.width();
                let height = decoded.height();
                let rgba = decoded.into_raw();
                let image: Image<'static> = Image::new_owned(rgba, width, height);
                Ok(image)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let presenter = TauriTrayActivityPresenter {
            tray,
            idle,
            processing_frames,
        };
        Ok(Self {
            inner: Arc::new(TrayActivityInner {
                state: Mutex::new(ActivityState::default()),
                render_lock: Mutex::new(()),
                animation_task: Mutex::new(None),
                presenter: Arc::new(presenter),
            }),
        })
    }

    pub(crate) fn update<R: Runtime>(&self, _app: &tauri::AppHandle<R>, status: &CaptureStatus) {
        match status {
            CaptureStatus::LoadingModel { sequence, .. }
            | CaptureStatus::DownloadingModel { sequence, .. }
            | CaptureStatus::Processing { sequence, .. } => self.start_processing(*sequence),
            CaptureStatus::Completed { sequence, .. }
            | CaptureStatus::Cancelled { sequence, .. }
            | CaptureStatus::Failed { sequence, .. } => self.stop_processing(*sequence),
            CaptureStatus::Idle { .. }
            | CaptureStatus::Started { .. }
            | CaptureStatus::ImageReady { .. } => {}
        }
    }

    fn start_processing(&self, sequence: u64) {
        let _render_guard = self
            .inner
            .render_lock
            .lock()
            .expect("tray activity render lock poisoned");
        let frame_count = self.inner.presenter.frame_count();
        let Some((generation, frame)) = self
            .inner
            .state
            .lock()
            .expect("tray activity state lock poisoned")
            .start(sequence, frame_count)
        else {
            return;
        };
        self.inner.presenter.show_processing_frame(frame);

        if frame_count <= 1 {
            self.cancel_animation_task();
            return;
        }

        let inner = Arc::clone(&self.inner);
        let task = tauri::async_runtime::spawn(async move {
            loop {
                let frame_duration = inner
                    .state
                    .lock()
                    .expect("tray activity state lock poisoned")
                    .current_frame_duration();
                tokio::time::sleep(frame_duration).await;
                let _render_guard = inner
                    .render_lock
                    .lock()
                    .expect("tray activity render lock poisoned");
                let frame = inner
                    .state
                    .lock()
                    .expect("tray activity state lock poisoned")
                    .next_frame(generation, inner.presenter.frame_count());
                let Some(frame) = frame else {
                    break;
                };
                inner.presenter.show_processing_frame(frame);
            }
        });
        if let Some(previous) = self
            .inner
            .animation_task
            .lock()
            .expect("tray activity task lock poisoned")
            .replace(task)
        {
            previous.abort();
        }
    }

    fn stop_processing(&self, sequence: u64) {
        let _render_guard = self
            .inner
            .render_lock
            .lock()
            .expect("tray activity render lock poisoned");
        let stopped = self
            .inner
            .state
            .lock()
            .expect("tray activity state lock poisoned")
            .stop(sequence);
        if stopped {
            self.inner.presenter.show_idle();
            self.cancel_animation_task();
        }
    }

    fn cancel_animation_task(&self) {
        if let Some(task) = self
            .inner
            .animation_task
            .lock()
            .expect("tray activity task lock poisoned")
            .take()
        {
            task.abort();
        }
    }
}

pub(crate) fn update<R: Runtime>(app: &tauri::AppHandle<R>, status: &CaptureStatus) {
    if let Some(controller) = app.try_state::<TrayActivityController>() {
        controller.update(app, status);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::ActivityState;

    #[test]
    fn duplicate_processing_status_does_not_restart_generation() {
        let mut state = ActivityState::default();

        assert_eq!(state.start(7, 3), Some((1, 0)));
        assert_eq!(state.start(7, 3), None);
        assert_eq!(state.next_frame(1, 3), Some(1));
    }

    #[test]
    fn stopping_invalidates_old_timer_generation_and_restores_idle_state() {
        let mut state = ActivityState::default();

        let (generation, _) = state.start(11, 3).expect("first processing status starts");
        assert!(state.stop(11));
        assert!(!state.stop(11));
        assert_eq!(state.next_frame(generation, 3), None);
    }

    #[test]
    fn terminal_status_for_an_older_sequence_does_not_stop_current_processing() {
        let mut state = ActivityState::default();

        let (generation, _) = state.start(11, 3).expect("first processing status starts");
        assert_eq!(state.start(12, 3), Some((2, 0)));
        assert!(!state.stop(11));
        assert_eq!(state.next_frame(generation, 3), None);
        assert_eq!(state.next_frame(2, 3), Some(1));
    }

    #[test]
    fn frame_duration_holds_full_logo_longer_than_moving_frames() {
        let mut state = ActivityState::default();

        assert_eq!(state.start(21, 4), Some((1, 0)));
        assert_eq!(state.current_frame_duration(), Duration::from_millis(300));
        assert_eq!(state.next_frame(1, 4), Some(1));
        assert_eq!(state.current_frame_duration(), Duration::from_millis(300));
        assert_eq!(state.next_frame(1, 4), Some(2));
        assert_eq!(state.current_frame_duration(), Duration::from_millis(300));
        assert_eq!(state.next_frame(1, 4), Some(3));
        assert_eq!(state.current_frame_duration(), Duration::from_millis(500));
        assert_eq!(state.next_frame(1, 4), Some(0));
        assert_eq!(state.current_frame_duration(), Duration::from_millis(300));
    }
}
