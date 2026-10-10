//! Read-only presentation of explicitly requested saved bytes, never knowledge authority.
use super::{AiState, Pending};
use brn_workflow::{
    app_worker::AppCommand,
    knowledge::{RawEvidence, RawEvidenceRequest},
};
use uuid::Uuid;

pub struct RawEvidenceState {
    pub path: String,
    pub reply: Option<RawEvidence>,
}
impl AiState {
    pub fn open_raw_evidence(&mut self, path: String) -> (Uuid, AppCommand) {
        self.clear_provenance();
        self.clear_links();
        self.clear_profile_context();
        self.clear_linked_action();
        self.editor = None;
        self.evidence = None;
        self.raw_evidence = Some(RawEvidenceState {
            path: path.clone(),
            reply: None,
        });
        self.raw_query(RawEvidenceRequest {
            path,
            start_byte: 0,
            end_byte: None,
            expected_sha256: None,
        })
    }
    fn raw_query(&mut self, request: RawEvidenceRequest) -> (Uuid, AppCommand) {
        self.note_generation = self.note_generation.wrapping_add(1);
        self.note_error = None;
        self.command(
            Pending::RawEvidence {
                request: request.clone(),
                generation: self.note_generation,
            },
            AppCommand::RawEvidence(request),
        )
    }
    pub fn raw_loading(&self) -> bool {
        self.pending.values().any(|pending| matches!(pending, Pending::RawEvidence { generation, .. } if *generation == self.note_generation))
    }
    pub fn raw_range(&mut self, start: &str, end: &str) -> Option<(Uuid, AppCommand)> {
        if self.raw_loading() || self.application_busy() {
            return None;
        }
        let reply = self.raw_evidence.as_ref()?.reply.as_ref()?;
        let request = start
            .parse::<usize>()
            .ok()
            .zip(end.parse::<usize>().ok())
            .map(|(start_byte, end)| RawEvidenceRequest {
                path: reply.path.clone(),
                start_byte,
                end_byte: Some(end),
                expected_sha256: Some(reply.sha256),
            });
        let Some(request) = request.filter(|request| request.validate().is_ok()) else {
            self.note_error =
                Some("Enter an exact byte range of at most 50,000 bytes. Nothing was read.".into());
            return None;
        };
        Some(self.raw_query(request))
    }
    pub fn next_raw_range(&mut self) -> Option<(Uuid, AppCommand)> {
        if self.raw_loading() || self.application_busy() {
            return None;
        }
        let reply = self.raw_evidence.as_ref()?.reply.as_ref()?;
        if reply.end_byte == reply.total_bytes {
            return None;
        }
        Some(self.raw_query(RawEvidenceRequest {
            path: reply.path.clone(),
            start_byte: reply.end_byte,
            end_byte: None,
            expected_sha256: Some(reply.sha256),
        }))
    }
    pub(super) fn apply_raw_evidence(&mut self, id: Uuid, reply: &RawEvidence) {
        let Some(Pending::RawEvidence {
            request,
            generation,
        }) = self.pending.get(&id)
        else {
            return;
        };
        if reply.validate_for(request).is_err() {
            return;
        }
        let generation = *generation;
        let path = request.path.clone();
        self.pending.remove(&id);
        if generation != self.note_generation {
            return;
        }
        if let Some(state) = &mut self.raw_evidence
            && state.path == path
        {
            state.reply = Some(reply.clone());
            self.note_error = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brn_workflow::app_worker::AppEvent;
    fn reply(request: &RawEvidenceRequest, total: usize) -> RawEvidence {
        let start = request.start_byte;
        let end = request.end_byte.unwrap_or((start + 50_000).min(total));
        RawEvidence {
            path: request.path.clone(),
            sha256: request.expected_sha256.unwrap_or([42; 32]),
            start_byte: start,
            end_byte: end,
            total_bytes: total,
            text: "x".repeat(end - start),
            partial: start != 0 || end != total,
            facts: None,
            metadata_issue: Some("Malformed saved identity".into()),
        }
    }
    #[test]
    fn raw_ranges_are_correlated_hash_bound_and_do_not_promote_damaged_metadata() {
        let mut state = AiState::default();
        let (id, AppCommand::RawEvidence(request)) = state.open_raw_evidence("damaged.md".into())
        else {
            panic!()
        };
        let prefix = reply(&request, 949_968);
        state.apply(
            Uuid::new_v4(),
            AppEvent::RawEvidence(Box::new(prefix.clone())),
        );
        assert!(state.pending.contains_key(&id));
        let mut wrong = prefix.clone();
        wrong.path = "other.md".into();
        state.apply(id, AppEvent::RawEvidence(Box::new(wrong)));
        assert!(state.pending.contains_key(&id));
        state.apply(id, AppEvent::RawEvidence(Box::new(prefix)));
        assert!(state.editor.is_none() && state.evidence.is_none());
        assert!(
            state
                .raw_evidence
                .as_ref()
                .unwrap()
                .reply
                .as_ref()
                .unwrap()
                .facts
                .is_none()
        );
        let (tail_id, AppCommand::RawEvidence(tail)) = state.raw_range("949000", "949968").unwrap()
        else {
            panic!()
        };
        assert_eq!(tail.expected_sha256, Some([42; 32]));
        let mut changed = reply(&tail, 949_968);
        changed.sha256 = [43; 32];
        state.apply(tail_id, AppEvent::RawEvidence(Box::new(changed)));
        assert!(state.pending.contains_key(&tail_id));
        state.apply(
            tail_id,
            AppEvent::RawEvidence(Box::new(reply(&tail, 949_968))),
        );
        assert_eq!(
            state
                .raw_evidence
                .as_ref()
                .unwrap()
                .reply
                .as_ref()
                .unwrap()
                .text
                .len(),
            968
        );
        assert!(state.raw_range("0", "50001").is_none());
        assert!(state.note_error.is_some());
        assert!(state.next_raw_range().is_none());
    }
    #[test]
    fn stale_raw_reply_cannot_replace_a_new_document() {
        let mut state = AiState::default();
        let (id, AppCommand::RawEvidence(request)) = state.open_raw_evidence("damaged.md".into())
        else {
            panic!()
        };
        state.open_evidence(
            "source.md".into(),
            brn_workflow::library::KnowledgeScope::Source,
        );
        state.apply(id, AppEvent::RawEvidence(Box::new(reply(&request, 100))));
        assert!(state.raw_evidence.is_none());
        assert_eq!(state.evidence.as_ref().unwrap().path, "source.md");
        assert!(!state.pending.contains_key(&id));
    }
}
