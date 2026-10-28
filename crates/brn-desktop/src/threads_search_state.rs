//! Deterministic request identity and cursor ownership; no tasks or persistence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Lanes {
    Both,
    Notes,
    Chats,
}
impl Lanes {
    pub(super) fn notes(self) -> bool {
        self != Self::Chats
    }
    pub(super) fn chats(self) -> bool {
        self != Self::Notes
    }
}
#[derive(Default)]
struct Lane {
    awaiting: Option<u64>,
    back: Vec<Option<String>>,
    current: Option<String>,
}
#[derive(Default)]
pub(super) struct SearchState {
    query: String,
    sequence: u64,
    notes: Lane,
    chats: Lane,
    fresh: Option<u64>,
}
impl SearchState {
    pub(super) fn query(&self) -> &str {
        &self.query
    }
    pub(super) fn change_query(&mut self, query: String) {
        self.query = query;
        self.notes = Lane::default();
        self.chats = Lane::default();
        self.fresh = None;
    }
    pub(super) fn begin(&mut self, lanes: Lanes, cursor: Option<String>) -> Option<u64> {
        self.sequence = self
            .sequence
            .checked_add(1)
            .expect("search request identity exhausted");
        let request = (!self.query.trim().is_empty()).then_some(self.sequence);
        if lanes.notes() {
            self.notes.awaiting = request;
            self.notes.current = cursor.clone();
        }
        if lanes.chats() {
            self.chats.awaiting = request;
            self.chats.current = cursor;
        }
        if lanes == Lanes::Both {
            self.fresh = request;
        }
        request
    }
    /// Success and failure have identical admission: only matching lanes may change.
    pub(super) fn answered(&mut self, request: u64) -> Option<Lanes> {
        let notes = self.notes.awaiting == Some(request);
        let chats = self.chats.awaiting == Some(request);
        let lanes = match (notes, chats) {
            (true, true) => Lanes::Both,
            (true, false) => Lanes::Notes,
            (false, true) => Lanes::Chats,
            (false, false) => return None,
        };
        if notes {
            self.notes.awaiting = None;
        }
        if chats {
            self.chats.awaiting = None;
        }
        if self.fresh == Some(request) {
            self.fresh = None;
        }
        Some(lanes)
    }
    fn lane(&self, lanes: Lanes) -> &Lane {
        match lanes {
            Lanes::Chats => &self.chats,
            _ => &self.notes,
        }
    }
    fn lane_mut(&mut self, lanes: Lanes) -> &mut Lane {
        match lanes {
            Lanes::Chats => &mut self.chats,
            _ => &mut self.notes,
        }
    }
    pub(super) fn pending(&self, lane: Lanes) -> bool {
        self.lane(lane).awaiting.is_some()
    }
    pub(super) fn fresh(&self) -> bool {
        self.fresh.is_some()
    }
    pub(super) fn has_previous(&self, lane: Lanes) -> bool {
        !self.lane(lane).back.is_empty()
    }
    pub(super) fn next(&mut self, lane: Lanes) {
        let lane = self.lane_mut(lane);
        lane.back.push(lane.current.take());
    }
    pub(super) fn previous(&mut self, lane: Lanes) -> Option<Option<String>> {
        self.lane_mut(lane).back.pop()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn queried(s: &mut SearchState, query: &str) -> u64 {
        s.change_query(query.into());
        s.begin(Lanes::Both, None).unwrap()
    }
    #[test]
    fn newer_query_rejects_old_reply_and_clears_both_cursor_histories() {
        let mut s = SearchState::default();
        let old = queried(&mut s, "alpha");
        s.next(Lanes::Notes);
        s.next(Lanes::Chats);
        let new = queried(&mut s, "beta");
        assert_eq!(s.answered(old), None);
        assert!(s.fresh());
        assert!(!s.has_previous(Lanes::Notes));
        assert!(!s.has_previous(Lanes::Chats));
        assert_eq!(s.answered(new), Some(Lanes::Both));
        assert!(!s.fresh());
        assert_eq!(s.answered(new), None);
    }
    #[test]
    fn clear_rejects_reply_even_before_another_request_is_issued() {
        let mut s = SearchState::default();
        let old = queried(&mut s, "alpha");
        s.change_query("  ".into());
        assert_eq!(s.answered(old), None);
        assert_eq!(s.begin(Lanes::Both, None), None);
        assert!(!s.pending(Lanes::Notes));
        assert!(!s.pending(Lanes::Chats));
        assert!(!s.fresh());
    }
    #[test]
    fn independently_paged_lanes_accept_reversed_replies_and_keep_their_cursors() {
        let mut s = SearchState::default();
        let first = queried(&mut s, "alpha");
        assert_eq!(s.answered(first), Some(Lanes::Both));
        s.next(Lanes::Notes);
        let notes = s.begin(Lanes::Notes, Some("n2".into())).unwrap();
        s.next(Lanes::Chats);
        let chats = s.begin(Lanes::Chats, Some("c2".into())).unwrap();
        assert_eq!(s.answered(chats), Some(Lanes::Chats));
        assert!(s.pending(Lanes::Notes));
        assert_eq!(s.answered(notes), Some(Lanes::Notes));
        assert_eq!(s.previous(Lanes::Notes), Some(None));
        assert!(s.has_previous(Lanes::Chats));
        assert_eq!(s.previous(Lanes::Chats), Some(None));
    }
    #[test]
    fn previous_returns_exact_lane_cursor_and_stops_at_first_page() {
        let mut s = SearchState::default();
        let first = queried(&mut s, "alpha");
        s.answered(first);
        s.next(Lanes::Notes);
        let second = s
            .begin(Lanes::Notes, Some("opaque-note-cursor-2".into()))
            .unwrap();
        s.answered(second);
        s.next(Lanes::Notes);
        let third = s
            .begin(Lanes::Notes, Some("opaque-note-cursor-3".into()))
            .unwrap();
        s.answered(third);
        assert_eq!(s.previous(Lanes::Chats), None);
        assert_eq!(
            s.previous(Lanes::Notes),
            Some(Some("opaque-note-cursor-2".into()))
        );
        assert_eq!(s.previous(Lanes::Notes), Some(None));
        assert_eq!(s.previous(Lanes::Notes), None);
    }
    #[test]
    fn new_query_during_two_page_requests_rejects_both_late_answers() {
        let mut s = SearchState::default();
        let first = queried(&mut s, "alpha");
        s.answered(first);
        let notes = s.begin(Lanes::Notes, Some("n2".into())).unwrap();
        let chats = s.begin(Lanes::Chats, Some("c2".into())).unwrap();
        let new = queried(&mut s, "beta");
        assert_eq!(s.answered(chats), None);
        assert_eq!(s.answered(notes), None);
        assert_eq!(s.answered(new), Some(Lanes::Both));
    }
    #[test]
    fn failure_completion_then_retry_does_not_readmit_old_reply() {
        let mut s = SearchState::default();
        let first = queried(&mut s, "alpha");
        s.answered(first);
        let failed = s.begin(Lanes::Notes, Some("n2".into())).unwrap();
        assert_eq!(s.answered(failed), Some(Lanes::Notes));
        assert!(!s.pending(Lanes::Notes));
        assert!(!s.pending(Lanes::Chats));
        let retry = s.begin(Lanes::Notes, Some("n2".into())).unwrap();
        assert_eq!(s.answered(failed), None);
        assert_eq!(s.answered(retry), Some(Lanes::Notes));
        assert_eq!(s.query(), "alpha");
    }
}
