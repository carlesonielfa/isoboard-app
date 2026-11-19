use std::collections::HashSet;
use std::io::Cursor;
use std::sync::mpsc::{self, Receiver, Sender};

use rodio::{Decoder, OutputStreamBuilder, Source};
use tauri::{AppHandle, Manager, Runtime};

use crate::capture::{CaptureFailureCode, CaptureSource, CaptureStatus};

#[derive(Clone)]
pub(crate) struct AppSounds {
    sender: Sender<CueRequest>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum SoundCue {
    Start,
    Capture,
    Success,
    InferenceError,
}

#[derive(Clone, Copy)]
struct CueRequest {
    sequence: u64,
    cue: SoundCue,
}

struct SoundAsset {
    bytes: &'static [u8],
    volume: f32,
}

impl AppSounds {
    pub(crate) fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        if let Err(error) = std::thread::Builder::new()
            .name("isoboard-audio".into())
            .spawn(move || run_audio_worker(receiver))
        {
            eprintln!("isoboard: could not start audio worker: {error}");
        }
        Self { sender }
    }

    fn queue(&self, sequence: u64, cue: SoundCue) {
        if let Err(error) = self.sender.send(CueRequest { sequence, cue }) {
            eprintln!("isoboard: could not queue sound cue: {error}");
        }
    }
}

pub(crate) fn update<R: Runtime>(app: &AppHandle<R>, status: &CaptureStatus) {
    let enabled = app
        .try_state::<crate::config::SharedConfig>()
        .is_some_and(|config| {
            config
                .lock()
                .expect("config lock poisoned")
                .sounds_enabled()
        });
    let Some((sequence, cue)) = sound_cue_for_status(status, enabled) else {
        return;
    };
    let Some(sounds) = app.try_state::<AppSounds>() else {
        return;
    };
    sounds.queue(sequence, cue);
}

fn sound_cue_for_status(status: &CaptureStatus, enabled: bool) -> Option<(u64, SoundCue)> {
    if !enabled {
        return None;
    }
    match status {
        CaptureStatus::Started {
            sequence,
            source: CaptureSource::Region,
            ..
        } => Some((*sequence, SoundCue::Start)),
        CaptureStatus::ImageReady { sequence, .. } => Some((*sequence, SoundCue::Capture)),
        CaptureStatus::Completed { sequence, .. } => Some((*sequence, SoundCue::Success)),
        CaptureStatus::Failed {
            sequence,
            code: CaptureFailureCode::Inference,
            ..
        } => Some((*sequence, SoundCue::InferenceError)),
        CaptureStatus::Idle { .. }
        | CaptureStatus::Started { .. }
        | CaptureStatus::LoadingModel { .. }
        | CaptureStatus::DownloadingModel { .. }
        | CaptureStatus::Processing { .. }
        | CaptureStatus::Cancelled { .. }
        | CaptureStatus::Failed { .. } => None,
    }
}

fn run_audio_worker(receiver: Receiver<CueRequest>) {
    let mut stream = match OutputStreamBuilder::open_default_stream() {
        Ok(stream) => stream,
        Err(error) => {
            eprintln!("isoboard: could not initialize audio output: {error}");
            return;
        }
    };
    stream.log_on_drop(false);
    let mut deduper = CueDeduper::default();

    for request in receiver {
        if !deduper.should_play(request) {
            continue;
        }
        let asset = asset_for(request.cue);
        let source = match Decoder::try_from(Cursor::new(asset.bytes)) {
            Ok(source) => source.amplify(asset.volume),
            Err(error) => {
                eprintln!("isoboard: could not decode sound cue: {error}");
                continue;
            }
        };
        stream.mixer().add(source);
    }
}

fn asset_for(cue: SoundCue) -> SoundAsset {
    match cue {
        SoundCue::Start => SoundAsset {
            bytes: include_bytes!("../assets/audio/capture-start.wav"),
            volume: 0.5,
        },
        SoundCue::Capture => SoundAsset {
            bytes: include_bytes!("../assets/audio/capture-taken.wav"),
            volume: 0.5,
        },
        SoundCue::Success => SoundAsset {
            bytes: include_bytes!("../assets/audio/capture-copied.wav"),
            volume: 0.5,
        },
        SoundCue::InferenceError => SoundAsset {
            bytes: include_bytes!("../assets/audio/capture-failed.wav"),
            volume: 0.5,
        },
    }
}

#[derive(Default)]
struct CueDeduper {
    sequence: Option<u64>,
    played: HashSet<SoundCue>,
}

