use std::sync::Mutex;

use super::OpenHistoryEvent;

#[derive(Default)]
struct Navigation {
    next_id: u64,
    pending: Option<OpenHistoryEvent>,
}

#[derive(Default)]
pub(crate) struct HistoryNavigationState(Mutex<Navigation>);

impl HistoryNavigationState {
    pub(super) fn queue(&self, entry_id: Option<String>) -> Result<OpenHistoryEvent, String> {
        let mut navigation = self
            .0
            .lock()
            .map_err(|_| "History navigation lock poisoned.")?;
        navigation.next_id += 1;
        let request = OpenHistoryEvent {
            request_id: navigation.next_id,
            entry_id,
        };
        navigation.pending = Some(request.clone());
        Ok(request)
    }

    pub(super) fn pending(&self) -> Result<Option<OpenHistoryEvent>, String> {
        Ok(self
            .0
            .lock()
            .map_err(|_| "History navigation lock poisoned.")?
            .pending
            .clone())
    }

    pub(super) fn acknowledge(&self, request_id: u64) -> Result<(), String> {
        let mut navigation = self
            .0
            .lock()
            .map_err(|_| "History navigation lock poisoned.")?;
        if navigation
            .pending
            .as_ref()
            .is_some_and(|request| request.request_id == request_id)
        {
            navigation.pending = None;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn destination_survives_until_the_main_window_acknowledges_it() {
        let state = HistoryNavigationState::default();
        let request = state.queue(Some("capture-entry".into())).unwrap();
        for _ in 0..2 {
            let pending = state.pending().unwrap().unwrap();
            assert_eq!(pending.entry_id.as_deref(), Some("capture-entry"));
            assert_eq!(pending.request_id, request.request_id);
        }
        state.acknowledge(request.request_id).unwrap();
        assert!(state.pending().unwrap().is_none());
    }

    #[test]
    fn an_old_acknowledgement_cannot_erase_a_newer_destination() {
        let state = HistoryNavigationState::default();
        let old = state.queue(Some("old-entry".into())).unwrap();
        let current = state.queue(Some("new-entry".into())).unwrap();
        assert!(current.request_id > old.request_id);
        state.acknowledge(old.request_id).unwrap();
        assert_eq!(
            state.pending().unwrap().unwrap().entry_id.as_deref(),
            Some("new-entry")
        );
        let list = state.queue(None).unwrap();
        state.acknowledge(current.request_id).unwrap();
        assert_eq!(
            state.pending().unwrap().unwrap().request_id,
            list.request_id
        );
    }
}
