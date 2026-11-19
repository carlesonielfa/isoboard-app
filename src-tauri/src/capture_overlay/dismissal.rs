use std::time::{Duration, Instant};

#[derive(Debug, PartialEq, Eq)]
pub(super) enum DismissalAction {
    Wait,
    Fade,
    Restore,
    Hide,
    Stop,
}

#[derive(Default)]
pub(super) struct Dismissal {
    sequence: Option<u64>,
    phase: Option<Phase>,
}

#[derive(Clone, Copy)]
enum Phase {
    Waiting { deadline: Instant },
    Hovered { remaining: Duration },
    Fading { since: Instant },
}

impl Dismissal {
    pub(super) fn start(&mut self, sequence: u64, duration: Option<Duration>, now: Instant) {
        *self = Self {
            sequence: Some(sequence),
            phase: duration.map(|wait| Phase::Waiting {
                deadline: now + wait,
            }),
        };
    }

    pub(super) fn poll(
        &mut self,
        sequence: u64,
        hovered: bool,
        now: Instant,
        exit_grace: Duration,
    ) -> DismissalAction {
        if self.sequence != Some(sequence) {
            return DismissalAction::Stop;
        }
        let Some(phase) = self.phase else {
            return DismissalAction::Stop;
        };
        if hovered {
            let (remaining, action) = match phase {
                Phase::Waiting { deadline } => (
                    deadline.saturating_duration_since(now),
                    DismissalAction::Wait,
                ),
                Phase::Hovered { .. } => return DismissalAction::Wait,
                Phase::Fading { .. } => (Duration::ZERO, DismissalAction::Restore),
            };
            self.phase = Some(Phase::Hovered { remaining });
            return action;
        }
        let phase = match phase {
            Phase::Hovered { remaining } => Phase::Waiting {
                deadline: now + remaining,
            },
            phase => phase,
        };
        self.phase = Some(phase);
        match phase {
            Phase::Waiting { deadline } if now < deadline => DismissalAction::Wait,
            Phase::Waiting { .. } => {
                self.phase = Some(Phase::Fading { since: now });
                DismissalAction::Fade
            }
            Phase::Fading { since } if now.duration_since(since) >= exit_grace => {
                DismissalAction::Hide
            }
            _ => DismissalAction::Wait,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GRACE: Duration = Duration::from_millis(320);

    #[test]
    fn native_cursor_samples_pause_and_resume_the_remaining_time() {
        let now = Instant::now();
        let mut clock = Dismissal::default();
        clock.start(1, Some(Duration::from_secs(4)), now);
        assert_eq!(
            clock.poll(1, true, now + Duration::from_millis(1500), GRACE),
            DismissalAction::Wait
        );
        assert_eq!(
            clock.poll(1, true, now + Duration::from_secs(30), GRACE),
            DismissalAction::Wait
        );
        assert_eq!(
            clock.poll(1, false, now + Duration::from_secs(30), GRACE),
            DismissalAction::Wait
        );
        assert_eq!(
            clock.poll(1, false, now + Duration::from_millis(32499), GRACE),
            DismissalAction::Wait
        );
        assert_eq!(
            clock.poll(1, false, now + Duration::from_millis(32500), GRACE),
            DismissalAction::Fade
        );
        assert_eq!(
            clock.poll(1, false, now + Duration::from_millis(32820), GRACE),
            DismissalAction::Hide
        );
    }

    #[test]
    fn cursor_already_over_the_panel_holds_the_full_terminal_duration() {
        let now = Instant::now();
        let mut clock = Dismissal::default();
        clock.start(1, Some(Duration::from_secs(12)), now);
        assert_eq!(clock.poll(1, true, now, GRACE), DismissalAction::Wait);
        assert_eq!(
            clock.poll(1, false, now + Duration::from_secs(30), GRACE),
            DismissalAction::Wait
        );
        assert_eq!(
            clock.poll(1, false, now + Duration::from_secs(42), GRACE),
            DismissalAction::Fade
        );
    }

    #[test]
    fn cursor_entering_during_fade_restores_and_cancels_pending_hide() {
        let now = Instant::now();
        let mut clock = Dismissal::default();
        clock.start(1, Some(Duration::from_secs(4)), now);
        assert_eq!(
            clock.poll(1, false, now + Duration::from_secs(4), GRACE),
            DismissalAction::Fade
        );
        assert_eq!(
            clock.poll(1, true, now + Duration::from_millis(4100), GRACE),
            DismissalAction::Restore
        );
        assert_eq!(
            clock.poll(1, true, now + Duration::from_secs(10), GRACE),
            DismissalAction::Wait
        );
        assert_eq!(
            clock.poll(1, false, now + Duration::from_secs(11), GRACE),
            DismissalAction::Fade
        );
        assert_eq!(
            clock.poll(1, false, now + Duration::from_millis(11320), GRACE),
            DismissalAction::Hide
        );
    }

    #[test]
    fn new_capture_discards_old_hold_and_stale_poll_work() {
        let now = Instant::now();
        let mut clock = Dismissal::default();
        clock.start(1, Some(Duration::from_secs(4)), now);
        clock.poll(1, true, now, GRACE);
        clock.start(2, Some(Duration::from_secs(4)), now);
        assert_eq!(clock.poll(1, false, now, GRACE), DismissalAction::Stop);
        assert_eq!(
            clock.poll(2, false, now + Duration::from_secs(4), GRACE),
            DismissalAction::Fade
        );
        clock.start(3, None, now);
        assert_eq!(clock.poll(3, true, now, GRACE), DismissalAction::Stop);
    }
}