impl CueDeduper {
    fn should_play(&mut self, request: CueRequest) -> bool {
        match self.sequence {
            Some(sequence) if request.sequence < sequence => false,
            Some(sequence) if request.sequence == sequence => self.played.insert(request.cue),
            _ => {
                self.sequence = Some(request.sequence);
                self.played.clear();
                self.played.insert(request.cue)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::{CaptureSource, ImagePayload};

    fn status_for(code: CaptureFailureCode) -> CaptureStatus {
        CaptureStatus::Failed {
            sequence: 7,
            source: CaptureSource::Region,
            profile_id: "profile".into(),
            binding_id: None,
            code,
            message: "failure".into(),
        }
    }

    #[test]
    fn maps_only_requested_capture_lifecycle_sounds() {
        let image_ready = CaptureStatus::ImageReady {
            sequence: 7,
            source: CaptureSource::Region,
            profile_id: "profile".into(),
            binding_id: None,
            image: ImagePayload {
                mime_type: "image/png".into(),
                data_base64: String::new(),
                width: 1,
                height: 1,
            },
        };
        let completed = CaptureStatus::Completed {
            sequence: 7,
            source: CaptureSource::Region,
            profile_id: "profile".into(),
            binding_id: None,
            html: String::new(),
        };
        let cancelled = CaptureStatus::Cancelled {
            sequence: 7,
            source: CaptureSource::Region,
            profile_id: "profile".into(),
            binding_id: None,
        };
        let started = CaptureStatus::Started {
            sequence: 7,
            source: CaptureSource::Region,
            profile_id: "profile".into(),
            binding_id: None,
        };
        let processing = CaptureStatus::Processing {
            revision: 0,
            sequence: 7,
            source: CaptureSource::Region,
            profile_id: "profile".into(),
            binding_id: None,
        };

        assert_eq!(
            sound_cue_for_status(&image_ready, true),
            Some((7, SoundCue::Capture))
        );
        assert_eq!(
            sound_cue_for_status(&completed, true),
            Some((7, SoundCue::Success))
        );
        assert_eq!(
            sound_cue_for_status(&status_for(CaptureFailureCode::Inference), true),
            Some((7, SoundCue::InferenceError))
        );
        assert_eq!(
            sound_cue_for_status(&CaptureStatus::Idle { sequence: 7 }, true),
            None
        );
        assert_eq!(
            sound_cue_for_status(&started, true),
            Some((7, SoundCue::Start))
        );
        let file_started = CaptureStatus::Started {
            sequence: 7,
            source: CaptureSource::File,
            profile_id: "profile".into(),
            binding_id: None,
        };
        assert_eq!(sound_cue_for_status(&file_started, true), None);
        assert_eq!(sound_cue_for_status(&processing, true), None);
        assert_eq!(sound_cue_for_status(&cancelled, true), None);
        for status in [
            &started,
            &image_ready,
            &completed,
            &status_for(CaptureFailureCode::Inference),
        ] {
            assert_eq!(sound_cue_for_status(status, false), None);
        }
        for code in [
            CaptureFailureCode::Busy,
            CaptureFailureCode::PermissionDenied,
            CaptureFailureCode::Unsupported,
            CaptureFailureCode::InvalidImage,
            CaptureFailureCode::TooLarge,
            CaptureFailureCode::Io,
            CaptureFailureCode::Clipboard,
        ] {
            assert_eq!(sound_cue_for_status(&status_for(code), true), None);
        }
    }

    #[test]
    fn deduplicates_cues_within_a_capture_and_rejects_stale_sequences() {
        let mut deduper = CueDeduper::default();
        let capture = CueRequest {
            sequence: 4,
            cue: SoundCue::Capture,
        };
        let success = CueRequest {
            sequence: 4,
            cue: SoundCue::Success,
        };

        assert!(deduper.should_play(capture));
        assert!(!deduper.should_play(capture));
        assert!(deduper.should_play(success));
        assert!(!deduper.should_play(CueRequest {
            sequence: 3,
            cue: SoundCue::InferenceError,
        }));
        assert!(deduper.should_play(CueRequest {
            sequence: 5,
            cue: SoundCue::Capture,
        }));
    }

    #[test]
    fn bundled_sound_assets_decode_without_an_audio_device() {
        for cue in [
            SoundCue::Start,
            SoundCue::Capture,
            SoundCue::Success,
            SoundCue::InferenceError,
        ] {
            let asset = asset_for(cue);
            let mut decoder = Decoder::try_from(Cursor::new(asset.bytes))
                .expect("bundled sound asset should decode");
            assert!(
                decoder.next().is_some(),
                "bundled {cue:?} sound should contain samples"
            );
        }
    }
}
